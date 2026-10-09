//! tracedraw-cli: headless tools.
//!
//! ```text
//! tracedraw-cli inspect file.cdr          dump the RIFF chunk tree
//! tracedraw-cli info file.cdr             version, pages, objects, warnings
//! tracedraw-cli convert in.cdr out.svg    export the first page (or .tdraw)
//! ```

use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage:\n  tracedraw-cli inspect <file.cdr>\n  tracedraw-cli info <file.cdr|file.tdraw>\n  tracedraw-cli icc <profile.icc>\n  tracedraw-cli convert <in.cdr|in.tdraw|in.svg|in.pdf|in.ai|in.dxf> <out.svg|out.pdf|out.eps|out.dxf|out.html|out.png|out.tdraw>");
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
                tracedraw_io::save_pdf(&doc, output)
            } else if output.to_ascii_lowercase().ends_with(".eps") {
                std::fs::write(output, tracedraw_io::eps::page_to_eps(&doc, 0))
                    .map_err(tracedraw_io::Error::from)
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
