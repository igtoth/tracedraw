//! tracedraw-cli: headless tools.
//!
//! ```text
//! tracedraw-cli inspect file.cdr          dump the RIFF chunk tree
//! tracedraw-cli info file.cdr             version, pages, objects, warnings
//! tracedraw-cli convert in.cdr out.svg    export the first page (or .tdraw)
//! ```

use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage:\n  tracedraw-cli inspect <file.cdr>\n  tracedraw-cli info <file.cdr|file.tdraw>\n  tracedraw-cli convert <in.cdr|in.tdraw> <out.svg|out.tdraw>");
    ExitCode::from(2)
}

fn load(
    path: &str,
) -> Result<(tracedraw_core::Document, Option<tracedraw_cdr::ParseReport>), String> {
    if path.to_ascii_lowercase().ends_with(".cdr") {
        tracedraw_cdr::open(path)
            .map(|(d, r)| (d, Some(r)))
            .map_err(|e| e.to_string())
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
