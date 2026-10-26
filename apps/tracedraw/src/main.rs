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

mod ai;
mod ai_image;
mod anchors;
mod app;
mod autocorrect;
mod barcode;
mod bg_remove;
mod bitmap_fx;
mod bitmap_modes;
mod blend_link;
mod border_grommet;
mod brushes;
mod callouts;
mod canvas;
mod clipboard;
mod clone_effect;
mod color_drop;
mod color_policy;
mod comments;
mod common_shapes;
mod connectors;
mod contour_tool;
mod coords;
mod copy_effect;
mod corner_widgets;
mod corners;
mod crop_tool;
mod curve_join;
mod defaults;
mod describe;
#[cfg(test)]
mod devanagari_join_tests;
mod dimensions;
mod distort_tool;
mod doc_stats;
mod documents;
mod drawing_scale;
mod drop_files;
mod drop_menu;
mod effects_ui;
mod encode;
mod envelope_edit;
mod envelope_shapes;
mod export;
mod export_assets;
mod extrude_tool;
mod eyedropper;
mod files;
mod fill_tool;
mod focus;
#[cfg(any(target_arch = "wasm32", test))]
mod font_folder;
#[cfg(not(target_arch = "wasm32"))]
mod frame_stats;
mod frame_type;
mod fx;
mod github;
#[cfg(not(target_arch = "wasm32"))]
mod gpu;
mod gradient_presets;
mod grammar;
mod guides;
mod help;
#[cfg(test)]
mod help_shots;
mod hunspell;
mod i18n;
mod image_dpi;
mod imposition;
#[cfg(not(target_arch = "wasm32"))]
mod instance;
mod interaction;
mod interaction2;
mod kind_nodes;
mod knife;
mod lens;
mod live_guides;
mod macro_forms;
mod macros;
mod mcp;
#[cfg(not(target_arch = "wasm32"))]
mod mcp_link;
mod media;
mod mesh_tool;
mod mixed;
#[cfg(test)]
mod mockup_tests;
mod multipage;
mod net;
mod new_document;
mod node_edit;
mod object_clone;
mod ops;
mod ops2;
mod painterly_tool;
mod palette;
mod pen_look;
mod perspective_draw;
#[cfg(test)]
mod pick_drag_tests;
mod picture_mosaic;
mod placeholder;
mod preflight;
mod prefs;
mod print_merge;
mod printers;
mod qr;
mod raster;
mod recognize;
mod recovery;
mod resample;
mod script_debug;
mod script_filters;
mod scripting;
mod segment_delete;
mod select_region;
mod settings;
mod shadow_tool;
mod shape_tool;
mod shelf;
mod shortcuts;
mod snap;
mod snap_points;
mod spell;
#[cfg(not(target_arch = "wasm32"))]
mod splash;
#[cfg(not(target_arch = "wasm32"))]
mod startup_log;
mod straighten;
mod styles;
mod symmetry_edit;
mod system_prefs;
mod table;
mod table_tool;
mod templates;
mod text_editing;
#[cfg(test)]
mod text_effect_tests;
mod text_frames;
mod text_nodes;
mod text_path;
#[cfg(test)]
mod text_wrap_tests;
mod textflow;
mod theme;
mod thesaurus;
mod thumbnails;
mod time_tracker;
mod tools;
mod tools2;
mod trace;
mod ui;
mod updates;
mod variable_outline;
mod vector_mosaic;
mod view;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(any(target_arch = "wasm32", test))]
mod web_page;
mod workspace_file;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // One window for every start (#646): files given to a start while
    // TraceDraw runs go to its window, in tabs of their own, and this
    // start ends (before its log would replace the running one's).
    let files = instance::file_args(&args);
    let (inbox, handed_over) = std::sync::mpsc::channel();
    let mut primary = None;
    if instance::hands_over(&args) {
        match instance::claim(settings::config_dir().as_deref(), &files, inbox.clone()) {
            instance::Claim::HandedOver => return Ok(()),
            instance::Claim::Primary(p) => primary = Some(p),
            instance::Claim::Alone => {}
        }
    }
    // The log in a file too and panics reported: the Windows build has no
    // console to show them (#570).
    startup_log::init();
    tracedraw_text::install();
    // `tracedraw --mcp`: no window, a Model Context Protocol server on
    // stdin/stdout (see mcp.rs and docs/behavior/mcp.md).
    if args.iter().any(|a| a == "--mcp") {
        mcp::serve();
        return Ok(());
    }
    // `tracedraw --mcp-window`: the same tools on the drawings of the open
    // window, which must let AI agents in (see mcp_link.rs).
    if args.iter().any(|a| a == "--mcp-window") {
        mcp_link::bridge();
        return Ok(());
    }
    // `--no-splash` or TRACEDRAW_NO_SPLASH (automation, the visual
    // checks): straight into a plain window at the default size.
    let plain = args.iter().any(|a| a == "--no-splash")
        || std::env::var("TRACEDRAW_NO_SPLASH").is_ok_and(|v| !v.is_empty() && v != "0");
    // The first file opens as the window starts, the others after it.
    let mut files = files.into_iter();
    let open_path = files.next();
    let rest: Vec<std::path::PathBuf> = files.collect();
    if !rest.is_empty() {
        let _ = inbox.send(rest);
    }
    let wake = primary.as_ref().map(|p| p.wake.clone());
    let mut settings = settings::Settings::load();
    // `--reset-settings`: every preference back to how TraceDraw comes
    // (the recent drawings and the window's size are kept).
    if args.iter().any(|a| a == "--reset-settings") {
        settings = settings.reset_keeping_history();
        settings.save();
    }
    i18n::set_language(&if settings.language.is_empty() {
        i18n::system_language().to_string()
    } else {
        settings.language.clone()
    });
    let splash = settings.show_splash && !plain;
    let viewport = if splash {
        // The splash has no frame, so it cannot be dragged larger; it is
        // still created resizable: some window managers (Xfwm4) keep a
        // window made not resizable at its size for good, and the main
        // window it becomes could then neither be resized nor maximized.
        egui::ViewportBuilder::default()
            .with_inner_size(splash::SIZE)
            .with_decorations(false)
            .with_resizable(true)
            .with_title("TraceDraw")
    } else {
        egui::ViewportBuilder::default()
            .with_inner_size(splash::main_window_size(settings.window_size, None))
            .with_min_inner_size(splash::MIN_SIZE)
            .with_maximized(settings.window_maximized && !plain)
            .with_title("TraceDraw")
    };
    // The graphics card, or its software adapter with hardware
    // acceleration off; `--renderer=` when a start before could not open
    // the window (#445, #439).
    // A start before that never got its window (the driver crashed or
    // hung it) goes on with the next renderer (#570).
    let failed = gpu::Mode::from_args(&args)
        .is_none()
        .then(gpu::failed_attempt)
        .flatten();
    if let Some(f) = failed {
        log::warn!("the last start did not open the window with {f:?}");
    }
    let mode = gpu::Mode::for_start(&args, &settings, failed);
    let first = gpu::Mode::first(&settings);
    let next = mode.next(first);
    log::info!("opening the window with {mode:?} (Options asks for {first:?})");
    gpu::begin_attempt(mode);
    // A renderer that hangs never returns here to hand over (#570).
    gpu::watch_start(
        mode,
        next,
        args.clone(),
        primary.as_ref().map(|p| p.decline.clone()),
    );
    let (renderer, wgpu_options) = gpu::options(mode, &settings.gpu_adapter);
    let options = eframe::NativeOptions {
        viewport,
        centered: splash,
        renderer,
        wgpu_options,
        ..Default::default()
    };
    // A panic before the window opens is this renderer failing (eframe's
    // OpenGL set-up panics when the driver offers no matching format):
    // caught, so the next renderer is still tried (#616).
    startup_log::quiet_panics(next.is_some());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        eframe::run_native(
            "TraceDraw",
            options,
            Box::new(move |cc| {
                gpu::note_renderer(cc, mode);
                if let Some(w) = &wake {
                    let _ = w.set(cc.egui_ctx.clone());
                }
                let mut shell = if splash {
                    Shell::with_splash(cc, open_path)
                } else {
                    Shell::new(cc, open_path)
                };
                shell.handed_over = Some(handed_over);
                Ok(Box::new(shell))
            }),
        )
    }));
    startup_log::quiet_panics(false);
    // A panic of the last renderer was reported as a crash already.
    let reported = result.is_err() && next.is_none();
    let result = match result {
        Ok(r) => r,
        Err(panic) if gpu::window_opened() => std::panic::resume_unwind(panic),
        Err(panic) => {
            let what = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "a panic".into());
            Err(eframe::Error::AppCreation(what.into()))
        }
    };
    // A start with the next renderer must be able to take the place.
    drop(primary);
    let Err(e) = result else {
        return Ok(());
    };
    // The window could not be opened this way: the next renderer, in a
    // new start (a window system cannot be set up twice in one).
    if let Some(next) = next {
        log::warn!("the window did not open with {mode:?} ({e}); trying {next:?}");
        if gpu::restart_with(next, &args) {
            return Ok(());
        }
    }
    log::error!("the window could not be opened with {mode:?}: {e}");
    if !reported {
        startup_log::no_window(&e.to_string());
    }
    Err(e)
}

