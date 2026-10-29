//! System clipboard and external bitmap editing.
//!
//! Copy puts the selection on the system clipboard as SVG text next to the
//! internal clipboard, so other applications can paste it. Paste Special
//! offers the system clipboard's content as text, as a bitmap or as vector
//! objects (SVG). Bitmaps > Edit Bitmap hands a PNG to the system's image
//! editor and Update from Link reads it back.

use crate::app::App;
use crate::i18n::{tr, trf};
use tracedraw_core::{
    document::{ShapeKind, TextSpan},
    geometry::Point,
    Command, ShapeId,
};

/// What the system clipboard currently offers.
#[derive(Debug, Clone, Default)]
pub struct ClipboardContent {
    pub text: Option<String>,
    pub image: Option<image::RgbaImage>,
    /// Files copied in a file manager.
    pub files: Vec<std::path::PathBuf>,
}

impl ClipboardContent {
    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.image.is_none() && self.files.is_empty()
    }

    /// What a [`Mark`] keeps of this content.
    pub fn mark(&self, seq: Option<u32>) -> Mark {
        Mark {
            seq,
            text: hash_of(&self.text),
            files: hash_of(&self.files),
            image: self.image.as_ref().map(image_sample),
        }
    }

    pub fn is_svg(&self) -> bool {
        self.text
            .as_deref()
            .map(|t| t.trim_start().starts_with('<') && t.contains("<svg"))
            .unwrap_or(false)
    }
}

/// What the system clipboard held when TraceDraw copied, so that a paste
/// can tell whether something was copied elsewhere since (#183). Where
/// the system counts clipboard changes (Windows) the count decides and
/// nothing is read; elsewhere the text and the files decide, and a
/// picture is decoded and sampled only when there is neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mark {
    /// The system's change count (Windows), `None` where there is none.
    pub seq: Option<u32>,
    text: u64,
    files: u64,
    image: Option<u64>,
}

impl Mark {
    /// Whether the clipboard, now at change count `seq` with `text` and
    /// `files`, still holds what was marked; `image` decodes and samples
    /// its picture, called only when the rest cannot tell.
    pub fn still_holds(
        &self,
        seq: Option<u32>,
        text: &Option<String>,
        files: &[std::path::PathBuf],
        image: impl FnOnce() -> Option<u64>,
    ) -> bool {
        if let (Some(then), Some(now)) = (self.seq, seq) {
            return then == now;
        }
        if hash_of(text) != self.text || hash_of(&files) != self.files {
            return false;
        }
        // Equal text or files: the same copy (a picture copied elsewhere
        // replaces them).
        if text.is_some() || !files.is_empty() {
            return true;
        }
        image() == self.image
    }
}

fn hash_of<T: std::hash::Hash + ?Sized>(v: &T) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

/// A picture's size and about 4096 of its bytes spread over it: enough to
/// tell two copied pictures apart without hashing 48 MB.
fn image_sample(img: &image::RgbaImage) -> u64 {
    use std::hash::{Hash, Hasher};
    let raw = img.as_raw();
    let step = (raw.len() / 4096).max(1);
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (img.width(), img.height()).hash(&mut h);
    for b in raw.iter().step_by(step) {
        b.hash(&mut h);
    }
    h.finish()
}

/// The system's clipboard change count: Windows counts every change, so
/// a paste can tell nothing changed without reading the clipboard.
#[cfg(windows)]
pub fn change_count() -> Option<u32> {
    #[link(name = "user32", kind = "raw-dylib")]
    unsafe extern "system" {
        fn GetClipboardSequenceNumber() -> u32;
    }
    // SAFETY: no arguments, it only returns a number (0 without access to
    // the window station).
    let n = unsafe { GetClipboardSequenceNumber() };
    (n != 0).then_some(n)
}

