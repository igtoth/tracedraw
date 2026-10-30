//! The browser build: start-up on a canvas, downloads, local storage and
//! the bundled fonts. Compiled only for `wasm32`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use wasm_bindgen::JsCast;

thread_local! {
    /// The egui context, so background tasks (file pickers) can ask for a
    /// frame when they finish.
    static CTX: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
    /// The files of each family in the page's font folder (see
    /// `font_folder.rs`).
    static FONT_FILES: RefCell<BTreeMap<String, Vec<String>>> =
        const { RefCell::new(BTreeMap::new()) };
    /// The font catalog: every other family of the google/fonts collection
    /// (see `font_folder.rs`).
    static CATALOG: RefCell<crate::font_folder::Catalog> = RefCell::new(Default::default());
    /// The interface language and font generation last looked at, and
    /// the script font last installed into the interface fonts.
    static UI_FONT: RefCell<(String, u64, Option<&'static str>)> =
        const { RefCell::new((String::new(), u64::MAX, None)) };
}

// Fonts found at build time (see build.rs): Liberation, DejaVu and Noto
// faces when the build machine has them.
include!(concat!(env!("OUT_DIR"), "/web_fonts.rs"));

/// Remember the context for [`request_repaint`].
pub fn set_context(ctx: &egui::Context) {
    CTX.with(|c| *c.borrow_mut() = Some(ctx.clone()));
}

/// Ask for a new frame from outside the frame loop.
pub fn request_repaint() {
    CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.request_repaint();
        }
    });
}

/// Open a link in a new browser tab.
pub fn open_url(url: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.open_with_url_and_target(url, "_blank");
    }
}

/// The page title.
pub fn set_title(title: &str) {
    if let Some(d) = web_sys::window().and_then(|w| w.document()) {
        d.set_title(title);
    }
}

/// Put text on the clipboard through egui (it uses the browser's API).
pub fn copy_text(text: &str) {
    CTX.with(|c| {
        if let Some(ctx) = c.borrow().as_ref() {
            ctx.copy_text(text.to_string());
        }
    });
}

/// Offer bytes as a download named `name`.
pub fn download(name: &str, bytes: &[u8]) -> Result<(), String> {
    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array.buffer());
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime_for(name));
    let blob = web_sys::Blob::new_with_buffer_source_sequence_and_options(&parts, &options)
        .map_err(|e| format!("{e:?}"))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(|e| format!("{e:?}"))?;
    let anchor: web_sys::HtmlAnchorElement = document
        .create_element("a")
        .map_err(|e| format!("{e:?}"))?
        .dyn_into()
        .map_err(|_| "not an anchor".to_string())?;
    anchor.set_href(&url);
    anchor.set_download(name);
    anchor.click();
    let _ = web_sys::Url::revoke_object_url(&url);
    Ok(())
}

fn mime_for(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "svg" => "image/svg+xml",
        "pdf" | "ai" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "html" => "text/html",
        "tdraw" | "json" => "application/json",
        "txt" | "js" | "csv" => "text/plain",
        _ => "application/octet-stream",
    }
}

/// Whether the page runs on an iPhone, iPad or Android device, whose
/// file picker filters by file type (see
/// [`crate::files::picker_filters_by_type`]), from the navigator's user
/// agent, platform and touch points.
pub fn picker_filters_by_type() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let get = |o: &wasm_bindgen::JsValue, key: &str| {
        js_sys::Reflect::get(o, &wasm_bindgen::JsValue::from_str(key)).ok()
    };
    let Some(navigator) = get(&window.into(), "navigator") else {
        return false;
    };
    let text = |key: &str| {
        get(&navigator, key)
            .and_then(|v| v.as_string())
            .unwrap_or_default()
    };
    let touch = get(&navigator, "maxTouchPoints")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    crate::files::picker_filters_by_type(&text("userAgent"), &text("platform"), touch as u32)
}

/// A value from the browser's local storage.
pub fn storage_get(key: &str) -> Option<String> {
    web_sys::window()?
        .local_storage()
        .ok()??
        .get_item(key)
        .ok()?
}

/// Store a value in the browser's local storage (best effort).
pub fn storage_set(key: &str, value: &str) {
    if let Some(Ok(Some(s))) = web_sys::window().map(|w| w.local_storage()) {
        let _ = s.set_item(key, value);
    }
}

