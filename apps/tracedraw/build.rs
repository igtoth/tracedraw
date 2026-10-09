// Embed the icon and version info into the Windows executable, and list
// the fonts bundled into the browser build.
fn main() {
    web_fonts();
    println!("cargo:rerun-if-changed=../../assets/icon/tracedraw.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icon/tracedraw.ico");
        res.set("ProductName", "TraceDraw");
        res.set("FileDescription", "TraceDraw vector editor");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed Windows resources: {e}");
        }
    }
}

/// The browser build has no system fonts: bundle the font files found on
/// the build machine (directories in TRACEDRAW_WEB_FONTS, separated by
/// ':', or the usual Linux font folders). Writes `web_fonts.rs` into
/// OUT_DIR; the list is empty for native builds.
fn web_fonts() {
    println!("cargo:rerun-if-env-changed=TRACEDRAW_WEB_FONTS");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default());
    let wasm = std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32");
    let mut entries = Vec::new();
    if wasm {
        let dirs: Vec<std::path::PathBuf> = match std::env::var("TRACEDRAW_WEB_FONTS") {
            Ok(v) if !v.is_empty() => v.split(':').map(std::path::PathBuf::from).collect(),
            _ => [
                "/usr/share/fonts/truetype/liberation",
                "/usr/share/fonts/truetype/liberation2",
                "/usr/share/fonts/truetype/dejavu",
                "/usr/share/fonts/truetype/noto",
            ]
            .iter()
            .map(std::path::PathBuf::from)
            .collect(),
        };
        let wanted = [
            "LiberationSans-Regular.ttf",
            "LiberationSans-Bold.ttf",
            "LiberationSans-Italic.ttf",
            "LiberationSans-BoldItalic.ttf",
            "LiberationSerif-Regular.ttf",
            "LiberationSerif-Bold.ttf",
            "LiberationSerif-Italic.ttf",
            "LiberationSerif-BoldItalic.ttf",
            "LiberationMono-Regular.ttf",
            "LiberationMono-Bold.ttf",
            "DejaVuSans.ttf",
            "NotoSansDevanagari-Regular.ttf",
            "NotoSansBengali-Regular.ttf",
        ];
        for name in wanted {
            if let Some(path) = dirs.iter().map(|d| d.join(name)).find(|p| p.is_file()) {
                println!("cargo:rerun-if-changed={}", path.display());
                entries.push(format!(
                    "    ({name:?}, include_bytes!({:?})),",
                    path.display().to_string()
                ));
            }
        }
        if entries.is_empty() {
            println!("cargo:warning=no fonts found for the browser build; set TRACEDRAW_WEB_FONTS");
        }
    }
    let code = format!(
        "/// Font files bundled at build time: (file name, data).\npub static WEB_FONTS: &[(&str, &[u8])] = &[\n{}\n];\n",
        entries.join("\n")
    );
    let _ = std::fs::write(out.join("web_fonts.rs"), code);
}
