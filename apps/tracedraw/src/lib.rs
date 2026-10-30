#![allow(
    clippy::field_reassign_with_default,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::enum_variant_names,
    clippy::wrong_self_convention
)]
// The Android app starts from android.rs and does not use the desktop
// start-up (instance, gpu, startup_log, mcp ...), which is compiled all
// the same; the desktop and browser builds keep the warnings on.
#![cfg_attr(target_os = "android", allow(dead_code))]

//! TraceDraw app: a thin egui shell over the engine. The library holds
//! the whole app; `main.rs` is the desktop and browser program that calls
//! [`run`], and on Android the library is the app itself (`android.rs`,
//! loaded by the system as a shared library).

mod ai;
mod ai_image;
mod anchors;
#[cfg(target_os = "android")]
mod android;
#[cfg(test)]
mod android_package;
mod app;
mod autocorrect;
mod barcode;
mod bg_remove;
mod bitmap_fx;
mod bitmap_modes;
mod blend_link;
mod blend_tool;
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
mod fit_path;
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
#[cfg(test)]
mod input_method_tests;
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
mod object_thumbs;
mod ops;
mod ops2;
mod package;
mod painterly_tool;
mod palette;
mod pen;
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
mod symbol_paste;
mod symmetry_edit;
mod system_chars;
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
mod trace_preview;
mod ui;
mod updates;
mod variable_outline;
mod vector_mosaic;
mod view;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(any(target_arch = "wasm32", test))]
mod web_page;
// The pen pressure helper of the patched winit copy, with its tests
// (vendor/README.md), so that the gate runs them.
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../vendor/winit/src/pen_pressure.rs"]
mod winit_pen_pressure;
mod workspace_file;

/// The desktop program: the window, with the command line's files and
/// options (see `docs/behavior/about-and-startup.md`).
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub fn run() -> eframe::Result<()> {
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
    tracedraw_io::emf::install();
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
    // `tracedraw --x11`: the window opens through X11 (XWayland on a
    // Wayland desktop), where a graphics tablet's pen moves the pointer;
    // Wayland compositors send pens only over a protocol the windowing
    // layer does not speak yet (#850, see docs/behavior/pen-pressure.md).
    // Without an X server there is no window to open: say so and stop,
    // rather than let every renderer fail in turn.
    #[cfg(target_os = "linux")]
    let x11 = args.iter().any(|a| a == "--x11");
    #[cfg(target_os = "linux")]
    if x11 && std::env::var_os("DISPLAY").is_none_or(|d| d.is_empty()) {
        // The log writes to stderr as well as to its file.
        log::error!(
            "tracedraw --x11: DISPLAY is not set, so there is no X server to open the window \
             on. --x11 needs XWayland (on a Wayland desktop) or an X11 session; start \
             TraceDraw without --x11 to use Wayland."
        );
        std::process::exit(2);
    }
    // The system font list is read while the window and the graphics
    // driver start up, not after them: the editor needs it (the font
    // lists, the text in drawings) and used to wait for it alone.
    warm_up_fonts();
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
        splash::main_window_builder(&settings, plain)
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
    gpu::note_next(next);
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
        // Characters keyboard programs send that the windowing layer
        // would lose (#1151).
        #[cfg(windows)]
        event_loop_builder: Some(Box::new(system_chars::install)),
        // `--x11`: the window through X11, chosen from the windowing
        // layer, so the environment children inherit stays as it was.
        #[cfg(target_os = "linux")]
        event_loop_builder: x11.then(|| -> eframe::EventLoopBuilderHook {
            Box::new(|builder| {
                use winit::platform::x11::EventLoopBuilderExtX11 as _;
                builder.with_x11();
            })
        }),
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
                system_chars::set_context(&cc.egui_ctx);
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
        // The window opened but what the renderer drew came back nearly
        // black (#616): the next renderer, in a new start.
        if let (true, Some(next)) = (gpu::blind(), next) {
            log::warn!(
                "the window opened with {mode:?} but its frames came back nearly black; trying {next:?}"
            );
            if !gpu::restart_with(next, &args) {
                let what = format!(
                    "the window opened with {mode:?} but what it drew came back nearly black, and TraceDraw could not start again with {next:?}"
                );
                log::error!("{what}");
                startup_log::no_window(&what);
            }
        }
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

/// Read the system fonts on a worker thread, so that this takes place
/// while the window opens. Whoever needs the fonts first waits for the
/// same read (`tracedraw_text::fonts` starts once), so nothing is read
/// twice and a failed thread costs nothing but the overlap.
#[cfg(not(target_arch = "wasm32"))]
fn warm_up_fonts() {
    if let Err(e) = std::thread::Builder::new()
        .name("tracedraw-font-scan".into())
        .spawn(|| {
            tracedraw_text::fonts();
        })
    {
        log::warn!("font scan thread: {e}");
    }
}

/// The browser build starts from `web::start` (the page loads the module,
/// which runs the program's `main`, which calls this).
#[cfg(target_arch = "wasm32")]
pub fn run() {
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
    /// The main window settling after it opens (see
    /// `splash::WindowSettle`).
    #[cfg(not(target_arch = "wasm32"))]
    window_settle: splash::WindowSettle,
    /// Where the check of the OpenGL renderer stands (see
    /// `check_renderer`).
    #[cfg(not(target_arch = "wasm32"))]
    renderer_check: RendererCheck,
    /// How many read-backs came back nearly black with the window in
    /// front.
    #[cfg(not(target_arch = "wasm32"))]
    blind_reads: u8,
    /// The main-window frames the check has seen (`window_frames` stops
    /// counting at `splash::SETTLE_FRAMES`).
    #[cfg(not(target_arch = "wasm32"))]
    check_frames: u32,
    /// The interface fonts still being read; the window draws with the
    /// bundled ones until they are in.
    #[cfg(not(target_arch = "wasm32"))]
    fonts: theme::FontLoad,
}

/// Where the check of the OpenGL renderer stands (`Shell::check_renderer`).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RendererCheck {
    /// A read-back is asked for at this main-window frame.
    At(u32),
    /// Asked for: the probe is drawn until the picture arrives.
    Pending,
    /// Over: the window is drawn, or blind and closing, or nothing is
    /// left to try.
    Done,
}