/// The bytes at `url` (relative to the page), or `None` when the request
/// fails or the server answers with an error.
async fn fetch_bytes(url: &str) -> Option<Vec<u8>> {
    let window = web_sys::window()?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_str(url))
        .await
        .ok()?;
    let response: web_sys::Response = response.dyn_into().ok()?;
    if !response.ok() {
        return None;
    }
    let buffer = wasm_bindgen_futures::JsFuture::from(response.array_buffer().ok()?)
        .await
        .ok()?;
    Some(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// Read the page's font folder list (`fonts/fonts.json`) and font catalog
/// (`fonts/catalog.json`) and offer their families to the text engine,
/// which asks for them as text (or a font list) needs them; a family both
/// have comes from the folder.
fn load_font_list() {
    wasm_bindgen_futures::spawn_local(async {
        let list = match fetch_bytes("fonts/fonts.json").await {
            Some(bytes) => crate::font_folder::parse_list(&bytes),
            None => {
                log::info!("no font folder next to the page");
                Default::default()
            }
        };
        let catalog = match fetch_bytes("fonts/catalog.json").await {
            Some(bytes) => crate::font_folder::parse_catalog(&bytes),
            None => {
                log::info!("no font catalog next to the page");
                Default::default()
            }
        };
        let mut families: Vec<String> = list.keys().cloned().collect();
        families.extend(catalog.families.keys().cloned());
        families.sort();
        families.dedup();
        log::info!(
            "fonts to fetch: {} in the folder, {} in the catalog",
            list.len(),
            catalog.families.len()
        );
        FONT_FILES.with(|f| *f.borrow_mut() = list);
        CATALOG.with(|c| *c.borrow_mut() = catalog);
        // The font lists take them in with the next frame (the fonts'
        // generation changes).
        tracedraw_text::fonts().offer_families(&families);
        request_repaint();
    });
}

/// Fetch the files of the families text asked for since the last frame;
/// the text engine takes them in as they arrive and the page draws again.
pub fn fetch_requested_fonts() {
    for family in tracedraw_text::fonts().take_font_requests() {
        // The folder's files, else the catalog's.
        let mut urls: Vec<String> = FONT_FILES.with(|f| {
            f.borrow()
                .get(&family)
                .map(|files| files.iter().map(|file| format!("fonts/{file}")).collect())
                .unwrap_or_default()
        });
        if urls.is_empty() {
            urls = CATALOG.with(|c| c.borrow().urls(&family));
        }
        wasm_bindgen_futures::spawn_local(async move {
            let mut data = Vec::new();
            for url in &urls {
                match fetch_bytes(url).await {
                    Some(bytes) => data.push(bytes),
                    None => log::warn!("could not fetch the font file {url}"),
                }
            }
            if tracedraw_text::fonts().add_fonts(data).is_empty() {
                log::warn!("the font folder gave nothing usable for {family}");
            }
            request_repaint();
        });
    }
}

/// The interface in Chinese or Japanese needs a font this build does not
/// bundle: ask the font folder for it, and install the interface fonts
/// again once it has arrived; the menus showed empty boxes (#561). Looks
/// again only when the language or the fonts change.
pub fn sync_ui_fonts(ctx: &egui::Context) {
    let fonts = tracedraw_text::fonts();
    let language = crate::i18n::language();
    let generation = fonts.generation();
    let install = UI_FONT.with(|u| {
        let mut u = u.borrow_mut();
        if u.0 == language && u.1 == generation {
            return false;
        }
        u.0 = language.clone();
        u.1 = generation;
        let Some(family) = crate::theme::ui_script_family(&language) else {
            return false;
        };
        if !fonts.has_family(family) {
            fonts.request_family(family);
            return false;
        }
        u.2.replace(family) != Some(family)
    });
    if install {
        crate::theme::install_fonts(ctx);
        request_repaint();
    }
}

/// Register the bundled fonts with the text engine (before first use).
pub fn register_fonts() {
    for (_, data) in WEB_FONTS {
        tracedraw_text::add_font_data(data.to_vec());
    }
    if WEB_FONTS.is_empty() {
        log::warn!("no fonts were bundled into this build; text objects cannot be drawn");
    }
}

/// Keep the editor's shortcuts away from the browser: Ctrl/Cmd plus a
/// letter (export, import, group, duplicate, repeat...) and the function
/// keys would otherwise open browser features (Ctrl+R and F5 reload the
/// page). Copy, cut, paste (egui needs those events) and the developer
/// tools stay with the browser (`web_page::browser_keeps_key`).
fn guard_shortcuts() {
    use wasm_bindgen::closure::Closure;
    let Some(window) = web_sys::window() else {
        return;
    };
    let guard = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(|e: web_sys::KeyboardEvent| {
        let key = e.key();
        let function_key =
            key.len() >= 2 && key.starts_with('F') && key[1..].chars().all(|c| c.is_ascii_digit());
        if function_key {
            e.prevent_default();
            return;
        }
        if !(e.ctrl_key() || e.meta_key()) {
            return;
        }
        if !crate::web_page::browser_keeps_key(&key, e.shift_key()) {
            e.prevent_default();
        }
    });
    let _ = window.add_event_listener_with_callback_and_bool(
        "keydown",
        guard.as_ref().unchecked_ref(),
        true,
    );
    guard.forget();
}

/// A right button pressed while the left one is held: the browser gives
/// it as a pointer move, not a press, so egui never sees it; the mouse
/// event (which the browser does send for every button) is noted for the
/// drag (`interaction.rs`).
fn watch_chorded_right_press() {
    use wasm_bindgen::closure::Closure;
    let Some(window) = web_sys::window() else {
        return;
    };
    let watch = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| {
        if crate::interaction::is_chorded_right_press(e.button(), e.buttons()) {
            crate::interaction::note_chorded_right_press();
            request_repaint();
        }
    });
    let _ = window.add_event_listener_with_callback_and_bool(
        "mousedown",
        watch.as_ref().unchecked_ref(),
        true,
    );
    watch.forget();
    // The left button let go while the right one is held: the browser
    // sends no pointer release until the last button goes up, so egui
    // would keep the left button held (#1334).
    let up = Closure::<dyn FnMut(web_sys::MouseEvent)>::new(|e: web_sys::MouseEvent| {
        if crate::interaction::is_chorded_left_release(e.button(), e.buttons()) {
            crate::interaction::note_chorded_left_release();
            request_repaint();
        }
    });
    let _ = window.add_event_listener_with_callback_and_bool(
        "mouseup",
        up.as_ref().unchecked_ref(),
        true,
    );
    up.forget();
}

