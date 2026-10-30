// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! The TraceDraw program: the whole app is the `tracedraw` library; this
//! is the desktop and browser entry (the Android app loads the library).

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    tracedraw::run()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    tracedraw::run();
}
