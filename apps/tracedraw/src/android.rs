//! The Android app (#893): the system loads this library into its
//! NativeActivity and the glue of `android-activity` calls
//! [`android_main`]. Phase 1: the editor opens full screen, drawn with
//! wgpu, with touch and stylus input through winit; file dialogs, the
//! system clipboard, printing and external programs are not available
//! yet (`files.rs`, `clipboard.rs`, docs/behavior/android.md).

use android_activity::AndroidApp;

/// The tag of the app's lines in the system log (`adb logcat -s tracedraw`).
const LOG_TAG: &str = "tracedraw";

/// Where the app keeps its settings and log (its private folder) and
/// where saved drawings and exports go (its folder under the shared
/// storage, which file managers show): `settings::config_dir` and
/// `files::save_path` read these.
fn prepare_folders(app: &AndroidApp) {
    if let Some(dir) = app.internal_data_path() {
        std::env::set_var("TRACEDRAW_CONFIG_DIR", &dir);
        log::info!("settings in {}", dir.display());
    }
    if let Some(dir) = app.external_data_path() {
        std::env::set_var("TRACEDRAW_DOCUMENTS_DIR", &dir);
        log::info!("documents in {}", dir.display());
    }
}

/// Panics go to the system log, where `adb logcat` shows them: the app
/// has no console and no error box.
fn log_panics() {
    std::panic::set_hook(Box::new(|info| {
        log::error!("panic: {info}");
    }));
}

/// The entry point the glue calls on the app's own thread, with the
/// activity's window and input queue behind `app`.
#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag(LOG_TAG),
    );
    log_panics();
    prepare_folders(&app);
    tracedraw_text::install();
    tracedraw_io::emf::install();
    // A fresh install starts in the Touch workspace (larger bars and
    // buttons, no rulers or panels, two fingers pan and a pinch zooms);
    // another one picked under Window > Workspace is kept.
    let first_start = crate::settings::Settings::path().is_some_and(|p| !p.exists());
    let mut settings = crate::settings::Settings::load();
    if first_start {
        settings.workspace = crate::app::Workspace::Touch.id().into();
        settings.save();
    }
    crate::i18n::set_language(&if settings.language.is_empty() {
        crate::i18n::system_language().to_string()
    } else {
        settings.language.clone()
    });
    let options = eframe::NativeOptions {
        android_app: Some(app),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    let result = eframe::run_native(
        "TraceDraw",
        options,
        Box::new(|cc| Ok(Box::new(crate::Shell::new(cc, None)))),
    );
    if let Err(e) = result {
        log::error!("TraceDraw could not open its window: {e}");
    }
}
