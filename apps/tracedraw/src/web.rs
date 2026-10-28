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
}

/// Start the editor on the page's `<canvas id="tracedraw">`.
pub fn start() {
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
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
