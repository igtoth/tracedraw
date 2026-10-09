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
    eprintln!("usage:\n  tracedraw-cli inspect <file.cdr>\n  tracedraw-cli info <file.cdr|file.tdraw>\n  tracedraw-cli icc <profile.icc>\n  tracedraw-cli stress <file> [iterations]   mutation test of the file's reader\n  tracedraw-cli convert <in.cdr|in.tdraw|in.svg|in.pdf|in.ai|in.eps|in.dxf|in.psd|in.emf|in.wmf> <out.svg|out.pdf|out.eps|out.dxf|out.emf|out.wmf|out.html|out.png|out.tdraw>");
    ExitCode::from(2)
}

fn load(
    path: &str,
) -> Result<(tracedraw_core::Document, Option<tracedraw_cdr::ParseReport>), String> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".cdr") {
        tracedraw_cdr::open(path)
            .map(|(d, r)| (d, Some(r)))
            .map_err(|e| e.to_string())
    } else if lower.ends_with(".pdf") || lower.ends_with(".ai") {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let imported =
            tracedraw_io::pdf_import::parse(&bytes, &mut tracedraw_core::id::IdSource::default())?;
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
    } else if lower.ends_with(".eps") || lower.ends_with(".ps") {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let imported =
            tracedraw_io::eps_import::parse(&bytes, &mut tracedraw_core::id::IdSource::default())?;
        for w in &imported.warnings {
            eprintln!("warning: {w}");
        }
        let mut doc = tracedraw_core::Document::new("eps", imported.size);
        doc.pages[0].layers[0].shapes = imported.shapes;
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
            println!(
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
                Ok(tree) => print!("{}", tree.dump()),
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
                    println!("title: {}", doc.title);
                    for p in &doc.pages {
                        let n: usize = p.layers.iter().map(|l| l.shapes.len()).sum();
                        println!(
                            "{}: {:.1} x {:.1} mm, {} layer(s), {} object(s)",
                            p.name,
                            p.size.width,
                            p.size.height,
                            p.layers.len(),
                            n
                        );
                    }
                    if let Some(r) = report {
                        println!(
                            "version: {}",
                            r.version.map(|v| v.name()).unwrap_or_default()
                        );
                        println!("skipped objects: {}", r.skipped_objects);
                        for w in &r.warnings {
                            println!("warning: {w}");
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
            println!(
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
                    println!("  {s:?} -> sRGB {rgb:?}");
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
                    if lower.ends_with(".cdr") {
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
            println!("{input}: {iters} mutated copies, {panics} panic(s)");
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
            let (doc, _) = match load(input) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{input}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let res = if output.to_ascii_lowercase().ends_with(".svg") {
                tracedraw_io::save_svg(&doc, 0, output)
            } else if output.to_ascii_lowercase().ends_with(".pdf") {
                // TRACEDRAW_PDFX=x1a|x3|x4 selects a PDF/X level; TRACEDRAW_ICC
                // names the output profile and TRACEDRAW_BLEED the bleed in mm.
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
                };
                std::fs::write(output, tracedraw_io::pdf::document_to_pdf_with(&doc, &opts))
                    .map_err(tracedraw_io::Error::from)
            } else if output.to_ascii_lowercase().ends_with(".eps") {
                std::fs::write(output, tracedraw_io::eps::page_to_eps(&doc, 0))
                    .map_err(tracedraw_io::Error::from)
            } else if output.to_ascii_lowercase().ends_with(".emf") {
                tracedraw_io::save_emf(&doc, 0, output)
            } else if output.to_ascii_lowercase().ends_with(".wmf") {
                tracedraw_io::save_wmf(&doc, 0, output)
            } else if output.to_ascii_lowercase().ends_with(".dxf") {
                tracedraw_io::save_dxf(&doc, 0, output)
            } else if output.to_ascii_lowercase().ends_with(".html") {
                std::fs::write(output, tracedraw_io::html::document_to_html(&doc))
                    .map_err(tracedraw_io::Error::from)
            } else if output.to_ascii_lowercase().ends_with(".png") {
                // Rasterise the first page at 96 dpi (or TRACEDRAW_DPI).
                let dpi = std::env::var("TRACEDRAW_DPI")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(96.0);
                match doc
                    .pages
                    .first()
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