/// Start the editor on the page's `<canvas id="tracedraw">`.
pub fn start() {
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    backups::install();
    guard_shortcuts();
    watch_chorded_right_press();
    register_fonts();
    tracedraw_text::install();
    tracedraw_io::emf::install();
    load_font_list();
    wasm_bindgen_futures::spawn_local(async {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Some(canvas) = document
            .get_element_by_id("tracedraw")
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        else {
            log::error!("no <canvas id=\"tracedraw\"> on the page");
            return;
        };
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| {
                    set_context(&cc.egui_ctx);
                    Ok(Box::new(crate::Shell::new(cc, None)))
                }),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match &result {
                Ok(()) => loading.remove(),
                Err(e) => {
                    // The splash says so and the reason shows under it,
                    // keeping the page's links to the downloads.
                    let text = format!(
                        "TraceDraw could not start: {e:?}. A browser with WebGL 2 is needed."
                    );
                    if let Some(status) = document.get_element_by_id("loading-status") {
                        status.set_inner_html("TraceDraw could not start");
                    }
                    if let Some(bar) = document.get_element_by_id("loading-bar") {
                        bar.set_attribute("fill", "#e0533d").ok();
                    }
                    match document.get_element_by_id("loading-error") {
                        Some(error) => {
                            error.set_inner_html(&escape_html(&text));
                            error.remove_attribute("hidden").ok();
                        }
                        None => loading.set_inner_html(&escape_html(&text)),
                    }
                }
            }
        }
    });
}

fn escape_html(text: &str) -> String {
    crate::web_page::escape_html(text)
}

/// Unsaved work in the browser (#1395): leaving the page with unsaved
/// drawings asks first, and auto-backups with the session markers of the
/// crash recovery (`recovery.rs`) are kept in the page's storage, so the
/// next visit offers what a closed, reloaded or killed tab left.
///
/// - The markers are small: they go to local storage, which is written at
///   once, so the page can still mark its session gone while it unloads.
/// - The backups can be large: they go to an IndexedDB database
///   (`tracedraw`, store `backups`), keyed by their path
///   (`/backup/<session id>/AutoBackup_of_<name>.tdraw`). The database is
///   read once when the page opens; the session's recovery waits for it
///   ([`ready`]). Backups no marker names any more are deleted then.
pub mod backups {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::JsFuture;

