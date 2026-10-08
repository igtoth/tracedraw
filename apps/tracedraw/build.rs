// Embed the icon and version info into the Windows executable.
fn main() {
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