#[cfg(not(windows))]
pub fn change_count() -> Option<u32> {
    None
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn read_system_text_and_files() -> ClipboardContent {
    read_text_and_files()
}

#[cfg(target_arch = "wasm32")]
fn read_system_text_and_files() -> ClipboardContent {
    ClipboardContent::default()
}

/// The system clipboard's text on Android, through the activity's
/// clipboard service (files and pictures are not read there yet).
#[cfg(target_os = "android")]
fn read_system_text_and_files() -> ClipboardContent {
    ClipboardContent {
        text: crate::android::clipboard::text(),
        ..Default::default()
    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn read_system_image() -> Option<image::RgbaImage> {
    read_image()
}

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
fn read_system_image() -> Option<image::RgbaImage> {
    None
}

/// The system clipboard as seen by the browser build: empty, because
/// browsers only hand clipboard contents to paste events.
#[cfg(target_arch = "wasm32")]
pub fn read_system() -> ClipboardContent {
    ClipboardContent::default()
}

/// The system clipboard on Android: its text (`android.rs`).
#[cfg(target_os = "android")]
pub fn read_system() -> ClipboardContent {
    read_system_text_and_files()
}

/// Read text, files and image from the system clipboard; failures yield
/// `None` fields (headless sessions have no clipboard).
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub fn read_system() -> ClipboardContent {
    let mut content = read_text_and_files();
    content.image = read_image();
    content
}

/// The clipboard's text and files, not its picture (decoding one costs
/// far more).
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn read_text_and_files() -> ClipboardContent {
    let Ok(mut cb) = arboard::Clipboard::new() else {
        return ClipboardContent::default();
    };
    let text = cb.get_text().ok().filter(|t| !t.is_empty());
    let files = cb
        .get()
        .file_list()
        .unwrap_or_default()
        .into_iter()
        .map(without_line_end)
        .collect();
    ClipboardContent {
        text,
        image: None,
        files,
    }
}

/// The clipboard's picture, decoded.
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn read_image() -> Option<image::RgbaImage> {
    let mut cb = arboard::Clipboard::new().ok()?;
    let img = cb.get_image().ok()?;
    image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned())
}

/// A path from a copied file list without the carriage return its line
/// may keep: the list's lines end in CR LF and the clipboard library
/// splits it at LF only.
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn without_line_end(path: std::path::PathBuf) -> std::path::PathBuf {
    match path.to_str() {
        Some(s) if s.ends_with(['\r', '\n']) => s.trim_end_matches(['\r', '\n']).into(),
        _ => path,
    }
}

/// Put text on the system clipboard (best effort); whether it went there.
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub fn write_text(text: &str) -> bool {
    arboard::Clipboard::new().is_ok_and(|mut cb| cb.set_text(text.to_string()).is_ok())
}

/// Put text on the system clipboard through the activity's clipboard
/// service (Android); whether it went there.
#[cfg(target_os = "android")]
pub fn write_text(text: &str) -> bool {
    crate::android::clipboard::set_text(text)
}

/// Put text on the system clipboard through egui (browser build).
#[cfg(target_arch = "wasm32")]
pub fn write_text(text: &str) -> bool {
    crate::web::copy_text(text);
    true
}

impl App {
    /// The selection as a standalone SVG document (page-sized viewport so
    /// positions survive a round trip).
    pub fn selection_svg(&self) -> Option<String> {
        let shapes = self.selected_shapes();
        if shapes.is_empty() {
            return None;
        }
        let mut tmp = tracedraw_core::Document::new("clipboard", self.page_size());
        tmp.symbols = self.doc().symbols.clone();
        let layer = tmp.pages[0].layers[0].id;
        if let Ok(l) = tmp.layer_mut(layer) {
            l.shapes = shapes;
        }
        Some(tracedraw_io::svg::page_to_svg(&tmp, 0))
    }

    /// Copy to the internal clipboard and, as SVG, to the system clipboard.
    pub fn copy_with_system(&mut self) {
        // Guidelines selected and no objects: they are copied (#376).
        if self.selection.is_empty() && self.copy_selected_guides() {
            return;
        }
        // Nothing selected: the last copy stays as it was.
        let Some(svg) = self.selection_svg() else {
            return;
        };
        self.copy();
        let written = write_text(&svg);
        self.mark_system_clipboard(written.then_some(svg.as_str()));
    }

    /// Remember what the system clipboard holds right after a copy here
    /// (`written`: the text the copy put there), so that a paste can tell
    /// whether something was copied elsewhere since. What was written is
    /// what the clipboard holds (writing text replaces everything there),
    /// so it is not read back; a copy that wrote nothing (nodes, guides)
    /// reads it, its picture only where the system keeps no change count
    /// and there is no text or file (#183).
    pub fn mark_system_clipboard(&mut self, written: Option<&str>) {
        let seq = change_count();
        let mark = match written {
            Some(text) => ClipboardContent {
                text: Some(text.to_string()),
                ..Default::default()
            }
            .mark(seq),
            None if seq.is_some() => Mark {
                seq,
                ..Default::default()
            },
            None => {
                let mut now = read_system_text_and_files();
                if now.text.is_none() && now.files.is_empty() {
                    now.image = read_system_image();
                }
                now.mark(seq)
            }
        };
        if let Some(cb) = self.clipboard.as_mut() {
            cb.system = mark;
        }
    }

    /// Paste the system clipboard's text as artistic text at the page centre.
    pub fn paste_as_text(&mut self, text: &str) {
        let c = self.page_rect().center();
        let span = TextSpan {
            bold: self.text_bold,
            italic: self.text_italic,
            ..TextSpan::new(text, self.text_font.clone(), self.text_size_pt)
        };
        let kind = ShapeKind::Text {
            spans: vec![span],
            origin: Point::new(c.x, c.y),
            frame: None,
            align: tracedraw_core::TextAlign::Left,
            para: Default::default(),
            on_path: None,
        };
        if let Some(id) = self.new_shape(kind) {
            self.run(Command::SetFill {
                shapes: vec![id],
                fill: tracedraw_core::Fill::Solid(tracedraw_core::Color::BLACK),
            });
            self.run(Command::SetStroke {
                shapes: vec![id],
                stroke: None,
            });
            self.select(vec![id]);
        }
    }

    /// Paste SVG text from the system clipboard as objects.
    pub fn paste_as_svg(&mut self, svg: &str) {
        let Some(layer) = self.active_layer() else {
            return;
        };
        match tracedraw_io::svg_import::parse(svg, &mut tracedraw_core::id::IdSource::default()) {
            Ok(imported) => {
                let mut cmds = Vec::new();
                let mut ids = Vec::new();
                for s in imported.shapes {
                    let id = self.engine.new_shape_id();
                    let shape = crate::app::reid_pub(s, id, &mut self.engine);
                    ids.push(id);
                    cmds.push(Command::AddShape { layer, shape });
                }
                if let Err(e) = self.engine.run_batch("Paste", &cmds) {
                    self.status = e.to_string();
                }
                self.select(ids);
            }
            Err(e) => self.status = trf("status.import_failed", &[("e", &e)]),
        }
    }

    /// Paste the system clipboard's image as a bitmap object.
    pub fn paste_as_bitmap(&mut self, img: image::RgbaImage) {
        if let Some(id) = self.place_image(img, Some(tr("kind.bitmap_short"))) {
            self.select(vec![id]);
        }
    }

    /// Edit > Paste: what was copied last, here or in another
    /// application.
    pub fn paste_any(&mut self) {
        self.paste_with_event_text(None);
    }

    /// Ctrl+V, with the text of the paste event when there was one (a
    /// browser offers the clipboard's text only that way).
    pub fn paste_with_event_text(&mut self, text: Option<String>) {
        if crate::files::WEB {
            let content = ClipboardContent {
                text: text.filter(|t| !t.is_empty()),
                ..Default::default()
            };
            self.paste_with(None, || content, || None);
        } else {
            self.paste_with(
                change_count(),
                read_system_text_and_files,
                read_system_image,
            );
        }
    }

    /// Paste with the clipboard at change count `seq`: `read` gives its
    /// text and files, `read_image` decodes its picture, called only when
    /// the change check needs it or it is what gets pasted (no files, no
    /// SVG), and at most once (#183).
    fn paste_with(
        &mut self,
        seq: Option<u32>,
        read: impl FnOnce() -> ClipboardContent,
        read_image: impl FnOnce() -> Option<image::RgbaImage>,
    ) {
        // Nothing copied anywhere since the copy here: nothing to read.
        if self
            .clipboard
            .as_ref()
            .is_some_and(|cb| seq.is_some() && cb.system.seq == seq)
        {
            self.paste();
            return;
        }
        let mut content = read();
        let mut read_image = Some(read_image);
        let mut take_image = || read_image.take().and_then(|f| f());
        if let Some(cb) = &self.clipboard {
            let held = cb
                .system
                .still_holds(seq, &content.text, &content.files, || {
                    content.image = take_image();
                    content.image.as_ref().map(image_sample)
                });
            if held {
                self.paste();
                return;
            }
        }
        if content.image.is_none() && content.files.is_empty() && !content.is_svg() {
            content.image = take_image();
        }
        self.paste_from_unmarked(content);
    }

    /// [`App::paste_with`] on `content` (the tests' clipboard).
    #[cfg(test)]
    pub fn paste_from(&mut self, content: ClipboardContent) {
        let ClipboardContent { text, image, files } = content;
        self.paste_with(
            None,
            || ClipboardContent {
                text,
                image: None,
                files,
            },
            || image,
        );
    }

    /// What the system clipboard offers, or the copy here when it offers
    /// nothing.
    fn paste_from_unmarked(&mut self, content: ClipboardContent) {
        if content.is_empty() && self.clipboard.is_some() {
            self.paste();
            return;
        }
        if !content.files.is_empty() {
            for path in &content.files {
                self.import_path(path);
            }
            return;
        }
        if content.is_svg() {
            if let Some(t) = content.text {
                self.paste_as_svg(&t);
            }
        } else if let Some(img) = content.image {
            self.paste_as_bitmap(img);
        } else if let Some(t) = content.text {
            self.paste_as_text(&t);
        }
    }

    // ----- external bitmap editing -------------------------------------------------

    fn bitmap_link(&self, id: ShapeId) -> Option<String> {
        self.doc()
            .find_shape(id)?
            .data
            .iter()
            .find(|(k, _)| k == "bitmap.link")
            .map(|(_, v)| v.clone())
    }

    pub fn selected_bitmap(&self) -> Option<ShapeId> {
        let id = *self.selection.first()?;
        let s = self.doc().find_shape(id)?;
        matches!(s.kind, ShapeKind::Bitmap { .. }).then_some(id)
    }

    pub fn selected_bitmap_is_linked(&self) -> bool {
        self.selected_bitmap()
            .and_then(|id| self.bitmap_link(id))
            .is_some()
    }

    /// The selected bitmap's PNG, its effects included.
    pub fn selected_bitmap_png(&self) -> Option<tracedraw_core::SharedBytes> {
        let id = self.selected_bitmap()?;
        match &self.doc().find_shape(id)?.kind {
            ShapeKind::Bitmap { png, .. } => Some(png.clone()),
            _ => None,
        }
    }

    /// Save Bitmap As (the bitmap's context menu): its pixels, effects
    /// included, to a PNG file.
    pub fn save_bitmap_as(&mut self) {
        let Some(png) = self.selected_bitmap_png() else {
            return;
        };
        let name = format!("{}.png", self.document_name());
        crate::files::Dialog::new()
            .add_filter("PNG", &["png"])
            .set_file_name(name)
            .save_file(self, move |app, path| {
                app.status = match crate::files::write(&path, &png) {
                    Ok(()) => crate::i18n::trf(
                        "status.exported",
                        &[("path", &path.display().to_string())],
                    ),
                    Err(e) => crate::i18n::trf("status.export_failed", &[("e", &e.to_string())]),
                };
            });
    }

    /// Bitmaps > Edit Bitmap: write the pixels to a PNG next to the system's
    /// temporary files, link the object to it and open the system editor.
    pub fn edit_bitmap_externally(&mut self) {
        if crate::files::WEB {
            // A browser cannot hand a file to another program.
            self.status = tr("status.not_in_browser");
            return;
        }
        let Some(id) = self.selected_bitmap() else {
            return;
        };
        let Some(s) = self.doc().find_shape(id).cloned() else {
            return;
        };
        let ShapeKind::Bitmap { png, .. } = &s.kind else {
            return;
        };
        let dir = std::env::temp_dir().join("tracedraw-edit");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.status = e.to_string();
            return;
        }
        let path = dir.join(format!("bitmap-{}.png", id.raw()));
        if let Err(e) = std::fs::write(&path, png) {
            self.status = e.to_string();
            return;
        }
        let mut data = s.data.clone();
        data.retain(|(k, _)| k != "bitmap.link");
        data.push(("bitmap.link".into(), path.display().to_string()));
        self.run(Command::SetObjectData { shape: id, data });
        #[cfg(not(target_arch = "wasm32"))]
        match crate::export::open_in_editor(&self.settings.bitmap_editor, &path) {
            Ok(()) => self.status = tr("status.bitmap_sent_to_editor"),
            Err(e) => self.status = e,
        }
    }

    /// Bitmaps > Update from Link: re-read the linked file into the object.
    pub fn update_bitmap_from_link(&mut self) {
        let Some(id) = self.selected_bitmap() else {
            return;
        };
        self.reload_linked_bitmap(id);
    }

    /// Re-read the file bitmap `id` is linked to into the object (Update
    /// from Link, or the file changed on disk). False when it is not
    /// linked or the file cannot be read (the status bar says so).
    pub fn reload_linked_bitmap(&mut self, id: ShapeId) -> bool {
        let Some(path) = self.bitmap_link(id) else {
            return false;
        };
        // A package's links are relative to its drawing (#852).
        let path = crate::package::resolve_link(&path, self.link_base().as_deref())
            .display()
            .to_string();
        let img = match crate::files::open_image(&path) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                self.status = trf(
                    "status.could_not_read",
                    &[("p", &path), ("e", &e.to_string())],
                );
                return false;
            }
        };
        let Some(s) = self.doc().find_shape(id).cloned() else {
            return false;
        };
        let ShapeKind::Bitmap { rect, .. } = s.kind else {
            return false;
        };
        let (w, h) = img.dimensions();
        let mut png = Vec::new();
        if image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .is_err()
        {
            return false;
        }
        self.run(Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Bitmap {
                rect,
                width_px: w,
                height_px: h,
                png: png.into(),
                fx: None,
            },
        });
        self.status = tr("status.bitmap_updated");
        true
    }

    /// Linked bitmaps (Edit Bitmap, an imported file) follow their file:
    /// once a second the files' modification times are read, and a
    /// bitmap whose file changed since it was last seen is read again, so
    /// a save in the outside editor shows here at once. Desktop only.
    /// `now` is the frame's time; `ctx` keeps a frame coming while the
    /// watch is on.
    pub fn watch_linked_bitmaps(&mut self, ctx: Option<&egui::Context>, now: web_time::Instant) {
        if crate::files::WEB {
            return;
        }
        let gap = std::time::Duration::from_secs(1);
        if let Some(last) = self.linked_checked {
            let since = now.duration_since(last);
            if since < gap {
                if let Some(ctx) = ctx {
                    if !self.linked_mtimes.is_empty() {
                        ctx.request_repaint_after(gap - since);
                    }
                }
                return;
            }
        }
        self.linked_checked = Some(now);
        let linked: Vec<(ShapeId, String)> = self
            .doc()
            .all_layers()
            .flat_map(|l| l.shapes.iter())
            .filter_map(|s| {
                s.data
                    .iter()
                    .find(|(k, _)| k == "bitmap.link")
                    .map(|(_, v)| (s.id, v.clone()))
            })
            .collect();
        self.linked_mtimes
            .retain(|id, _| linked.iter().any(|(l, _)| l == id));
        let base = self.link_base();
        for (id, path) in linked {
            let path = crate::package::resolve_link(&path, base.as_deref());
            let Ok(modified) = std::fs::metadata(&path).and_then(|m| m.modified()) else {
                continue;
            };
            match self.linked_mtimes.insert(id, modified) {
                Some(seen) if seen != modified => {
                    self.reload_linked_bitmap(id);
                }
                _ => {}
            }
        }
        if let Some(ctx) = ctx {
            if !self.linked_mtimes.is_empty() {
                ctx.request_repaint_after(gap);
            }
        }
    }

    /// Bitmaps > Break Link: forget the file; the pixels stay embedded.
    pub fn break_bitmap_link(&mut self) {
        let Some(id) = self.selected_bitmap() else {
            return;
        };
        let Some(s) = self.doc().find_shape(id).cloned() else {
            return;
        };
        let mut data = s.data;
        data.retain(|(k, _)| k != "bitmap.link");
        self.run(Command::SetObjectData { shape: id, data });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape_count(app: &App) -> usize {
        app.doc().all_layers().map(|l| l.shapes.len()).sum()
    }

    /// A drawing with one rectangle, copied here; the system clipboard
    /// then held `system`.
    fn copied(system: &ClipboardContent) -> App {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        app.copy();
        if let Some(cb) = app.clipboard.as_mut() {
            cb.system = system.mark(None);
        }
        app
    }

    fn svg_text() -> ClipboardContent {
        ClipboardContent {
            text: Some("<svg xmlns=\"http://www.w3.org/2000/svg\"/>".into()),
            ..Default::default()
        }
    }

    fn picture() -> ClipboardContent {
        ClipboardContent {
            image: Some(image::RgbaImage::from_pixel(
                3,
                2,
                image::Rgba([1, 2, 3, 255]),
            )),
            ..Default::default()
        }
    }

    fn is_bitmap(app: &App) -> bool {
        app.selected_shapes()
            .first()
            .is_some_and(|s| matches!(s.kind, ShapeKind::Bitmap { .. }))
    }

    #[test]
    fn the_copy_here_is_pasted_while_the_system_clipboard_still_holds_it() {
        let mut app = copied(&svg_text());
        app.paste_from(svg_text());
        assert_eq!(shape_count(&app), 2);
        assert!(matches!(
            app.selected_shapes().first().map(|s| &s.kind),
            Some(ShapeKind::Rect { .. })
        ));
        // No system clipboard (a session without one): the copy here.
        app.paste_from(ClipboardContent::default());
        assert_eq!(shape_count(&app), 3);
    }

    #[test]
    fn a_picture_copied_elsewhere_afterwards_wins_over_the_copy_here() {
        let mut app = copied(&svg_text());
        app.paste_from(picture());
        assert_eq!(shape_count(&app), 2);
        assert!(is_bitmap(&app), "the picture, not the rectangle");
        // Text copied elsewhere becomes text.
        app.paste_from(ClipboardContent {
            text: Some("Hello".into()),
            ..Default::default()
        });
        assert!(matches!(
            app.selected_shapes().first().map(|s| &s.kind),
            Some(ShapeKind::Text { .. })
        ));
    }

    #[test]
    fn files_copied_in_a_file_manager_are_imported() {
        let dir = std::env::temp_dir().join(format!("tracedraw-paste-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("dot.png");
        image::RgbaImage::from_pixel(4, 2, image::Rgba([0, 120, 215, 255]))
            .save(&path)
            .expect("png");
        let mut app = copied(&svg_text());
        app.paste_from(ClipboardContent {
            files: vec![path],
            ..Default::default()
        });
        assert_eq!(shape_count(&app), 2);
        assert!(is_bitmap(&app));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn copied_file_paths_lose_the_line_end() {
        use std::path::PathBuf;
        assert_eq!(
            without_line_end(PathBuf::from("/home/a/b c.psd\r")),
            PathBuf::from("/home/a/b c.psd")
        );
        assert_eq!(
            without_line_end(PathBuf::from("/home/a.png")),
            PathBuf::from("/home/a.png")
        );
    }

    #[test]
    fn the_mark_follows_the_content() {
        let empty = ClipboardContent::default();
        assert!(empty.is_empty());
        assert_eq!(empty.mark(None), ClipboardContent::default().mark(None));
        assert_ne!(empty.mark(None), svg_text().mark(None));
        assert_ne!(svg_text().mark(None), picture().mark(None));
        assert!(!picture().is_empty());
        // Pictures of one size told apart by their pixels.
        let mut other = picture();
        if let Some(img) = other.image.as_mut() {
            img.put_pixel(2, 1, image::Rgba([9, 9, 9, 255]));
        }
        assert_ne!(picture().mark(None), other.mark(None));
    }

    /// The change check reads as little as it can (#183): the system's
    /// change count decides alone; else equal text or files say the same
    /// copy without looking at the picture, and the picture is sampled
    /// only on a clipboard with neither.
    #[test]
    fn the_change_check_decodes_a_picture_only_when_it_must() {
        let no_image = || -> Option<u64> { panic!("the picture was decoded") };
        let counted = svg_text().mark(Some(7));
        assert!(counted.still_holds(Some(7), &None, &[], no_image));
        assert!(!counted.still_holds(Some(8), &svg_text().text, &[], no_image));
        let text = svg_text().mark(None);
        assert!(text.still_holds(None, &svg_text().text, &[], no_image));
        assert!(!text.still_holds(None, &Some("other".into()), &[], no_image));
        assert!(!text.still_holds(None, &None, &[], no_image));
        let pic = picture().mark(None);
        let sample = picture().image.as_ref().map(image_sample);
        let mut decoded = 0;
        assert!(pic.still_holds(None, &None, &[], || {
            decoded += 1;
            sample
        }));
        assert!(!pic.still_holds(None, &None, &[], || {
            decoded += 1;
            None
        }));
        assert_eq!(decoded, 2);
    }

    /// Pasting the copy here never decodes the picture of a clipboard that
    /// still holds the copy's text; a picture copied elsewhere is decoded
    /// once and pasted, files are imported without decoding one (#183).
    #[test]
    fn a_paste_decodes_the_picture_only_when_it_needs_it() {
        let mut app = copied(&svg_text());
        let no_image = || -> Option<image::RgbaImage> { panic!("the picture was decoded") };
        app.paste_with(None, svg_text, no_image);
        assert_eq!(shape_count(&app), 2);
        // The system's change count unchanged: nothing is read at all.
        if let Some(cb) = app.clipboard.as_mut() {
            cb.system.seq = Some(3);
        }
        app.paste_with(Some(3), || panic!("the clipboard was read"), no_image);
        assert_eq!(shape_count(&app), 3);
        // A picture copied elsewhere since: decoded once, pasted.
        let mut decoded = 0;
        app.paste_with(Some(4), ClipboardContent::default, || {
            decoded += 1;
            picture().image
        });
        assert_eq!(decoded, 1);
        assert!(is_bitmap(&app));
        // Text written by the copy here is what a paste reads back.
        let mut app = copied(&ClipboardContent::default());
        app.mark_system_clipboard(svg_text().text.as_deref());
        app.paste_with(None, svg_text, no_image);
        assert!(matches!(
            app.selected_shapes().first().map(|s| &s.kind),
            Some(ShapeKind::Rect { .. })
        ));
    }
}

/// What copy and paste cost with a 4000 x 3000 picture on the system
/// clipboard: run in a session with a clipboard (on Linux an X server,
/// such as `Xvfb :99` with `DISPLAY=:99`) with
/// `cargo test --release -p tracedraw -- --ignored clipboard_costs --nocapture`.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod clipboard_bench {
    use super::*;

    fn picture_elsewhere(cb: &mut arboard::Clipboard, shade: u8) {
        let (w, h) = (4000usize, 3000usize);
        let bytes: Vec<u8> = (0..w * h * 4).map(|i| (i % 251) as u8 ^ shade).collect();
        cb.set_image(arboard::ImageData {
            width: w,
            height: h,
            bytes: bytes.into(),
        })
        .expect("a clipboard");
    }

    fn time(label: &str, f: impl FnOnce()) {
        let t = std::time::Instant::now();
        f();
        eprintln!("{label}: {:.1} ms", t.elapsed().as_secs_f64() * 1000.0);
    }

    #[test]
    #[ignore]
    fn clipboard_costs() {
        let Ok(mut cb) = arboard::Clipboard::new() else {
            eprintln!("no clipboard");
            return;
        };
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: tracedraw_core::geometry::Rect::new(10.0, 10.0, 60.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("rect");
        app.select(vec![id]);
        picture_elsewhere(&mut cb, 0);
        time("Ctrl+C over a picture copied elsewhere", || {
            app.copy_with_system()
        });
        time("Ctrl+V of that copy", || app.paste_with_event_text(None));
        picture_elsewhere(&mut cb, 1);
        app.select(vec![id]);
        app.copy();
        time("a copy of nodes or guides (mark, picture kept)", || {
            app.mark_system_clipboard(None)
        });
        time("Ctrl+V of that copy, picture still there", || {
            app.paste_with_event_text(None)
        });
        time("reading and decoding the picture alone", || {
            let _ = read_system_image();
        });
        time("the change count (Windows; none elsewhere)", || {
            let _ = change_count();
        });
        picture_elsewhere(&mut cb, 2);
        let before = app
            .doc()
            .all_layers()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        time("Ctrl+V of a picture copied elsewhere since", || {
            app.paste_with_event_text(None)
        });
        let after = app
            .doc()
            .all_layers()
            .map(|l| l.shapes.len())
            .sum::<usize>();
        assert_eq!(after, before + 1);
    }
}

#[cfg(test)]
mod linked_watch_tests {
    use crate::app::App;
    use tracedraw_core::geometry::Rect;
    use tracedraw_core::{Command, ShapeKind};

    fn png(w: u32, h: u32) -> Vec<u8> {
        tiny_skia::Pixmap::new(w, h)
            .and_then(|p| p.encode_png().ok())
            .unwrap_or_default()
    }

    /// A linked bitmap follows its file: a change on disk is read in on
    /// the next check (once a second), as Update from Link does; a file
    /// seen for the first time is only remembered (#284).
    #[test]
    fn a_linked_bitmap_is_read_again_when_its_file_changes() {
        if crate::files::WEB {
            return;
        }
        let dir = std::env::temp_dir().join(format!("tracedraw-link-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("picture.png");
        std::fs::write(&path, png(2, 2)).expect("png");
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Bitmap {
                rect: Rect::new(10.0, 10.0, 30.0, 30.0),
                width_px: 2,
                height_px: 2,
                png: png(2, 2).into(),
                fx: None,
            })
            .expect("bitmap");
        app.run(Command::SetObjectData {
            shape: id,
            data: vec![("bitmap.link".into(), path.display().to_string())],
        });
        let width = |app: &App| match &app.doc().find_shape(id).expect("bitmap").kind {
            ShapeKind::Bitmap { width_px, .. } => *width_px,
            _ => 0,
        };
        let t0 = web_time::Instant::now();
        app.watch_linked_bitmaps(None, t0);
        assert_eq!(width(&app), 2, "first sight: remembered, not read");
        // The file changes (its time stamp moved on by two seconds).
        std::fs::write(&path, png(3, 3)).expect("png");
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(2);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .and_then(|f| f.set_modified(later))
            .expect("mtime");
        // Within the second: not looked at yet.
        app.watch_linked_bitmaps(None, t0 + std::time::Duration::from_millis(500));
        assert_eq!(width(&app), 2);
        app.watch_linked_bitmaps(None, t0 + std::time::Duration::from_millis(1100));
        assert_eq!(width(&app), 3, "read again");
        // Unchanged since: left alone (no new undo step).
        let depth = app.engine.history_labels().0.len();
        app.watch_linked_bitmaps(None, t0 + std::time::Duration::from_millis(2200));
        assert_eq!(app.engine.history_labels().0.len(), depth);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