    const DATABASE_NAME: &str = "tracedraw";
    const STORE: &str = "backups";
    /// Local storage keys of the markers: this, then the marker's path.
    const MARKER_KEY: &str = "tracedraw.recovery:";
    /// How often a session checks in from its timer, in milliseconds:
    /// a hidden tab draws no frames, and browsers run its timers at
    /// least once a minute, well within `recovery::STALE_SECS`.
    const KEEP_ALIVE_MS: i32 = 30_000;

    thread_local! {
        static READY: Cell<bool> = const { Cell::new(false) };
        static DATABASE: RefCell<Option<web_sys::IdbDatabase>> = const { RefCell::new(None) };
        /// The backups read when the page opened, by path.
        static LOADED: RefCell<HashMap<PathBuf, Vec<u8>>> = RefCell::new(HashMap::new());
        /// This session's marker, once it has one.
        static MARKER: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
        /// Drawings with unsaved changes are open.
        static UNSAVED: Cell<bool> = const { Cell::new(false) };
        /// The page is about to be left (`beforeunload`): back up now,
        /// while the browser asks.
        static UNLOADING: Cell<bool> = const { Cell::new(false) };
        /// The page was left for good (`pagehide`, not into the
        /// back-forward cache).
        static LEFT: Cell<bool> = const { Cell::new(false) };
    }

    /// True once the database has been read (or could not be opened):
    /// the session's recovery can look for what earlier ones left.
    pub fn ready() -> bool {
        READY.with(Cell::get)
    }

    /// Tell the page whether drawings with unsaved changes are open
    /// (every frame), so leaving it asks first.
    pub fn set_unsaved(unsaved: bool) {
        UNSAVED.with(|u| u.set(unsaved));
    }

    /// This session's marker: its timer keeps it checked in and leaving
    /// the page marks it gone.
    pub fn watch_session(marker: PathBuf) {
        MARKER.with(|m| *m.borrow_mut() = Some(marker));
    }

    /// True while the page is not shown (another tab, a minimized window,
    /// a phone's home screen, a tab being closed).
    pub fn page_hidden() -> bool {
        web_sys::window()
            .and_then(|w| w.document())
            .is_some_and(|d| d.hidden())
    }

    /// True once after the page was about to be left (`beforeunload`).
    pub fn take_unloading() -> bool {
        UNLOADING.with(|u| u.replace(false))
    }

    /// True once the page was left for good (`pagehide`): the session
    /// ends there.
    pub fn left() -> bool {
        LEFT.with(Cell::get)
    }

    fn is_marker(path: &Path) -> bool {
        path.parent() == Some(Path::new(crate::files::WEB_BACKUP_DIR))
            && path.extension().is_some_and(|e| e == "json")
    }

    fn marker_key(path: &Path) -> String {
        format!("{MARKER_KEY}{}", path.display())
    }

    fn local() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    /// The paths of the markers in local storage.
    fn markers() -> Vec<PathBuf> {
        let Some(storage) = local() else {
            return Vec::new();
        };
        let n = storage.length().unwrap_or(0);
        (0..n)
            .filter_map(|i| storage.key(i).ok().flatten())
            .filter_map(|k| k.strip_prefix(MARKER_KEY).map(PathBuf::from))
            .filter(|p| is_marker(p))
            .collect()
    }

    pub fn read(path: &Path) -> Option<Vec<u8>> {
        if is_marker(path) {
            return local()?
                .get_item(&marker_key(path))
                .ok()?
                .map(String::into_bytes);
        }
        LOADED.with(|l| l.borrow().get(path).cloned())
    }

    pub fn exists(path: &Path) -> bool {
        if is_marker(path) {
            return read(path).is_some();
        }
        LOADED.with(|l| l.borrow().contains_key(path))
    }

    /// The markers (directly in the folder) or the backups read when the
    /// page opened (in a session's folder).
    pub fn list(dir: &Path) -> Vec<PathBuf> {
        if dir == Path::new(crate::files::WEB_BACKUP_DIR) {
            return markers();
        }
        LOADED.with(|l| {
            l.borrow()
                .keys()
                .filter(|p| p.parent() == Some(dir))
                .cloned()
                .collect()
        })
    }

