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

/// Pictures and files copied in other applications (#1403): the paste
/// event offers them as files, which egui leaves aside (it takes only the
/// text). They are read in the background and pasted on a later frame
/// (`App::take_handed_pastes`); the paste shortcut's own frame knows one
/// is coming and does not paste the copy here instead.
fn watch_pasted_files() {
    use wasm_bindgen::closure::Closure;
    let Some(window) = web_sys::window() else {
        return;
    };
    let paste = Closure::<dyn FnMut(web_sys::ClipboardEvent)>::new(|e: web_sys::ClipboardEvent| {
        let Some(data) = e.clipboard_data() else {
            return;
        };
        let Some(list) = data.files() else {
            return;
        };
        let files: Vec<web_sys::File> = (0..list.length()).filter_map(|i| list.get(i)).collect();
        if files.is_empty() {
            return;
        }
        let text = data.get_data("text/plain").ok().filter(|t| !t.is_empty());
        crate::clipboard::expect_handed();
        wasm_bindgen_futures::spawn_local(async move {
            let mut handed = crate::clipboard::Handed {
                text,
                ..Default::default()
            };
            for file in files {
                match blob_bytes(&file).await {
                    Some(bytes) => handed.files.push(crate::clipboard::PastedFile {
                        name: file.name(),
                        mime: file.type_(),
                        bytes,
                    }),
                    None => log::warn!("the pasted file {} cannot be read", file.name()),
                }
            }
            crate::clipboard::hand_over(handed);
            request_repaint();
        });
    });
    let _ = window.add_event_listener_with_callback_and_bool(
        "paste",
        paste.as_ref().unchecked_ref(),
        true,
    );
    paste.forget();
}

