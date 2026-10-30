//! The Android app (#893): the system loads this library into the app's
//! activity (`dev.tracedraw.app.TraceDrawActivity`, a `NativeActivity`
//! written in `packaging/android/java`) and the glue of `android-activity`
//! calls [`android_main`]. The editor opens full screen, drawn with wgpu,
//! with touch and stylus input through winit. Files go through the
//! system's document picker (Storage Access Framework) and the clipboard
//! through the system's clipboard service, both reached over JNI with the
//! activity's Java methods ([`documents`], [`clipboard`]); printing and
//! external programs are not available (docs/behavior/android.md).

use android_activity::AndroidApp;
use jni::refs::Reference as _;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// The tag of the app's lines in the system log (`adb logcat -s tracedraw`).
const LOG_TAG: &str = "tracedraw";

/// The activity, as the unowned JNI global reference `android-activity`
/// holds (valid while the app runs), for calls from the editor's thread.
static ACTIVITY: AtomicUsize = AtomicUsize::new(0);

/// The egui context, so that an answer arriving on the Java UI thread
/// wakes the editor's frame loop.
static EGUI: OnceLock<egui::Context> = OnceLock::new();

/// Where the app keeps its settings and log (its private folder) and
/// where files written without a picker go (its folder under the shared
/// storage): `settings::config_dir` and `files::save_path` read these.
fn prepare_folders(app: &AndroidApp) {
    if let Some(dir) = app.internal_data_path() {
        std::env::set_var("TRACEDRAW_CONFIG_DIR", &dir);
        log::info!("settings in {}", dir.display());
        // The temporary folder (auto-backups and the session marker of
        // the crash recovery, files handed to other steps): the system's
        // default, /data/local/tmp, is closed to apps.
        let tmp = dir.join("tmp");
        match std::fs::create_dir_all(&tmp) {
            Ok(()) => std::env::set_var("TMPDIR", &tmp),
            Err(e) => log::warn!("temporary folder {}: {e}", tmp.display()),
        }
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
    ACTIVITY.store(app.activity_as_ptr() as usize, Ordering::Release);
    documents::register_natives();
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
        wgpu_options: wgpu_options(),
        ..Default::default()
    };
    let result = eframe::run_native(
        "TraceDraw",
        options,
        Box::new(|cc| {
            let _ = EGUI.set(cc.egui_ctx.clone());
            Ok(Box::new(crate::Shell::new(cc, None)))
        }),
    );
    if let Err(e) = result {
        log::error!("TraceDraw could not open its window: {e}");
    }
}

/// The wgpu set-up of the window: OpenGL ES only, the API the manifest
/// requires (`android:glEsVersion` 3.0), so every device that installs
/// the app has it. One API, never two: a window surface made for Vulkan
/// holds the Android window, and OpenGL ES can then no longer draw into
/// it (`eglCreateWindowSurface` fails "already connected" and wgpu stops
/// the app on `Surface::configure`). With Vulkan and OpenGL ES both asked
/// for, the app closed at once wherever wgpu picked OpenGL ES, as on an
/// emulator.
fn wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{SurfaceConfig, WgpuConfiguration, WgpuSetup, WgpuSetupCreateNew};
    let mut setup = WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor.backends = crate::gpu::ANDROID_BACKENDS;
    WgpuConfiguration {
        surface: SurfaceConfig::LOW_LATENCY,
        wgpu_setup: WgpuSetup::CreateNew(setup),
        ..Default::default()
    }
}

/// Ask for a frame: something arrived from the Java side (a picked
/// document) while the editor waits for input.
fn request_repaint() {
    if let Some(ctx) = EGUI.get() {
        ctx.request_repaint();
    }
}

