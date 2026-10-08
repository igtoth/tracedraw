// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Traco desktop app: a thin egui shell over the engine.

mod app;
mod canvas;
mod view;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let open_path = std::env::args().nth(1).map(std::path::PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]).with_title("Traco"),
        ..Default::default()
    };
    eframe::run_native("Traco", options, Box::new(move |cc| Ok(Box::new(app::App::new(cc, open_path)))))
}