/// The browser build starts from `web::start` (the page loads the module,
/// which runs `main`).
#[cfg(target_arch = "wasm32")]
fn main() {
    web::start();
}

struct Shell {
    app: Option<app::App>,
    /// The splash screen while TraceDraw starts (see splash.rs).
    #[cfg(not(target_arch = "wasm32"))]
    splash: Option<splash::Splash>,
    /// The file to open once the splash is done.
    #[cfg(not(target_arch = "wasm32"))]
    open: Option<std::path::PathBuf>,
    /// Files later starts handed over (see instance.rs).
    #[cfg(not(target_arch = "wasm32"))]
    handed_over: Option<std::sync::mpsc::Receiver<Vec<std::path::PathBuf>>>,
    /// Frames paced on a software adapter (#616).
    #[cfg(not(target_arch = "wasm32"))]
    pacer: gpu::Pacer,
    /// Frames the main window has had (up to the settling ones, see
    /// `splash::settle_window`).
    #[cfg(not(target_arch = "wasm32"))]
    window_frames: u32,
}

impl Shell {
    /// The editor at once (the browser build, or no splash screen).
    fn new(cc: &eframe::CreationContext<'_>, open: Option<std::path::PathBuf>) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        Shell {
            app: Some(app::App::new(&cc.egui_ctx, open)),
            #[cfg(not(target_arch = "wasm32"))]
            splash: None,
            #[cfg(not(target_arch = "wasm32"))]
            open: None,
            #[cfg(not(target_arch = "wasm32"))]
            handed_over: None,
            #[cfg(not(target_arch = "wasm32"))]
            pacer: gpu::Pacer::new(cc),
            #[cfg(not(target_arch = "wasm32"))]
            window_frames: 0,
        }
    }

    /// The splash screen first; the editor is built when it is done.
    #[cfg(not(target_arch = "wasm32"))]
    fn with_splash(cc: &eframe::CreationContext<'_>, open: Option<std::path::PathBuf>) -> Self {
        Shell {
            app: None,
            splash: Some(splash::Splash::start(&cc.egui_ctx, open.as_deref())),
            open,
            handed_over: None,
            pacer: gpu::Pacer::new(cc),
            window_frames: 0,
        }
    }
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        self.pacer.pace();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(splash) = self.splash.as_mut() {
            if splash.update(ui) == splash::Step::Wait {
                return;
            }
            let reset = splash.reset_requested;
            self.splash = None;
            let mut app = app::App::new(ui.ctx(), self.open.take());
            splash::open_main_window(ui.ctx(), &app.settings);
            // F8 held while TraceDraw started: ask before resetting.
            if reset {
                app.dialog = ui::dialogs::Dialog::ConfirmResetSettings;
            }
            self.app = Some(app);
            ui.ctx().request_repaint();
            return;
        }
        let Some(app) = self.app.as_mut() else {
            return;
        };
        // The first frame's window size, and frame statistics at debug
        // level (#616).
        #[cfg(not(target_arch = "wasm32"))]
        let began = frame_stats::begin(ui.ctx());
        // Fonts text asked for (browser build), and fonts that arrived.
        #[cfg(target_arch = "wasm32")]
        web::sync_ui_fonts(ui.ctx());
        #[cfg(target_arch = "wasm32")]
        web::fetch_requested_fonts();
        app.sync_fonts();
        app.sync_document_colors();
        // File dialogs that finished in the background (browser build).
        files::run_finished(app);
        // Files a later start handed over (one try_recv when none came).
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(rx) = &self.handed_over {
            if instance::open_handed_over(app, ui.ctx(), rx) {
                ui.ctx().request_repaint();
            }
        }
        // Requests of AI agents for the open drawing (MCP).
        #[cfg(not(target_arch = "wasm32"))]
        if app.sync_mcp_link(ui.ctx()) {
            ui.ctx().request_repaint();
        }
        // An answer from GitHub (File > GitHub).
        if app.poll_github() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        // A reply of the AI service (AI Drawing).
        if app.poll_ai() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        // Pictures from the image service (AI Generate).
        if app.poll_ai_image() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        // A Picture Mosaic library indexed on worker threads.
        if app.poll_picture_mosaic_index() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        ui::root(app, ui);
        // Dimensions moved or scaled this frame show their new values.
        app.bake_dimensions();
        // Bitmap effects on vector objects follow what changed them.
        app.sync_raster_fx();
        // Connectors follow the objects they join.
        app.refresh_connectors();
        // Callouts follow their text.
        app.refresh_callouts();
        // Blends follow their end objects.
        app.refresh_blends();
        // The window's state is kept once it has settled; until then a
        // maximize the window system dropped is asked for again (#787).
        #[cfg(not(target_arch = "wasm32"))]
        if self.window_frames < splash::SETTLE_FRAMES {
            splash::settle_window(ui.ctx(), &app.settings, self.window_frames);
            self.window_frames += 1;
        } else {
            splash::remember_window(ui.ctx(), &mut app.settings);
        }
        // Preferences changed this frame last without waiting for the
        // exit (the browser has none).
        app.autosave_settings(Some(ui.ctx()));
        // Bitmaps edited outside follow their files.
        app.watch_linked_bitmaps(Some(ui.ctx()), web_time::Instant::now());
        #[cfg(not(target_arch = "wasm32"))]
        frame_stats::end(ui.ctx(), began);
    }

    /// Keep the toolbox, dockers and other preferences for the next session.
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(app) = self.app.as_mut() {
            app.save_settings();
            app.end_session();
        }
    }
}
