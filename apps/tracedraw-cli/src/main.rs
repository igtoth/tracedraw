//! tracedraw-cli: headless tools.
//!
//! ```text
//! tracedraw-cli inspect file.cdr          dump the RIFF chunk tree
//! tracedraw-cli info file.cdr             version, pages, objects, warnings
//! tracedraw-cli convert in.cdr out.svg    export the first page (or .tdraw)
//! ```

use std::process::ExitCode;

/// Damage a file deterministically: truncate, flip bytes, cut a span, or
/// overwrite a few 32-bit words.
/// `println!` that ends the program quietly when the reader of its output
/// went away (`tracedraw-cli inspect file | head`) instead of panicking.
macro_rules! outln {
    () => {
        outln!("")
    };
    ($($t:tt)*) => {{
        use std::io::Write as _;
        if let Err(e) = writeln!(std::io::stdout(), $($t)*) {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                std::process::exit(0);
            }
        }
    }};
}

/// `print!` with the same handling as [`outln!`].
macro_rules! out {
    ($($t:tt)*) => {{
        use std::io::Write as _;
        if let Err(e) = write!(std::io::stdout(), $($t)*) {
            if e.kind() == std::io::ErrorKind::BrokenPipe {
                std::process::exit(0);
            }
        }
    }};
}

fn mutate(data: &[u8], seed: u64) -> Vec<u8> {
    let mut v = data.to_vec();
    let mut s = seed | 1;
    let mut rnd = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    let len = v.len().max(1);
    match rnd() % 4 {
        0 => {
            let n = (rnd() as usize) % len;
            v.truncate(n);
        }
        1 => {
            for _ in 0..(1 + rnd() % 64) {
                let i = (rnd() as usize) % len;
                if i < v.len() {
                    v[i] = rnd() as u8;
                }
            }
        }
        2 => {
            let i = (rnd() as usize) % len;
            let j = (rnd() as usize) % len;
            let (a, b) = (i.min(j), i.max(j).min(v.len()));
            v.drain(a..b);
        }
        _ => {
            for _ in 0..(1 + rnd() % 8) {
                let i = (rnd() as usize) % len;
                if i + 4 <= v.len() {
                    v[i..i + 4].copy_from_slice(&(rnd() as u32).to_le_bytes());
                }
            }
        }
    }
    v
}

fn usage() -> ExitCode {
    eprintln!("usage:\n  tracedraw-cli inspect <file.cdr> [chunk id]\n  tracedraw-cli info <file.cdr|file.cmx|file.tdraw>\n  tracedraw-cli icc <profile.icc>\n  tracedraw-cli stress <file> [iterations]   mutation test of the file's reader\n  tracedraw-cli convert <in.cdr|in.cmx|in.tdraw|in.svg|in.pdf|in.ai|in.eps|in.ps|in.prn|in.dxf|in.psd|in.emf|in.wmf|in.plt|in.txt|in.rtf|in.docx> <out.svg|out.pdf|out.eps|out.ps|out.prn|out.dxf|out.emf|out.wmf|out.plt|out.psd|out.html|out.png|out.tdraw|out.cdr>");
    ExitCode::from(2)
}

/// How text goes into a PDF: `curves`, `named` (fonts left out of the
/// file) or, by default, text with embedded fonts.
fn pdf_text(choice: Option<&str>) -> tracedraw_io::pdf::PdfText {
    let mut t = tracedraw_io::pdf::PdfText::default();
    match choice.map(str::to_ascii_lowercase).as_deref() {
        Some("curves") => t.as_curves = true,
        Some("named") => t.embed_fonts = false,
        _ => {}
    }
    t
}