/// The bytes of a blob (a pasted file, a clipboard item).
async fn blob_bytes(blob: &web_sys::Blob) -> Option<Vec<u8>> {
    let buffer = wasm_bindgen_futures::JsFuture::from(blob.array_buffer())
        .await
        .ok()?;
    Some(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// Edit > Paste (#1403): read the clipboard with the browser's clipboard
/// API (it asks the user the first time) and hand what it holds over as
/// `handed`: its picture and its text. False when the page has no such
/// API (an insecure page, an old browser): the caller pastes the copy
/// made here.
pub fn read_clipboard(handed: crate::clipboard::Handed) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    let has_read = js_sys::Reflect::get(&navigator, &wasm_bindgen::JsValue::from_str("clipboard"))
        .ok()
        .filter(|c| !c.is_undefined() && !c.is_null())
        .and_then(|c| js_sys::Reflect::get(&c, &wasm_bindgen::JsValue::from_str("read")).ok())
        .is_some_and(|f| f.is_function());
    if !has_read {
        return false;
    }
    let clipboard = navigator.clipboard();
    wasm_bindgen_futures::spawn_local(async move {
        let mut handed = handed;
        match wasm_bindgen_futures::JsFuture::from(clipboard.read()).await {
            Ok(items) => {
                let items: js_sys::Array = items.unchecked_into();
                for item in items.iter() {
                    let item: web_sys::ClipboardItem = item.unchecked_into();
                    let types: Vec<String> =
                        item.types().iter().filter_map(|t| t.as_string()).collect();
                    if let Some(mime) = types.iter().find(|t| t.starts_with("image/")) {
                        if let Some(bytes) = clipboard_item_bytes(&item, mime).await {
                            let ext = mime.trim_start_matches("image/").replace("svg+xml", "svg");
                            handed.files.push(crate::clipboard::PastedFile {
                                name: format!("clipboard.{ext}"),
                                mime: mime.clone(),
                                bytes,
                            });
                        }
                    }
                    if handed.text.is_none() && types.iter().any(|t| t == "text/plain") {
                        handed.text = clipboard_item_bytes(&item, "text/plain")
                            .await
                            .and_then(|b| String::from_utf8(b).ok())
                            .map(|t| t.replace("\r\n", "\n"));
                    }
                }
            }
            Err(e) => {
                log::info!("the browser did not let the clipboard be read: {e:?}");
                handed.refused = true;
            }
        }
        crate::clipboard::hand_over(handed);
        request_repaint();
    });
    true
}

/// The bytes of one type of a clipboard item.
async fn clipboard_item_bytes(item: &web_sys::ClipboardItem, mime: &str) -> Option<Vec<u8>> {
    let blob = wasm_bindgen_futures::JsFuture::from(item.get_type(mime))
        .await
        .ok()?;
    blob_bytes(blob.unchecked_ref()).await
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

/// Store a value in the browser's local storage. `Err(full)` when the
/// browser refused it, `full` when its quota for the page is full
/// (#1414).
pub fn storage_set(key: &str, value: &str) -> Result<(), bool> {
    let Some(Ok(Some(s))) = web_sys::window().map(|w| w.local_storage()) else {
        return Err(false);
    };
    s.set_item(key, value).map_err(|e| quota_error(&e))
}

/// Remove a value from the browser's local storage.
pub fn storage_remove(key: &str) {
    if let Some(Ok(Some(s))) = web_sys::window().map(|w| w.local_storage()) {
        let _ = s.remove_item(key);
    }
}

/// Whether a browser error is a full storage quota.
pub fn quota_error(e: &wasm_bindgen::JsValue) -> bool {
    js_sys::Reflect::get(e, &wasm_bindgen::JsValue::from_str("name"))
        .ok()
        .and_then(|n| n.as_string())
        .is_some_and(|n| crate::storage_full::is_quota_error(&n))
}

/// Ask the browser, once per visit and at the first thing kept in
/// IndexedDB, to keep the page's storage when the disk runs low instead
/// of clearing it (`navigator.storage.persist()`, #1414). Browsers that
/// grant it by themselves (an installed app, a site often visited) say
/// nothing; others may ask the user.
pub fn request_persistence() {
    thread_local! {
        static ASKED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    if ASKED.with(|a| a.replace(true)) {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let get = |o: &wasm_bindgen::JsValue, key: &str| {
        js_sys::Reflect::get(o, &wasm_bindgen::JsValue::from_str(key))
            .ok()
            .filter(|v| !v.is_undefined() && !v.is_null())
    };
    let Some(storage) = get(&window.into(), "navigator").and_then(|n| get(&n, "storage")) else {
        return;
    };
    // `storage.persisted()` and `storage.persist()`: promises of a bool.
    let ask = move |method: &str| {
        let promise = get(&storage, method)
            .and_then(|f| f.dyn_into::<js_sys::Function>().ok())
            .and_then(|f| f.call0(&storage).ok())
            .and_then(|p| p.dyn_into::<js_sys::Promise>().ok());
        async move {
            wasm_bindgen_futures::JsFuture::from(promise?)
                .await
                .ok()?
                .as_bool()
        }
    };
    wasm_bindgen_futures::spawn_local(async move {
        if ask("persisted").await == Some(true) {
            return;
        }
        match ask("persist").await {
            Some(true) => log::info!("the browser keeps this page's storage"),
            _ => log::info!("the browser may clear this page's storage when the disk runs low"),
        }
    });
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
/// tools stay with the browser (`web_page::browser_keeps_key`), and so do
/// the characters typed with AltGr, which Windows browsers report with
/// Ctrl held (`web_page::typed_with_altgr`, #1408). Alt tapped alone and
/// Alt shortcuts are kept from the browser too, which would give the
/// keyboard focus to its menu (`web_page::AltGuard`, #1407).
fn guard_shortcuts() {
    use wasm_bindgen::closure::Closure;
    let Some(window) = web_sys::window() else {
        return;
    };
    let apple = crate::web_page::apple_platform(
        &navigator_text(&window, "platform"),
        &navigator_text(&window, "userAgent"),
    );
    let alt = std::rc::Rc::new(std::cell::Cell::new(crate::web_page::AltGuard::default()));
    let alt_down = alt.clone();
    let guard =
        Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
            let key = e.key();
            let alt_graph = e.get_modifier_state("AltGraph");
            let mut a = alt_down.get();
            let alt_kept = a.key_down(
                &key,
                &e.code(),
                e.ctrl_key(),
                e.meta_key(),
                e.alt_key(),
                alt_graph,
                apple,
            );
            alt_down.set(a);
            if alt_kept
                || crate::web_page::guard_keeps_from_browser(
                    &key,
                    e.ctrl_key(),
                    e.meta_key(),
                    e.shift_key(),
                    e.alt_key(),
                    alt_graph,
                )
            {
                e.prevent_default();
            }
        });
    let _ = window.add_event_listener_with_callback_and_bool(
        "keydown",
        guard.as_ref().unchecked_ref(),
        true,
    );
    guard.forget();
    let up = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(move |e: web_sys::KeyboardEvent| {
        let mut a = alt.get();
        let kept = a.key_up(&e.key(), apple);
        alt.set(a);
        if kept {
            e.prevent_default();
        }
    });
    let _ = window.add_event_listener_with_callback_and_bool(
        "keyup",
        up.as_ref().unchecked_ref(),
        true,
    );
    up.forget();
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

/// A text property of the browser's navigator ("" when missing).
fn navigator_text(window: &web_sys::Window, key: &str) -> String {
    let window: &wasm_bindgen::JsValue = window.as_ref();
    js_sys::Reflect::get(window, &wasm_bindgen::JsValue::from_str("navigator"))
        .and_then(|n| js_sys::Reflect::get(&n, &wasm_bindgen::JsValue::from_str(key)))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default()
}

/// Start the editor on the page's `<canvas id="tracedraw">`.
pub fn start() {
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    backups::install();
    kept::install();
    guard_shortcuts();
    watch_chorded_right_press();
    watch_pasted_files();
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

    /// Write a marker (local storage) or a backup (IndexedDB); a write
    /// the browser refuses is reported (`storage_full`, #1414), also when
    /// it fails only after this returns.
    pub fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        let error = |e: JsValue| {
            crate::storage_full::report(crate::storage_full::Kept::Backup, super::quota_error(&e));
            std::io::Error::other(format!("{e:?}"))
        };
        if is_marker(path) {
            let text = std::str::from_utf8(bytes)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            let storage = local().ok_or_else(|| error(JsValue::from_str("no local storage")))?;
            return storage.set_item(&marker_key(path), text).map_err(error);
        }
        super::request_persistence();
        let key = JsValue::from_str(&path.display().to_string());
        let value = js_sys::Uint8Array::from(bytes);
        with_store(&path.display().to_string(), true, |store| {
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
        if let Err(e) = with_store(&path.display().to_string(), false, |store| {
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
    /// failure that only shows later is logged, and reported when it
    /// was a backup being written (`report`).
    fn with_store(
        what: &str,
        report: bool,
        f: impl FnOnce(&web_sys::IdbObjectStore) -> Result<(), JsValue>,
    ) -> Result<(), JsValue> {
        DATABASE.with(|d| {
            let d = d.borrow();
            let db = d
                .as_ref()
                .ok_or_else(|| JsValue::from_str("the backup database is not open"))?;
            let kept = report.then_some(crate::storage_full::Kept::Backup);
            let tx = write_transaction(db, STORE, what, kept, None)?;
            f(&tx.object_store(STORE)?)?;
            commit(&tx);
            Ok(())
        })
    }

    /// A read-write transaction on `store` whose failure (it aborts when
    /// the quota is full) is logged and, with `kept`, reported to the
    /// user (#1414); `done` runs once it has been written.
    pub(super) fn write_transaction(
        db: &web_sys::IdbDatabase,
        store: &str,
        what: &str,
        kept: Option<crate::storage_full::Kept>,
        done: Option<Box<dyn FnOnce()>>,
    ) -> Result<web_sys::IdbTransaction, JsValue> {
        let tx = db.transaction_with_str_and_mode(store, web_sys::IdbTransactionMode::Readwrite)?;
        let what = what.to_string();
        let aborted = tx.clone();
        let failed = Closure::once_into_js(move |_: web_sys::Event| {
            let full = js_sys::Reflect::get(&aborted, &JsValue::from_str("error"))
                .is_ok_and(|e| super::quota_error(&e));
            log::warn!(
                "the browser did not store {what}{}",
                if full { " (storage full)" } else { "" }
            );
            if let Some(kept) = kept {
                crate::storage_full::report(kept, full);
            }
        });
        tx.set_onabort(Some(failed.unchecked_ref()));
        if let Some(done) = done {
            let done = Closure::once_into_js(move |_: web_sys::Event| done());
            tx.set_oncomplete(Some(done.unchecked_ref()));
        }
        Ok(tx)
    }

    /// Commit at once. Marked deprecated in web-sys's bindings only; a
    /// browser without it (Safari before 15) commits when the task ends.
    pub(super) fn commit(tx: &web_sys::IdbTransaction) {
        #[allow(deprecated)]
        let _ = tx.commit();
    }

    /// A promise settled by a request's success (with its result) or
    /// error.
    pub(super) fn settled(request: &web_sys::IdbRequest) -> js_sys::Promise {
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

    /// Open the database `name` (version 1) with its one `store`.
    pub(super) async fn open(name: &str, store: &str) -> Result<web_sys::IdbDatabase, JsValue> {
        let factory = web_sys::window()
            .ok_or_else(|| JsValue::from_str("no window"))?
            .indexed_db()?
            .ok_or_else(|| JsValue::from_str("no IndexedDB"))?;
        let request = factory.open_with_u32(name, 1)?;
        let made = request.clone();
        let store = store.to_string();
        let upgrade = Closure::once_into_js(move |_: web_sys::Event| {
            if let Ok(db) = made
                .result()
                .and_then(|r| r.dyn_into::<web_sys::IdbDatabase>())
            {
                let _ = db.create_object_store(&store);
            }
        });
        request.set_onupgradeneeded(Some(upgrade.unchecked_ref()));
        JsFuture::from(settled(&request)).await?.dyn_into()
    }

    /// Every value in `store` of the database, by key.
    pub(super) async fn read_all(
        db: &web_sys::IdbDatabase,
        store: &str,
    ) -> Result<Vec<(PathBuf, Vec<u8>)>, JsValue> {
        let tx = db.transaction_with_str(store)?;
        let store = tx.object_store(store)?;
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
            match open(DATABASE_NAME, STORE).await {
                Ok(db) => {
                    let all = read_all(&db, STORE).await.unwrap_or_else(|e| {
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

/// What the browser version keeps between visits besides the settings
/// and the auto-backups: the shelf (#1414). It goes to an IndexedDB
/// database of its own (`tracedraw-kept`, store `kept`), keyed by name,
/// which can hold far more than local storage (where the shelf was kept
/// before, sharing its few megabytes with the settings) and which older
/// versions of the page never touch. The database is read once when the
/// page opens ([`ready`]); a write the browser refuses is reported
/// (`storage_full`) and the values stay in memory.
pub mod kept {
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;
    use wasm_bindgen::JsValue;

    const DATABASE_NAME: &str = "tracedraw-kept";
    const STORE: &str = "kept";

    thread_local! {
        static READY: Cell<bool> = const { Cell::new(false) };
        static DATABASE: RefCell<Option<web_sys::IdbDatabase>> = const { RefCell::new(None) };
        /// The values, as read when the page opened and written since.
        static VALUES: RefCell<HashMap<String, Vec<u8>>> = RefCell::new(HashMap::new());
    }

    /// Read the database.
    pub fn install() {
        wasm_bindgen_futures::spawn_local(async {
            match super::backups::open(DATABASE_NAME, STORE).await {
                Ok(db) => {
                    match super::backups::read_all(&db, STORE).await {
                        Ok(all) => VALUES.with(|v| {
                            let mut v = v.borrow_mut();
                            for (key, bytes) in all {
                                v.insert(key.to_string_lossy().to_string(), bytes);
                            }
                        }),
                        Err(e) => log::warn!("reading the kept data: {e:?}"),
                    }
                    DATABASE.with(|d| *d.borrow_mut() = Some(db));
                }
                Err(e) => log::warn!("nothing can be kept in IndexedDB in this browser: {e:?}"),
            }
            READY.with(|r| r.set(true));
            super::request_repaint();
        });
    }

    /// True once the database has been read (or could not be opened).
    pub fn ready() -> bool {
        READY.with(Cell::get)
    }

    /// The value kept under `key`.
    pub fn get(key: &str) -> Option<Vec<u8>> {
        VALUES.with(|v| v.borrow().get(key).cloned())
    }

    /// Keep `bytes` under `key` (`what` says what it is when the browser
    /// refuses); `done` runs once they are written. False when there is
    /// no database to write to.
    pub fn put(
        key: &str,
        bytes: &[u8],
        what: crate::storage_full::Kept,
        done: Option<Box<dyn FnOnce()>>,
    ) -> bool {
        VALUES.with(|v| v.borrow_mut().insert(key.to_string(), bytes.to_vec()));
        let started = DATABASE.with(|d| {
            let d = d.borrow();
            let Some(db) = d.as_ref() else {
                return Ok(false);
            };
            super::request_persistence();
            let tx = super::backups::write_transaction(db, STORE, key, Some(what), done)?;
            let value = js_sys::Uint8Array::from(bytes);
            tx.object_store(STORE)?
                .put_with_key(&value, &JsValue::from_str(key))?;
            super::backups::commit(&tx);
            Ok::<bool, JsValue>(true)
        });
        match started {
            Ok(started) => started,
            Err(e) => {
                crate::storage_full::report(what, super::quota_error(&e));
                true
            }
        }
    }
}
