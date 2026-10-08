//! traco-cli: headless tools.
//!
//! ```text
//! traco-cli inspect file.cdr          dump the RIFF chunk tree
//! traco-cli info file.cdr             version, pages, objects, warnings
//! traco-cli convert in.cdr out.svg    export the first page (or .traco)
//! ```

use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage:\n  traco-cli inspect <file.cdr>\n  traco-cli info <file.cdr|file.traco>\n  traco-cli convert <in.cdr|in.traco> <out.svg|out.traco>");
    ExitCode::from(2)
}

fn load(path: &str) -> Result<(traco_core::Document, Option<traco_cdr::ParseReport>), String> {
    if path.to_ascii_lowercase().ends_with(".cdr") {
        traco_cdr::open(path).map(|(d, r)| (d, Some(r))).map_err(|e| e.to_string())
    } else {
        traco_io::load_native(path).map(|d| (d, None)).map_err(|e| e.to_string())
    }
}

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("inspect") => {
            let Some(path) = args.get(2) else { return usage() };
            let bytes = match std::fs::read(path) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("{path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let container = match traco_cdr::container::detect(&bytes) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("{path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            println!("container: {:?}, {}", container.kind, container.version.name());
            let riff = match traco_cdr::container::riff_stream(&bytes, &container) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("{path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            match traco_cdr::riff::parse(&riff) {
                Ok(tree) => print!("{}", tree.dump()),
                Err(e) => {
                    eprintln!("{path}: {e}");
                    return ExitCode::FAILURE;
                }
            }
            ExitCode::SUCCESS
        }
        Some("info") => {
            let Some(path) = args.get(2) else { return usage() };
            match load(path) {
                Ok((doc, report)) => {
                    println!("title: {}", doc.title);
                    for p in &doc.pages {
                        let n: usize = p.layers.iter().map(|l| l.shapes.len()).sum();
                        println!("{}: {:.1} x {:.1} mm, {} layer(s), {} object(s)", p.name, p.size.width, p.size.height, p.layers.len(), n);
                    }
                    if let Some(r) = report {
                        println!("version: {}", r.version.map(|v| v.name()).unwrap_or_default());
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
            let (Some(input), Some(output)) = (args.get(2), args.get(3)) else { return usage() };
            let (doc, _) = match load(input) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{input}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let res = if output.to_ascii_lowercase().ends_with(".svg") {
                traco_io::save_svg(&doc, 0, output)
            } else {
                traco_io::save_native(&doc, output)
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
