// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! TraceDraw desktop app: a thin egui shell over the engine.

mod app;
mod barcode;
mod bitmap_fx;
mod border_grommet;
mod canvas;
mod clipboard;
mod effects_ui;
mod export;
mod grammar;
mod i18n;
mod interaction;
mod interaction2;
mod lens;
mod media;
mod ops;
mod ops2;
mod palette;
mod autocorrect;
mod raster;
mod scripting;
mod settings;
mod shape_tool;
mod snap;
mod spell;
mod table;
mod textflow;
mod theme;
mod thesaurus;
mod tools;
mod tools2;
mod trace;
mod ui;
mod view;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    tracedraw_text::install();
    let open_path = std::env::args().nth(1).map(std::path::PathBuf::from);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1600.0, 950.0])
            .with_min_inner_size([1000.0, 600.0])
            .with_title("TraceDraw"),
        ..Default::default()
    };
    eframe::run_native(
        "TraceDraw",
        options,
        Box::new(move |cc| Ok(Box::new(Shell::new(cc, open_path)))),
    )
}

struct Shell {
    app: app::App,
}

impl Shell {
    fn new(cc: &eframe::CreationContext<'_>, open: Option<std::path::PathBuf>) -> Self {
        Shell {
            app: app::App::new(cc, open),
        }
    }
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui::root(&mut self.app, ui);
    }
}