/// Run `f` with the current thread attached to the Java VM and the
/// activity object; `None` (logged) when there is no activity yet or the
/// call failed or threw.
fn with_activity<T>(
    what: &str,
    f: impl FnOnce(&mut jni::Env, &jni::objects::JObject) -> jni::errors::Result<T>,
) -> Option<T> {
    let raw = ACTIVITY.load(Ordering::Acquire);
    if raw == 0 {
        log::warn!("{what}: the activity is not running");
        return None;
    }
    let vm = match jni::JavaVM::singleton() {
        Ok(vm) => vm,
        Err(e) => {
            log::warn!("{what}: no Java VM: {e}");
            return None;
        }
    };
    let result = vm.attach_current_thread(|env| -> jni::errors::Result<T> {
        // SAFETY: the pointer is the global reference `android-activity`
        // holds for the activity, valid while the app runs; it is only
        // borrowed here (a `JObject` neither deletes nor owns it).
        let activity = unsafe { jni::objects::JObject::from_raw(env, raw as jni::sys::jobject) };
        f(env, &activity)
    });
    match result {
        Ok(v) => Some(v),
        Err(e) => {
            log::warn!("{what}: {e}");
            None
        }
    }
}

/// A Java string as a Rust one (`None` for null).
fn string_of(env: &jni::Env, s: &jni::objects::JString) -> Option<String> {
    if s.is_null() {
        return None;
    }
    s.mutf8_chars(env)
        .ok()
        .map(|chars| chars.to_str().into_owned())
}

/// Files through the system's document picker and content URIs.
pub mod documents {
    use super::*;
    use jni::objects::{JByteArray, JObject, JString};
    use jni::sys::jint;
    use jni::{jni_sig, jni_str, EnvUnowned, JValue};

