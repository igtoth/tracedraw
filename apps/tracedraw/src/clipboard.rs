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
}

impl ClipboardContent {
    pub fn is_svg(&self) -> bool {
        self.text
            .as_deref()
            .map(|t| t.trim_start().starts_with('<') && t.contains("<svg"))
            .unwrap_or(false)
    }
}

/// The system clipboard as seen by the browser build: empty, because
/// browsers only hand clipboard contents to paste events.
#[cfg(target_arch = "wasm32")]
pub fn read_system() -> ClipboardContent {
    ClipboardContent::default()
}

/// Read text and image from the system clipboard; failures yield `None`
/// fields (headless sessions have no clipboard).
#[cfg(not(target_arch = "wasm32"))]
pub fn read_system() -> ClipboardContent {
    let Ok(mut cb) = arboard::Clipboard::new() else {
        return ClipboardContent::default();
    };
    let text = cb.get_text().ok().filter(|t| !t.is_empty());
    let image = cb.get_image().ok().and_then(|img| {
        image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned())
    });
    ClipboardContent { text, image }
}

/// Put text on the system clipboard (best effort).
#[cfg(not(target_arch = "wasm32"))]
pub fn write_text(text: &str) {
    if let Ok(mut cb) = arboard::Clipboard::new() {
        let _ = cb.set_text(text.to_string());
    }
}

/// Put text on the system clipboard through egui (browser build).
#[cfg(target_arch = "wasm32")]
pub fn write_text(text: &str) {
    crate::web::copy_text(text);
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
        self.copy();
        if let Some(svg) = self.selection_svg() {
            write_text(&svg);
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

    /// Ctrl+V: the internal clipboard first, then whatever the system offers
    /// (SVG as objects, an image as a bitmap, text as artistic text).
    pub fn paste_any(&mut self) {
        if self.clipboard.is_some() {
            self.paste();
            return;
        }
        let content = read_system();
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
        match crate::export::open_with_system(&path) {
            Ok(()) => self.status = tr("status.bitmap_sent_to_editor"),
            Err(e) => self.status = e,
        }
    }

    /// Bitmaps > Update from Link: re-read the linked file into the object.
    pub fn update_bitmap_from_link(&mut self) {
        let Some(id) = self.selected_bitmap() else {
            return;
        };
        let Some(path) = self.bitmap_link(id) else {
            return;
        };
        let img = match crate::files::open_image(&path) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                self.status = trf(
                    "status.could_not_read",
                    &[("p", &path), ("e", &e.to_string())],
                );
                return;
            }
        };
        let Some(s) = self.doc().find_shape(id).cloned() else {
            return;
        };
        let ShapeKind::Bitmap { rect, .. } = s.kind else {
            return;
        };
        let (w, h) = img.dimensions();
        let mut png = Vec::new();
        if image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .is_err()
        {
            return;
        }
        self.run(Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Bitmap {
                rect,
                width_px: w,
                height_px: h,
                png,
            },
        });
        self.status = tr("status.bitmap_updated");
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
