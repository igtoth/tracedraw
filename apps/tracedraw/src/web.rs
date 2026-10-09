//! The browser build: start-up on a canvas, downloads, local storage and
//! the bundled fonts. Compiled only for `wasm32`.

use std::cell::RefCell;
use wasm_bindgen::JsCast;

thread_local! {
    /// The egui context, so background tasks (file pickers) can ask for a
    /// frame when they finish.
    static CTX: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
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
/// letter (export, import, group, duplicate...) and the function keys
/// would otherwise open browser features. Copy, cut, paste (egui needs
/// those events), reload (Ctrl+R) and the developer tools stay with the
/// browser.
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
        let k = key.to_ascii_lowercase();
        let keep = matches!(k.as_str(), "c" | "v" | "x" | "r") || (e.shift_key() && k == "i");
        if !keep {
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

/// Start the editor on the page's `<canvas id="tracedraw">`.
pub fn start() {
    eframe::WebLogger::init(log::LevelFilter::Info).ok();
    guard_shortcuts();
    register_fonts();
    tracedraw_text::install();
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
                Err(e) => loading.set_inner_html(&format!(
                    "TraceDraw could not start: {e:?}. A browser with WebGL 2 is needed."
                )),
            }
        }
    });
}