    /// One document the picker returned.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Picked {
        /// The content URI, which [`read`] and [`write`] take.
        pub uri: String,
        /// The name the provider shows for it.
        pub name: String,
        /// Its MIME type when the provider knows one.
        pub mime: String,
    }

    /// What the picker answered for one request: the documents, or none
    /// when it was cancelled.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Answer {
        pub request: i32,
        pub picked: Vec<Picked>,
    }

    /// Answers that arrived from the Java side, taken by the frame loop.
    static ANSWERS: Mutex<Vec<Answer>> = Mutex::new(Vec::new());

    /// The next request code; the activity hands them back unchanged.
    static NEXT_REQUEST: AtomicI32 = AtomicI32::new(1000);

    /// A request code no other request has.
    pub fn next_request() -> i32 {
        NEXT_REQUEST.fetch_add(1, Ordering::Relaxed)
    }

    /// `TraceDrawActivity.nativeDocumentResult`: the picked documents (or
    /// null for a cancel), queued for the frame loop, which is woken.
    extern "system" fn native_document_result<'local>(
        mut env: EnvUnowned<'local>,
        _this: JObject<'local>,
        request: jint,
        uris: JString<'local>,
        names: JString<'local>,
        mimes: JString<'local>,
    ) {
        env.with_env(|env| -> jni::errors::Result<()> {
            let uris = string_of(env, &uris);
            let names = string_of(env, &names).unwrap_or_default();
            let mimes = string_of(env, &mimes).unwrap_or_default();
            let picked = match uris {
                Some(uris) => {
                    let names: Vec<&str> = names.split('\n').collect();
                    let mimes: Vec<&str> = mimes.split('\n').collect();
                    uris.split('\n')
                        .enumerate()
                        .filter(|(_, u)| !u.is_empty())
                        .map(|(i, uri)| Picked {
                            uri: uri.to_string(),
                            name: names.get(i).copied().unwrap_or("").to_string(),
                            mime: mimes.get(i).copied().unwrap_or("").to_string(),
                        })
                        .collect()
                }
                None => Vec::new(),
            };
            log::info!(
                "document picker: request {request}, {} picked",
                picked.len()
            );
            if let Ok(mut a) = ANSWERS.lock() {
                a.push(Answer { request, picked });
            }
            Ok(())
        })
        .resolve::<jni::errors::LogErrorAndDefault>();
        request_repaint();
    }

    /// Bind the activity's native methods to their Rust functions (the
    /// library is loaded by the system, not by `System.loadLibrary`
    /// alone, so the names are registered rather than looked up).
    pub fn register_natives() {
        let done = with_activity("register natives", |env, activity| {
            let class = env.get_object_class(activity)?;
            let methods = [
                // SAFETY: the function has the instance-method shape for
                // this signature: an unowned env, `this`, an int and three
                // strings, returning nothing.
                unsafe {
                    jni::NativeMethod::from_raw_parts(
                        jni_str!("nativeDocumentResult"),
                        jni_str!("(ILjava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"),
                        native_document_result as *mut std::ffi::c_void,
                    )
                },
            ];
            // SAFETY: as above; the class is the activity's own.
            unsafe { env.register_native_methods(&class, &methods) }
        });
        if done.is_none() {
            log::error!("the document picker will not answer: native methods not registered");
        }
    }

    /// Show the picker for a document (several when `multiple`); the
    /// answer arrives through [`take_answers`] with `request`.
    pub fn open(request: i32, multiple: bool) -> bool {
        with_activity("open document", |env, activity| {
            env.call_method(
                activity,
                jni_str!("openDocument"),
                jni_sig!("(IZ)V"),
                &[
                    JValue::Int(request),
                    JValue::Bool(multiple as jni::sys::jboolean),
                ],
            )?
            .v()
        })
        .is_some()
    }

    /// Show the picker for a place to save a new document called `name`
    /// of type `mime`; the answer arrives through [`take_answers`].
    pub fn create(request: i32, mime: &str, name: &str) -> bool {
        with_activity("create document", |env, activity| {
            let mime = env.new_string(mime)?;
            let name = env.new_string(name)?;
            env.call_method(
                activity,
                jni_str!("createDocument"),
                jni_sig!("(ILjava/lang/String;Ljava/lang/String;)V"),
                &[
                    JValue::Int(request),
                    JValue::Object(&mime),
                    JValue::Object(&name),
                ],
            )?
            .v()
        })
        .is_some()
    }

    /// The answers that arrived since the last call.
    pub fn take_answers() -> Vec<Answer> {
        ANSWERS
            .lock()
            .map(|mut a| std::mem::take(&mut *a))
            .unwrap_or_default()
    }

    /// The whole document at `uri`.
    pub fn read(uri: &str) -> std::io::Result<Vec<u8>> {
        let bytes = with_activity("read document", |env, activity| {
            let uri = env.new_string(uri)?;
            let result = env
                .call_method(
                    activity,
                    jni_str!("readDocument"),
                    jni_sig!("(Ljava/lang/String;)[B"),
                    &[JValue::Object(&uri)],
                )?
                .l()?;
            if result.is_null() {
                return Ok(None);
            }
            let array: JByteArray = env.cast_local::<JByteArray>(result)?;
            Ok(Some(env.convert_byte_array(&array)?))
        });
        match bytes {
            Some(Some(b)) => Ok(b),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("{uri} could not be read"),
            )),
        }
    }

    /// Replace the document at `uri` with `bytes`.
    pub fn write(uri: &str, bytes: &[u8]) -> std::io::Result<()> {
        let written = with_activity("write document", |env, activity| {
            let uri = env.new_string(uri)?;
            let data = env.byte_array_from_slice(bytes)?;
            env.call_method(
                activity,
                jni_str!("writeDocument"),
                jni_sig!("(Ljava/lang/String;[B)Z"),
                &[JValue::Object(&uri), JValue::Object(&data)],
            )?
            .z()
        });
        match written {
            Some(true) => Ok(()),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("{uri} could not be written"),
            )),
        }
    }
}

/// The system clipboard through the activity's clipboard service.
pub mod clipboard {
    use super::*;
    use jni::objects::JString;
    use jni::{jni_sig, jni_str, JValue};

    /// The clipboard's text, when it holds some and the app is in front.
    pub fn text() -> Option<String> {
        with_activity("read clipboard", |env, activity| {
            let result = env
                .call_method(
                    activity,
                    jni_str!("getClipboardText"),
                    jni_sig!("()Ljava/lang/String;"),
                    &[],
                )?
                .l()?;
            let s: JString = env.cast_local::<JString>(result)?;
            Ok(string_of(env, &s))
        })
        .flatten()
        .filter(|t| !t.is_empty())
    }

    /// Put `text` on the clipboard; whether it went there.
    pub fn set_text(text: &str) -> bool {
        with_activity("write clipboard", |env, activity| {
            let text = env.new_string(text)?;
            env.call_method(
                activity,
                jni_str!("setClipboardText"),
                jni_sig!("(Ljava/lang/String;)Z"),
                &[JValue::Object(&text)],
            )?
            .z()
        })
        .unwrap_or(false)
    }
}