    pub fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let error = |e: JsValue| std::io::Error::other(format!("{e:?}"));
        if is_marker(path) {
            let text = std::str::from_utf8(bytes)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let storage = local().ok_or_else(|| std::io::Error::other("no local storage"))?;
            return storage.set_item(&marker_key(path), text).map_err(error);
        }
        let key = JsValue::from_str(&path.display().to_string());
        let value = js_sys::Uint8Array::from(bytes);
        with_store(&path.display().to_string(), |store| {
            store.put_with_key(&value, &key).map(|_| ())
        })
        .map_err(error)
    }

    pub fn remove(path: &Path) {
        if is_marker(path) {
            if let Some(storage) = local() {
                let _ = storage.remove_item(&marker_key(path));
            }
            return;
        }
        LOADED.with(|l| l.borrow_mut().remove(path));
        let key = JsValue::from_str(&path.display().to_string());
        if let Err(e) = with_store(&path.display().to_string(), |store| {
            store.delete(&key).map(|_| ())
        }) {
            log::warn!("removing the backup {}: {e:?}", path.display());
        }
    }

    /// Forget the backups read when the page opened except `keep` (the
    /// ones offered to recover): the others belong to tabs still open.
    pub fn release_except(keep: &[PathBuf]) {
        LOADED.with(|l| l.borrow_mut().retain(|p, _| keep.contains(p)));
    }

    /// Run `f` on the store in a read-write transaction, committed at
    /// once (so it is not lost when the page closes right after); a
    /// failure that only shows later is logged.
    fn with_store(
        what: &str,
        f: impl FnOnce(&web_sys::IdbObjectStore) -> Result<(), JsValue>,
    ) -> Result<(), JsValue> {
        DATABASE.with(|d| {
            let d = d.borrow();
            let db = d
                .as_ref()
                .ok_or_else(|| JsValue::from_str("the backup database is not open"))?;
            let tx =
                db.transaction_with_str_and_mode(STORE, web_sys::IdbTransactionMode::Readwrite)?;
            let what = what.to_string();
            let failed = Closure::once_into_js(move |_: web_sys::Event| {
                log::warn!("the browser did not store {what} (storage full?)");
            });
            tx.set_onabort(Some(failed.unchecked_ref()));
            f(&tx.object_store(STORE)?)?;
            // Marked deprecated in web-sys's bindings only; a browser
            // without it (Safari before 15) commits when the task ends.
            #[allow(deprecated)]
            let _ = tx.commit();
            Ok(())
        })
    }

    /// A promise settled by a request's success (with its result) or
    /// error.
    fn settled(request: &web_sys::IdbRequest) -> js_sys::Promise {
        let request = request.clone();
        js_sys::Promise::new(&mut |resolve, reject| {
            let done = request.clone();
            let ok = Closure::once_into_js(move |_: web_sys::Event| {
                let _ = resolve.call1(&JsValue::NULL, &done.result().unwrap_or_default());
            });
            let failed = Closure::once_into_js(move |_: web_sys::Event| {
                let _ = reject.call0(&JsValue::NULL);
            });
            request.set_onsuccess(Some(ok.unchecked_ref()));
            request.set_onerror(Some(failed.unchecked_ref()));
        })
    }

    async fn open() -> Result<web_sys::IdbDatabase, JsValue> {
        let factory = web_sys::window()
            .ok_or_else(|| JsValue::from_str("no window"))?
            .indexed_db()?
            .ok_or_else(|| JsValue::from_str("no IndexedDB"))?;
        let request = factory.open_with_u32(DATABASE_NAME, 1)?;
        let made = request.clone();
        let upgrade = Closure::once_into_js(move |_: web_sys::Event| {
            if let Ok(db) = made
                .result()
                .and_then(|r| r.dyn_into::<web_sys::IdbDatabase>())
            {
                let _ = db.create_object_store(STORE);
            }
        });
        request.set_onupgradeneeded(Some(upgrade.unchecked_ref()));
        JsFuture::from(settled(&request)).await?.dyn_into()
    }

    /// Every backup in the database, by path.
    async fn read_all(db: &web_sys::IdbDatabase) -> Result<Vec<(PathBuf, Vec<u8>)>, JsValue> {
        let tx = db.transaction_with_str(STORE)?;
        let store = tx.object_store(STORE)?;
        // Both asked before waiting: the transaction ends once nothing
        // is pending. Both come in key order.
        let keys = settled(&store.get_all_keys()?);
        let values = settled(&store.get_all()?);
        let keys: js_sys::Array = JsFuture::from(keys).await?.dyn_into()?;
        let values: js_sys::Array = JsFuture::from(values).await?.dyn_into()?;
        Ok(keys
            .iter()
            .zip(values.iter())
            .filter_map(|(k, v)| {
                let bytes = v.dyn_into::<js_sys::Uint8Array>().ok()?.to_vec();
                Some((PathBuf::from(k.as_string()?), bytes))
            })
            .collect())
    }

    /// The session a backup belongs to: the folder it is in.
    fn session_of(path: &Path) -> Option<String> {
        let rest = path.strip_prefix(crate::files::WEB_BACKUP_DIR).ok()?;
        let first = rest.components().next()?;
        Some(first.as_os_str().to_string_lossy().to_string())
    }

    /// Read the database, dropping the backups of sessions without a
    /// marker, then let the recovery look.
    fn load() {
        wasm_bindgen_futures::spawn_local(async {
            match open().await {
                Ok(db) => {
                    let all = read_all(&db).await.unwrap_or_else(|e| {
                        log::warn!("reading the backups: {e:?}");
                        Vec::new()
                    });
                    DATABASE.with(|d| *d.borrow_mut() = Some(db));
                    let sessions: Vec<String> = markers()
                        .iter()
                        .filter_map(|m| m.file_name()?.to_str())
                        .filter_map(crate::recovery::marker_id)
                        .map(str::to_string)
                        .collect();
                    for (path, bytes) in all {
                        if session_of(&path).is_some_and(|s| sessions.contains(&s)) {
                            LOADED.with(|l| l.borrow_mut().insert(path, bytes));
                        } else {
                            remove(&path);
                        }
                    }
                }
                Err(e) => log::warn!("no auto-backup in this browser: {e:?}"),
            }
            READY.with(|r| r.set(true));
            super::request_repaint();
        });
    }

    /// Read the database and listen for the page being left.
    pub fn install() {
        use wasm_bindgen::JsCast as _;
        load();
        let Some(window) = web_sys::window() else {
            return;
        };
        // Unsaved drawings: the browser asks before closing or reloading
        // the tab, going back or following a link away (Ctrl+W, the
        // close button and Back cannot be kept from it).
        let ask = Closure::<dyn FnMut(web_sys::BeforeUnloadEvent)>::new(
            |e: web_sys::BeforeUnloadEvent| {
                // eframe's own listener, which comes after this one,
                // calls `Shell::save`: it backs up at once.
                UNLOADING.with(|u| u.set(true));
                if UNSAVED.with(Cell::get) {
                    e.prevent_default();
                    // Older browsers ask only when this is set.
                    e.set_return_value("unsaved");
                }
            },
        );
        let _ =
            window.add_event_listener_with_callback("beforeunload", ask.as_ref().unchecked_ref());
        ask.forget();
        // Leaving for good (not into the back-forward cache): the next
        // visit offers what was left at once. The page hides after this
        // (or before it, as browsers differ); `Shell::save` then ends the
        // session as `App::leave_session` does.
        let gone = Closure::<dyn FnMut(web_sys::PageTransitionEvent)>::new(
            |e: web_sys::PageTransitionEvent| {
                if !e.persisted() {
                    LEFT.with(|l| l.set(true));
                    if let Some(marker) = MARKER.with(|m| m.borrow().clone()) {
                        crate::recovery::mark_gone(&marker);
                    }
                }
            },
        );
        let _ = window.add_event_listener_with_callback("pagehide", gone.as_ref().unchecked_ref());
        gone.forget();
        // Checking in while no frame is drawn (a hidden tab, an idle one).
        let alive = Closure::<dyn FnMut()>::new(|| {
            if let Some(marker) = MARKER.with(|m| m.borrow().clone()) {
                crate::recovery::keep_alive(&marker, crate::recovery::now_secs());
            }
        });
        let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
            alive.as_ref().unchecked_ref(),
            KEEP_ALIVE_MS,
        );
        alive.forget();
    }
}