/// Protection for a written PDF: a password to open it, a password for
/// the permissions and the permissions to deny (a comma-separated list of
/// print, hq-print, copy, modify, annotate, forms, assemble and
/// accessibility). Nothing set: no protection.
fn pdf_security(
    open: Option<String>,
    owner: Option<String>,
    deny: Option<String>,
) -> Option<tracedraw_io::pdf_crypt::Security> {
    let open = open.unwrap_or_default();
    let owner = owner.unwrap_or_default();
    if open.is_empty() && owner.is_empty() {
        return None;
    }
    let mut p = tracedraw_io::pdf_crypt::Permissions::default();
    if !owner.is_empty() {
        for item in deny.unwrap_or_default().split(',') {
            match item.trim().to_ascii_lowercase().as_str() {
                "print" => p.print = false,
                "hq-print" => p.print_high_quality = false,
                "copy" => p.copy = false,
                "modify" => p.modify = false,
                "annotate" => p.annotate = false,
                "forms" => p.fill_forms = false,
                "assemble" => p.assemble = false,
                "accessibility" => p.accessibility = false,
                "" => {}
                other => eprintln!("warning: unknown permission {other}"),
            }
        }
    }
    let owner = if owner.is_empty() {
        open.clone()
    } else {
        owner
    };
    Some(tracedraw_io::pdf_crypt::Security {
        user_password: open,
        owner_password: owner,
        permissions: p,
    })
}

/// What the .cdr reader skipped or guessed, one line each, as the other
/// readers report it while they read.
fn reader_warnings(report: &tracedraw_cdr::ParseReport) -> Vec<String> {
    let mut lines: Vec<String> = report
        .warnings
        .iter()
        .map(|w| format!("warning: {w}"))
        .collect();
    if report.skipped_objects > 0 {
        lines.push(format!(
            "warning: {} object(s) skipped",
            report.skipped_objects
        ));
    }
    lines
}

/// Read a document of any kind the CLI knows; the fonts it carries are
/// loaded for its text.
fn load(
    path: &str,
) -> Result<(tracedraw_core::Document, Option<tracedraw_cdr::ParseReport>), String> {
    let mut loaded = read_document(path)?;
    if !loaded.0.fonts.is_empty() {
        tracedraw_text::fonts().add_document_fonts(&loaded.0.fonts);
    }
    // Paragraph text flows around the objects marked to wrap it.
    loaded.0.refresh_text_flow();
    // CMYK colours render through the profile the drawing carries.
    if let Some(p) = &loaded.0.cmyk_profile {
        match tracedraw_core::icc::Profile::parse(&p.data) {
            Ok(profile) if profile.color_space() == tracedraw_core::icc::ColorSpace::Cmyk => {
                tracedraw_core::color::engine::install(tracedraw_core::color::IccEngine::new(
                    None,
                    Some(profile),
                    tracedraw_core::icc::Intent::default(),
                    false,
                ));
            }
            _ => eprintln!("{path}: the colour profile it carries could not be used"),
        }
    }
    Ok(loaded)
}

