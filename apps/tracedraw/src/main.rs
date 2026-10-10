#![allow(
    clippy::field_reassign_with_default,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::enum_variant_names,
    clippy::wrong_self_convention
)]
// No console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! TraceDraw desktop app: a thin egui shell over the engine.

mod anchors;
mod app;
mod barcode;
mod bitmap_fx;
mod bitmap_modes;
mod border_grommet;
mod canvas;
mod clipboard;
mod clone_effect;
mod coords;
mod corners;
mod documents;
mod effects_ui;
mod encode;
mod export;
mod eyedropper;
mod files;
mod fill_tool;
mod fx;
mod grammar;
mod guides;
mod i18n;
mod interaction;
mod interaction2;
mod kind_nodes;
mod lens;
#[cfg(not(target_arch = "wasm32"))]
mod mcp;
mod media;
mod new_document;
mod node_edit;
mod ops;
mod ops2;
mod palette;
mod picture_mosaic;
mod vector_mosaic;
mod autocorrect;
mod raster;
mod scripting;
mod settings;
mod shape_tool;
mod snap;
mod snap_points;
mod spell;
mod straighten;
mod table;
mod text_editing;
mod text_nodes;
mod textflow;
mod theme;
mod thesaurus;
mod tools;
mod tools2;
mod trace;
mod ui;
mod view;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    tracedraw_text::install();
    // `tracedraw --mcp`: no window, a Model Context Protocol server on
    // stdin/stdout (see mcp.rs and docs/behavior/mcp.md).
    if std::env::args().any(|a| a == "--mcp") {
        mcp::serve();
        return Ok(());
    }
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

/// The browser build starts from `web::start` (the page loads the module,
/// which runs `main`).
#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
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
        // File dialogs that finished in the background (browser build).
        files::run_finished(&mut self.app);
        // A PictureMosaic library indexed on worker threads.
        if self.app.poll_picture_mosaic_index() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        ui::root(&mut self.app, ui);
    }

    /// Keep the toolbox, dockers and other preferences for the next session.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.app.save_settings();
    }
}