impl Shell {
    /// The editor at once (the browser build, or no splash screen).
    fn new(cc: &eframe::CreationContext<'_>, open: Option<std::path::PathBuf>) -> Self {
        #[cfg(target_arch = "wasm32")]
        theme::install_fonts(&cc.egui_ctx);
        #[cfg(not(target_arch = "wasm32"))]
        let fonts = theme::install_fonts_in_background(&cc.egui_ctx);
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
            window_settle: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            renderer_check: RendererCheck::At(gpu::CHECKED_FRAME),
            #[cfg(not(target_arch = "wasm32"))]
            blind_reads: 0,
            #[cfg(not(target_arch = "wasm32"))]
            check_frames: 0,
            #[cfg(not(target_arch = "wasm32"))]
            fonts,
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
            window_settle: Default::default(),
            renderer_check: RendererCheck::At(gpu::CHECKED_FRAME),
            blind_reads: 0,
            check_frames: 0,
            fonts: theme::FontLoad::default(),
        }
    }

    /// The OpenGL renderer is checked once it draws: a frame of the main
    /// window is read back and looked at, with a probe drawn last in it
    /// (`gpu::renderer_blind`). A driver that takes every call and shows
    /// nothing (#616: the first meshes of a frame right, the rest black)
    /// loses the probe and leaves the frame nearly black; after
    /// `gpu::BLIND_READS` such read-backs with the window in front the
    /// renderer is noted as blind and the window closes, so the start
    /// goes on with the next renderer. The first read-back is asked for
    /// at frame `gpu::CHECKED_FRAME` and is of the frame after it; a
    /// read-back of a window another covers does not count, and when no
    /// renderer is left to try the window stays.
    #[cfg(not(target_arch = "wasm32"))]
    fn check_renderer(&mut self, ctx: &egui::Context) {
        if gpu::mode_in_use() != Some(gpu::Mode::Gl) {
            return;
        }
        let frame = self.check_frames;
        self.check_frames += 1;
        match self.renderer_check {
            RendererCheck::Done => return,
            RendererCheck::At(at) => {
                if frame >= at {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
                        RENDERER_CHECK,
                    )));
                    ctx.request_repaint();
                    self.renderer_check = RendererCheck::Pending;
                    paint_probe(ctx);
                } else if ctx.input(|i| i.viewport().focused) != Some(false) {
                    // The frames up to it keep coming while the window is
                    // in front; a window behind another waits for an event.
                    ctx.request_repaint();
                }
                return;
            }
            RendererCheck::Pending => {}
        }
        let probe = paint_probe(ctx);
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot {
                    user_data, image, ..
                } if user_data
                    .data
                    .as_ref()
                    .and_then(|d| d.downcast_ref::<&str>())
                    .is_some_and(|tag| *tag == RENDERER_CHECK) =>
                {
                    Some(image.clone())
                }
                _ => None,
            })
        });
        let Some(image) = image else {
            return;
        };
        let [w, h] = image.size;
        if !gpu::renderer_blind(&image, probe) {
            self.renderer_check = RendererCheck::Done;
            log::info!("the window ({w} x {h} px) read back drawn");
            return;
        }
        if ctx.input(|i| i.viewport().focused) == Some(false) {
            log::warn!(
                "the window ({w} x {h} px) came back nearly black from the renderer while another window is in front; checking again"
            );
        } else {
            self.blind_reads += 1;
            log::error!(
                "the window ({w} x {h} px) came back nearly black from the renderer: it draws nothing that can be seen ({} of {})",
                self.blind_reads,
                gpu::BLIND_READS
            );
        }
        if self.blind_reads < gpu::BLIND_READS {
            self.renderer_check = RendererCheck::At(frame + gpu::RECHECK_FRAMES);
            ctx.request_repaint();
            return;
        }
        self.renderer_check = RendererCheck::Done;
        if gpu::next_renderer().is_none() {
            log::error!("no other renderer is left to try; the window stays");
            return;
        }
        gpu::note_blind();
        // The window closes whatever the documents hold: nothing of them
        // can be seen, and the start goes on with them.
        if let Some(app) = self.app.as_mut() {
            app.quit_now = true;
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

/// Draw the probe of `Shell::check_renderer`, a light square
/// `gpu::PROBE_SIDE` points wide near the window's bottom right corner,
/// on the topmost layer, so that it is the last mesh of the frame; the
/// pixel its centre lands on is returned.
#[cfg(not(target_arch = "wasm32"))]
fn paint_probe(ctx: &egui::Context) -> [usize; 2] {
    let window = ctx.content_rect();
    let side = gpu::PROBE_SIDE;
    let probe = egui::Rect::from_min_size(
        egui::pos2(window.max.x - 2.0 * side, window.max.y - 2.0 * side),
        egui::vec2(side, side),
    );
    ctx.debug_painter()
        .rect_filled(probe, 0.0, egui::Color32::from_gray(220));
    let pixels_per_point = ctx.pixels_per_point();
    let px = |points: f32| (points * pixels_per_point).round().max(0.0) as usize;
    [px(probe.center().x), px(probe.center().y)]
}

/// The tag of the frame read back by `Shell::check_renderer`.
#[cfg(not(target_arch = "wasm32"))]
const RENDERER_CHECK: &str = "renderer-check";

impl eframe::App for Shell {
    /// Characters the windowing layer lost (input methods on Windows,
    /// #1151) join the frame's input as typed text. A graphics tablet's
    /// pressure is read from the touch events, and the pen's touches that
    /// only move are taken out, so egui never takes the pen for a touch
    /// screen (#850, see `pen.rs`).
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        system_chars::take_into(raw_input);
        pen::filter_raw_input(ctx, raw_input);
        interaction::release_chorded_left(ctx, raw_input);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(not(target_arch = "wasm32"))]
        self.pacer.pace();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(splash) = self.splash.as_mut() {
            if splash.update(ui) == splash::Step::Wait {
                return;
            }
            let reset = splash.reset_requested;
            // Fonts not read yet are installed once they are.
            self.fonts = splash.take_fonts();
            self.splash = None;
            let built = web_time::Instant::now();
            let mut app = app::App::new(ui.ctx(), self.open.take());
            log::info!(
                "the editor was built in {:.0} ms",
                built.elapsed().as_secs_f64() * 1000.0
            );
            splash::open_main_window(ui.ctx(), &app.settings);
            // F8 held while TraceDraw started: ask before resetting.
            if reset {
                app.dialog = ui::dialogs::Dialog::ConfirmResetSettings;
            }
            self.app = Some(app);
            ui.ctx().request_repaint();
            return;
        }
        if self.app.is_none() {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        self.check_renderer(ui.ctx());
        let Some(app) = self.app.as_mut() else {
            return;
        };
        // The first frame's window size, and frame statistics at debug
        // level (#616).
        #[cfg(not(target_arch = "wasm32"))]
        let began = frame_stats::begin(ui.ctx());
        // The system fonts of the interface, read while the editor came up.
        #[cfg(not(target_arch = "wasm32"))]
        if self.fonts.poll(ui.ctx()) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
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
        // The window's state is kept once it has settled; until then it
        // is maximized when it should start so, and checked (#787).
        #[cfg(not(target_arch = "wasm32"))]
        if self.window_settle.settling() {
            self.window_settle.step(ui.ctx(), &app.settings);
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