fn read_document(
    path: &str,
) -> Result<(tracedraw_core::Document, Option<tracedraw_cdr::ParseReport>), String> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".cdr")
        || lower.ends_with(".cdt")
        || lower.ends_with(".cdrt")
        || lower.ends_with(".cmx")
    {
        tracedraw_cdr::open(path)
            .map(|(d, r)| (d, Some(r)))
            .map_err(|e| e.to_string())
    } else if lower.ends_with(".pdf")
        || (lower.ends_with(".ai") && !std::fs::read(path).is_ok_and(|b| b.starts_with(b"%!")))
    {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        // TRACEDRAW_PASSWORD opens a protected file.
        let password = std::env::var("TRACEDRAW_PASSWORD").unwrap_or_default();
        let imported = tracedraw_io::pdf_import::parse_with_password(
            &bytes,
            &mut tracedraw_core::id::IdSource::default(),
            &password,
        )
        .map_err(|e| e.to_string())?;
        for w in &imported.warnings {
            eprintln!("warning: {w}");
        }
        let title = std::path::Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok((
            tracedraw_io::pdf_import::to_document(imported, &title),
            None,
        ))
    } else if [".eps", ".ps", ".prn", ".ai"]
        .iter()
        .any(|e| lower.ends_with(e))
    {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let imported =
            tracedraw_io::eps_import::parse(&bytes, &mut tracedraw_core::id::IdSource::default())?;
        for w in &imported.warnings {
            eprintln!("warning: {w}");
        }
        // Every page of the file, each at its own size.
        let pages = imported.pages();
        let first = pages
            .first()
            .map(|p| p.1)
            .unwrap_or(tracedraw_core::document::paper::A4);
        let mut doc = tracedraw_core::Document::new("eps", first);
        let mut ids = doc.ids().clone();
        for (i, (shapes, size)) in pages.into_iter().enumerate() {
            if i == 0 {
                doc.pages[0].layers[0].shapes = shapes;
                continue;
            }
            let mut layer = tracedraw_core::Layer::new(ids.layer(), "Layer 1");
            layer.shapes = shapes;
            doc.pages.push(tracedraw_core::Page {
                id: ids.page(),
                name: format!("Page {}", i + 1),
                size,
                layers: vec![layer],
                guides: Vec::new(),
                background: None,
            });
        }
        doc.set_ids(ids);
        Ok((doc, None))
    } else if lower.ends_with(".psd") || lower.ends_with(".psb") {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let imported =
            tracedraw_io::psd::parse(&bytes, &mut tracedraw_core::id::IdSource::default())?;
        for w in &imported.warnings {
            eprintln!("warning: {w}");
        }
        let mut doc = tracedraw_core::Document::new("psd", imported.size);
        doc.pages[0].layers[0].shapes = imported.shapes;
        Ok((doc, None))
    } else if lower.ends_with(".emf") || lower.ends_with(".wmf") {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let imported =
            tracedraw_io::emf::parse(&bytes, &mut tracedraw_core::id::IdSource::default())?;
        for w in &imported.warnings {
            eprintln!("warning: {w}");
        }
        let title = std::path::Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok((tracedraw_io::emf::to_document(imported, &title), None))
    } else if lower.ends_with(".plt") || lower.ends_with(".hpgl") || lower.ends_with(".hgl") {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let imported =
            tracedraw_io::plt::parse(&bytes, &mut tracedraw_core::id::IdSource::default())?;
        for w in &imported.warnings {
            eprintln!("warning: {w}");
        }
        let title = std::path::Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok((tracedraw_io::plt::to_document(imported, &title), None))
    } else if lower.ends_with(".txt") || lower.ends_with(".rtf") || lower.ends_with(".docx") {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let imported = if lower.ends_with(".rtf") {
            tracedraw_io::text_import::parse_rtf(&bytes)?
        } else if lower.ends_with(".docx") {
            tracedraw_io::text_import::parse_docx(&bytes)?
        } else {
            tracedraw_io::text_import::parse_txt(&bytes)
        };
        for w in &imported.warnings {
            eprintln!("warning: {w}");
        }
        let mut doc = tracedraw_core::Document::new("text", tracedraw_core::document::paper::A4);
        let mut ids = doc.ids().clone();
        let mut shape = imported.to_shape(tracedraw_io::text_import::Placement::a4(), &mut ids);
        shape.fill = tracedraw_core::Fill::Solid(tracedraw_core::Color::BLACK);
        shape.stroke = None;
        doc.pages[0].layers[0].shapes.push(shape);
        doc.set_ids(ids);
        Ok((doc, None))
    } else if lower.ends_with(".dxf") {
        let text = std::fs::read(path)
            .map_err(|e| e.to_string())
            .map(|b| String::from_utf8_lossy(&b).into_owned())?;
        let imported =
            tracedraw_io::dxf::parse(&text, &mut tracedraw_core::id::IdSource::default())?;
        for w in &imported.warnings {
            eprintln!("warning: {w}");
        }
        let title = std::path::Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok((tracedraw_io::dxf::to_document(imported, &title), None))
    } else if lower.ends_with(".svg") || lower.ends_with(".svgz") {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let text = if bytes.len() > 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
            use std::io::Read;
            let mut out = String::new();
            flate2::read::GzDecoder::new(&bytes[..])
                .read_to_string(&mut out)
                .map_err(|e| e.to_string())?;
            out
        } else {
            String::from_utf8(bytes).map_err(|e| e.to_string())?
        };
        let imported =
            tracedraw_io::svg_import::parse(&text, &mut tracedraw_core::id::IdSource::default())?;
        let mut doc = tracedraw_core::Document::new("svg", imported.size);
        doc.pages[0].layers[0].shapes = imported.shapes;
        Ok((doc, None))
    } else {
        tracedraw_io::load_native(path)
            .map(|d| (d, None))
            .map_err(|e| e.to_string())
    }
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    tracedraw_text::install();
    tracedraw_io::emf::install();
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("inspect") => {
            let Some(path) = args.get(2) else {
                return usage();
            };
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let container = match tracedraw_cdr::container::detect(&bytes) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            outln!(
                "container: {:?}, {}",
                container.kind,
                container.version.name()
            );
            let riff = match tracedraw_cdr::container::riff_stream(&bytes, &container) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let externals = tracedraw_cdr::container::external_streams(&bytes, &container);
            match tracedraw_cdr::riff::parse_with_externals(&riff, externals, container.version.0) {
                Ok(tree) => match args.get(3) {
                    // `inspect file.cdr arrw`: the data of every chunk with
                    // that id, in hex.
                    Some(id) => {
                        let mut name = [b' '; 4];
                        for (k, b) in id.bytes().take(4).enumerate() {
                            name[k] = b;
                        }
                        tree.root.walk(&mut |c, _| {
                            if c.is(&name) {
                                let d = tree.data(&riff, c);
                                outln!("{} {} bytes", c.id_str(), d.len());
                                for (i, row) in d.chunks(16).enumerate() {
                                    let hex: Vec<String> =
                                        row.iter().map(|b| format!("{b:02x}")).collect();
                                    outln!("{:6} {}", i * 16, hex.join(" "));
                                }
                            }
                        });
                    }
                    None => out!("{}", tree.dump()),
                },
                Err(e) => {
                    eprintln!("{path}: {e}");
                    return ExitCode::FAILURE;
                }
            }
            ExitCode::SUCCESS
        }
        Some("info") => {
            let Some(path) = args.get(2) else {
                return usage();
            };
            match load(path) {
                Ok((doc, report)) => {
                    outln!("title: {}", doc.title);
                    for p in &doc.pages {
                        let n: usize = p.layers.iter().map(|l| l.shapes.len()).sum();
                        outln!(
                            "{}: {:.1} x {:.1} mm, {} layer(s), {} object(s)",
                            p.name,
                            p.size.width,
                            p.size.height,
                            p.layers.len(),
                            n
                        );
                    }
                    for f in &doc.fonts {
                        outln!("carries font: {} ({} bytes)", f.family, f.data.len());
                    }
                    if let Some(p) = &doc.cmyk_profile {
                        outln!("carries CMYK profile: {} ({} bytes)", p.name, p.data.len());
                    }
                    // TRACEDRAW_OBJECTS=1 lists every object, for checking a
                    // reader against what the file holds.
                    if std::env::var("TRACEDRAW_OBJECTS").is_ok_and(|v| v != "0") {
                        for p in &doc.pages {
                            outln!("{}:", p.name);
                            for l in &p.layers {
                                outln!("  layer {:?}", l.name);
                                for s in &l.shapes {
                                    out!("{}", describe_shape(s, 2));
                                }
                            }
                        }
                    }
                    if let Some(r) = report {
                        outln!(
                            "version: {}",
                            r.version.map(|v| v.name()).unwrap_or_else(|| {
                                if path.to_ascii_lowercase().ends_with(".cmx") {
                                    "CMX".into()
                                } else {
                                    String::new()
                                }
                            })
                        );
                        outln!("skipped objects: {}", r.skipped_objects);
                        for w in &r.warnings {
                            outln!("warning: {w}");
                        }
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{path}: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("icc") => {
            // Describe an ICC profile and show a few conversions through it.
            let Some(path) = args.get(2) else {
                usage();
                return ExitCode::FAILURE;
            };
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let profile = match tracedraw_core::icc::Profile::parse(&bytes) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            outln!(
                "{}: {:?}, {:?} -> {:?}, version {:?}, {} channel(s), device->PCS {}, PCS->device {}",
                profile.description().unwrap_or("(no description)"),
                profile.class(),
                profile.color_space(),
                profile.pcs(),
                profile.version(),
                profile.channels(),
                profile.has_device_to_pcs(),
                profile.has_pcs_to_device()
            );
            let srgb = tracedraw_core::icc::Profile::srgb();
            let intent = tracedraw_core::icc::Intent::default();
            if let Some(t) = tracedraw_core::icc::Transform::new(&profile, &srgb, intent, true) {
                let n = profile.channels();
                let samples: Vec<Vec<f32>> = if n == 4 {
                    vec![
                        vec![0.0, 0.0, 0.0, 0.0],
                        vec![1.0, 0.0, 0.0, 0.0],
                        vec![0.0, 1.0, 0.0, 0.0],
                        vec![0.0, 0.0, 1.0, 0.0],
                        vec![0.0, 0.0, 0.0, 1.0],
                    ]
                } else {
                    vec![vec![1.0; n], vec![0.0; n]]
                };
                for s in samples {
                    let rgb = t.transform(&s);
                    outln!("  {s:?} -> sRGB {rgb:?}");
                }
            }
            ExitCode::SUCCESS
        }
        Some("stress") => {
            // Mutation test: feed damaged copies of a file to its reader and
            // report panics (there must be none).
            let Some(input) = args.get(2) else {
                return usage();
            };
            let iters: u64 = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(200);
            let data = match std::fs::read(input) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("{input}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let lower = input.to_ascii_lowercase();
            let mut panics = 0;
            std::panic::set_hook(Box::new(|info| eprintln!("panic: {info}")));
            for seed in 0..iters {
                let m = mutate(&data, seed * 7919 + 13);
                let lower = lower.clone();
                let ok = std::panic::catch_unwind(move || {
                    let mut ids = tracedraw_core::id::IdSource::default();
                    if lower.ends_with(".cdr")
                        || lower.ends_with(".cdt")
                        || lower.ends_with(".cdrt")
                        || lower.ends_with(".cmx")
                    {
                        let _ = tracedraw_cdr::open_bytes(&m, "stress");
                    } else if lower.ends_with(".pdf") || lower.ends_with(".ai") {
                        let _ = tracedraw_io::pdf_import::parse(&m, &mut ids);
                    } else if lower.ends_with(".eps") || lower.ends_with(".ps") {
                        let _ = tracedraw_io::eps_import::parse(&m, &mut ids);
                    } else if lower.ends_with(".dxf") {
                        let _ = tracedraw_io::dxf::parse(&String::from_utf8_lossy(&m), &mut ids);
                    } else if lower.ends_with(".psd") || lower.ends_with(".psb") {
                        let _ = tracedraw_io::psd::parse(&m, &mut ids);
                    } else if lower.ends_with(".emf") || lower.ends_with(".wmf") {
                        let _ = tracedraw_io::emf::parse(&m, &mut ids);
                    } else if lower.ends_with(".plt") {
                        let _ = tracedraw_io::plt::parse(&m, &mut ids);
                    } else if lower.ends_with(".rtf") {
                        let _ = tracedraw_io::text_import::parse_rtf(&m);
                    } else if lower.ends_with(".docx") {
                        let _ = tracedraw_io::text_import::parse_docx(&m);
                    } else if lower.ends_with(".svg") {
                        let _ =
                            tracedraw_io::svg_import::parse(&String::from_utf8_lossy(&m), &mut ids);
                    } else {
                        let _ = tracedraw_core::Document::from_json(&String::from_utf8_lossy(&m));
                    }
                })
                .is_ok();
                if !ok {
                    panics += 1;
                    eprintln!("seed {seed} panicked");
                }
            }
            outln!("{input}: {iters} mutated copies, {panics} panic(s)");
            if panics == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Some("convert") => {
            let (Some(input), Some(output)) = (args.get(2), args.get(3)) else {
                return usage();
            };
            let (doc, report) = match load(input) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{input}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            for line in report.as_ref().map(reader_warnings).unwrap_or_default() {
                eprintln!("{line}");
            }
            let res = if output.to_ascii_lowercase().ends_with(".svg") {
                tracedraw_io::save_svg(&doc, 0, output)
            } else if output.to_ascii_lowercase().ends_with(".pdf") {
                // TRACEDRAW_PDFX=x1a|x3|x4 selects a PDF/X level; TRACEDRAW_ICC
                // names the output profile (default: the CMYK profile the
                // drawing carries) and TRACEDRAW_BLEED the bleed in mm.
                // TRACEDRAW_PDF_BOOKMARKS=1 adds a bookmark per page; the
                // TRACEDRAW_PDF_*PASSWORD and _DENY variables protect it.
                // TRACEDRAW_PDF_TEXT=curves writes text as curves, =named
                // leaves the fonts out of the file.
                let standard = match std::env::var("TRACEDRAW_PDFX")
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .as_str()
                {
                    "x1a" | "x-1a" => tracedraw_io::pdf::PdfStandard::X1a,
                    "x3" | "x-3" => tracedraw_io::pdf::PdfStandard::X3,
                    "x4" | "x-4" => tracedraw_io::pdf::PdfStandard::X4,
                    _ => tracedraw_io::pdf::PdfStandard::None,
                };
                let opts = tracedraw_io::pdf::PdfOptions {
                    standard,
                    output_profile: std::env::var("TRACEDRAW_ICC")
                        .ok()
                        .and_then(|p| std::fs::read(p).ok()),
                    output_condition: std::env::var("TRACEDRAW_CONDITION").unwrap_or_default(),
                    bleed_mm: std::env::var("TRACEDRAW_BLEED")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0),
                    title: None,
                    flatten_dpi: 300.0,
                    bookmarks: std::env::var("TRACEDRAW_PDF_BOOKMARKS")
                        .is_ok_and(|v| !matches!(v.as_str(), "" | "0" | "no" | "false")),
                    security: pdf_security(
                        std::env::var("TRACEDRAW_PDF_OPEN_PASSWORD").ok(),
                        std::env::var("TRACEDRAW_PDF_OWNER_PASSWORD").ok(),
                        std::env::var("TRACEDRAW_PDF_DENY").ok(),
                    ),
                    text: pdf_text(std::env::var("TRACEDRAW_PDF_TEXT").ok().as_deref()),
                    ..Default::default()
                };
                std::fs::write(output, tracedraw_io::pdf::document_to_pdf_with(&doc, &opts))
                    .map_err(tracedraw_io::Error::from)
            } else if output.to_ascii_lowercase().ends_with(".eps") {
                std::fs::write(output, tracedraw_io::eps::page_to_eps(&doc, 0))
                    .map_err(tracedraw_io::Error::from)
            } else if [".ps", ".prn"]
                .iter()
                .any(|e| output.to_ascii_lowercase().ends_with(e))
            {
                std::fs::write(output, tracedraw_io::eps::document_to_ps(&doc))
                    .map_err(tracedraw_io::Error::from)
            } else if output.to_ascii_lowercase().ends_with(".cdr") {
                // TRACEDRAW_CDR_VERSION picks the version written (14 to
                // 27, the versions checked to open in other programs; the
                // experimental 7 to 13 only with TRACEDRAW_CDR_EXPERIMENTAL=1).
                let asked: u16 = std::env::var("TRACEDRAW_CDR_VERSION")
                    .ok()
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(tracedraw_cdr::write::DEFAULT_VERSION);
                let experimental =
                    std::env::var("TRACEDRAW_CDR_EXPERIMENTAL").is_ok_and(|v| v.trim() == "1");
                let allowed = if experimental {
                    tracedraw_cdr::write::versions()
                } else {
                    tracedraw_cdr::write::checked_versions()
                };
                if !allowed.contains(&asked) {
                    eprintln!(
                        "cdr version {asked} is not saved: use {} to {}{}",
                        allowed.start(),
                        allowed.end(),
                        if experimental {
                            ""
                        } else {
                            " (7 to 13 are experimental: TRACEDRAW_CDR_EXPERIMENTAL=1)"
                        }
                    );
                    std::process::exit(2);
                }
                let version = asked;
                std::fs::write(
                    output,
                    tracedraw_cdr::write::document_to_cdr_version_with(&doc, version, &{
                        let pages: Vec<tracedraw_cdr::write::Png> =
                            tracedraw_io::page_pictures(&doc, 256)
                                .into_iter()
                                .map(|(data, width, height)| tracedraw_cdr::write::Png {
                                    data,
                                    width,
                                    height,
                                })
                                .collect();
                        // The text's faces go in the font table (from 16);
                        // TRACEDRAW_CDR_EMBED_FONTS=1 embeds them.
                        let fonts = tracedraw_text::fonts();
                        tracedraw_cdr::write::Previews {
                            thumbnail: pages.first().cloned(),
                            pages,
                            fonts: doc
                                .font_faces_used()
                                .into_iter()
                                .map(|(family, bold, italic)| tracedraw_cdr::write::FontFace {
                                    data: fonts.face_file(&family, bold, italic),
                                    family,
                                    bold,
                                    italic,
                                })
                                .collect(),
                            embed_fonts: std::env::var("TRACEDRAW_CDR_EMBED_FONTS")
                                .is_ok_and(|v| v.trim() == "1"),
                        }
                    }),
                )
                .map_err(tracedraw_io::Error::from)
            } else if output.to_ascii_lowercase().ends_with(".emf") {
                tracedraw_io::save_emf(&doc, page_index(), output)
            } else if output.to_ascii_lowercase().ends_with(".wmf") {
                tracedraw_io::save_wmf(&doc, page_index(), output)
            } else if output.to_ascii_lowercase().ends_with(".plt") {
                tracedraw_io::save_plt(&doc, page_index(), output)
            } else if output.to_ascii_lowercase().ends_with(".psd") {
                let dpi = std::env::var("TRACEDRAW_DPI")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(96.0);
                tracedraw_io::save_psd(&doc, page_index(), dpi, output)
            } else if output.to_ascii_lowercase().ends_with(".dxf") {
                tracedraw_io::save_dxf(&doc, page_index(), output)
            } else if output.to_ascii_lowercase().ends_with(".html") {
                std::fs::write(output, tracedraw_io::html::document_to_html(&doc))
                    .map_err(tracedraw_io::Error::from)
            } else if output.to_ascii_lowercase().ends_with(".png") {
                // Rasterise the first page (or TRACEDRAW_PAGE) at 96 dpi
                // (or TRACEDRAW_DPI).
                let dpi = std::env::var("TRACEDRAW_DPI")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(96.0);
                match doc
                    .pages
                    .get(page_index())
                    .and_then(|p| tracedraw_render::render_page_image(&doc, p.id, dpi))
                {
                    Some(pm) => pm
                        .save_png(output)
                        .map_err(|e| tracedraw_io::Error::Io(std::io::Error::other(e))),
                    None => Err(tracedraw_io::Error::Io(std::io::Error::other(
                        "render failed",
                    ))),
                }
            } else {
                tracedraw_io::save_native(&doc, output)
            };
            match res {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("{output}: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => usage(),
    }
}

/// One line per object (and its children), indented by `depth`: kind,
/// geometry, transform, fill, outline and text.
fn describe_shape(s: &tracedraw_core::Shape, depth: usize) -> String {
    use tracedraw_core::document::ShapeKind;
    let pad = "  ".repeat(depth);
    let kind = match &s.kind {
        ShapeKind::Rect {
            rect,
            radius,
            corners,
        } => format!(
            "rect {:.2}x{:.2} at ({:.2},{:.2}) radius {:.2}{}",
            rect.width(),
            rect.height(),
            rect.x0,
            rect.y0,
            radius,
            corners
                .as_ref()
                .map(|c| format!(" corners {c:?}"))
                .unwrap_or_default()
        ),
        ShapeKind::Ellipse { rect, arc } => format!(
            "ellipse {:.2}x{:.2} at ({:.2},{:.2}){}",
            rect.width(),
            rect.height(),
            rect.x0,
            rect.y0,
            arc.as_ref()
                .map(|a| format!(" arc {a:?}"))
                .unwrap_or_default()
        ),
        ShapeKind::Polygon { rect, points, .. } => {
            format!(
                "polygon {points} points {:.2}x{:.2}",
                rect.width(),
                rect.height()
            )
        }
        ShapeKind::Path { path, closed } => format!(
            "path {} elements{}",
            path.elements().len(),
            if *closed { " closed" } else { "" }
        ),
        ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para,
            on_path,
        } => {
            let runs: Vec<String> = spans
                .iter()
                .map(|r| {
                    format!(
                        "{:?} [{} {:.1}pt{}{}]",
                        r.text,
                        r.font_family,
                        r.size_pt,
                        if r.bold { " bold" } else { "" },
                        if r.italic { " italic" } else { "" }
                    )
                })
                .collect();
            format!(
                "text at ({:.2},{:.2}) frame {:?} align {:?} leading {:.0}%{} {}",
                origin.x,
                origin.y,
                frame.map(|f| (f.width, f.height)),
                align,
                para.leading_pct,
                if on_path.is_some() { " on path" } else { "" },
                runs.join(" + ")
            )
        }
        ShapeKind::Group { children } => format!("group of {}", children.len()),
        ShapeKind::ClipFrame { contents, .. } => format!("clip frame of {}", contents.len()),
        ShapeKind::Table(_) => "table".to_string(),
        ShapeKind::SymbolInstance { index } => format!("symbol {index}"),
        ShapeKind::Bitmap {
            rect,
            width_px,
            height_px,
            ..
        } => format!(
            "bitmap {width_px}x{height_px} px in {:.2}x{:.2} mm",
            rect.width(),
            rect.height()
        ),
    };
    let b = s.bounds();
    let t = s.transform.as_coeffs();
    let fill = match &s.fill {
        tracedraw_core::Fill::None => "none".to_string(),
        tracedraw_core::Fill::Solid(c) => format!("{c:?}"),
        other => format!("{:?}", std::mem::discriminant(other)),
    };
    let stroke = s
        .stroke
        .as_ref()
        .map(|o| format!("{:.3} mm {:?}", o.width, o.color))
        .unwrap_or_else(|| "none".to_string());
    let mut out = format!(
        "{pad}{kind}\n{pad}  bounds ({:.2},{:.2})-({:.2},{:.2}) transform [{:.3} {:.3} {:.3} {:.3} {:.2} {:.2}] opacity {:.2}\n{pad}  fill {fill}; outline {stroke}\n",
        b.x0, b.y0, b.x1, b.y1, t[0], t[1], t[2], t[3], t[4], t[5], s.opacity
    );
    match &s.kind {
        ShapeKind::Group { children } => {
            for c in children {
                out.push_str(&describe_shape(c, depth + 1));
            }
        }
        ShapeKind::ClipFrame { frame, contents } => {
            out.push_str(&describe_shape(frame, depth + 1));
            for c in contents {
                out.push_str(&describe_shape(c, depth + 1));
            }
        }
        _ => {}
    }
    out
}

/// The page the single-page exports write: TRACEDRAW_PAGE, counted from
/// 1 (the first page when unset or unreadable).
fn page_index() -> usize {
    page_from(std::env::var("TRACEDRAW_PAGE").ok().as_deref())
}

fn page_from(v: Option<&str>) -> usize {
    v.and_then(|v| v.trim().parse::<usize>().ok())
        .map(|n| n.saturating_sub(1))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_to_export_counts_from_one() {
        assert_eq!(page_from(None), 0);
        assert_eq!(page_from(Some("1")), 0);
        assert_eq!(page_from(Some(" 3 ")), 2);
        assert_eq!(page_from(Some("0")), 0);
        assert_eq!(page_from(Some("two")), 0);
    }

    #[test]
    fn pdf_text_comes_from_the_environment_value() {
        assert!(pdf_text(Some("curves")).as_curves);
        assert!(!pdf_text(Some("named")).embed_fonts);
        let t = pdf_text(None);
        assert!(!t.as_curves && t.embed_fonts && t.subset);
    }

    #[test]
    fn pdf_protection_comes_from_the_environment_values() {
        assert!(pdf_security(None, None, Some("print".into())).is_none());
        // A password to open it only: everything allowed, one password.
        let s = pdf_security(Some("open".into()), None, Some("print".into())).expect("set");
        assert_eq!(
            (s.user_password.as_str(), s.owner_password.as_str()),
            ("open", "open")
        );
        assert!(s.permissions.print);
        // A permissions password with what to deny.
        let s =
            pdf_security(None, Some("boss".into()), Some("print, Copy,forms".into())).expect("set");
        assert!(s.user_password.is_empty());
        assert_eq!(s.owner_password, "boss");
        assert!(!s.permissions.print && !s.permissions.copy && !s.permissions.fill_forms);
        assert!(s.permissions.modify && s.permissions.accessibility);
    }

    #[test]
    fn reader_warnings_name_what_was_skipped_or_guessed() {
        let clean = tracedraw_cdr::ParseReport::default();
        assert!(reader_warnings(&clean).is_empty());
        let report = tracedraw_cdr::ParseReport {
            skipped_objects: 2,
            warnings: vec!["no readable mcfg chunk; assuming A4".into()],
            ..Default::default()
        };
        assert_eq!(
            reader_warnings(&report),
            [
                "warning: no readable mcfg chunk; assuming A4",
                "warning: 2 object(s) skipped"
            ]
        );
    }
}
