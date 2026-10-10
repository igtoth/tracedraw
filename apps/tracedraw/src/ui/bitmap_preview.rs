//! Before and after previews for the bitmap dialogs (colour modes and
//! adjustments): a small copy of the selected bitmap and the dialog's
//! result on it, worked out again only when the settings change.

use crate::app::App;
use crate::i18n::tr;
use egui::{ColorImage, TextureHandle, TextureOptions, Ui};
use image::RgbaImage;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tracedraw_core::{document::ShapeKind, ShapeId};

/// The largest side of the preview copy, pixels.
pub const THUMB: u32 = 240;

#[derive(Clone)]
struct Thumb {
    shape: ShapeId,
    len: usize,
    img: Arc<RgbaImage>,
    /// The copy's scale against the bitmap.
    scale: f32,
    tex: TextureHandle,
}

#[derive(Clone)]
struct After {
    key: u64,
    tex: TextureHandle,
}

fn color_image(img: &RgbaImage) -> ColorImage {
    ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw())
}

/// The first selected bitmap: its id, pixels per inch and PNG bytes.
pub fn selected_bitmap(app: &App) -> Option<(ShapeId, f32, Vec<u8>)> {
    app.selected_shapes()
        .into_iter()
        .find_map(|s| match s.kind {
            ShapeKind::Bitmap {
                rect,
                width_px,
                png,
                ..
            } => {
                let mm = rect.width().abs().max(1e-6);
                Some((s.id, (width_px as f64 / (mm / 25.4)) as f32, png))
            }
            _ => None,
        })
}

/// The preview copy of the selected bitmap and its scale, decoded once.
fn thumb(app: &App, ctx: &egui::Context) -> Option<Thumb> {
    let (id, _, png) = selected_bitmap(app)?;
    let key = egui::Id::new("bitmap_preview_thumb");
    if let Some(t) = ctx.data(|d| d.get_temp::<Thumb>(key)) {
        if t.shape == id && t.len == png.len() {
            return Some(t);
        }
    }
    let img = crate::bitmap_fx::decode(&png)?;
    let (w, h) = img.dimensions();
    let scale = (THUMB as f32 / w.max(h).max(1) as f32).min(1.0);
    let small = if scale < 1.0 {
        image::imageops::resize(
            &img,
            ((w as f32 * scale).round() as u32).max(1),
            ((h as f32 * scale).round() as u32).max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let tex = ctx.load_texture(
        "bitmap_preview_before",
        color_image(&small),
        TextureOptions::LINEAR,
    );
    let t = Thumb {
        shape: id,
        len: png.len(),
        img: Arc::new(small),
        scale,
        tex,
    };
    ctx.data_mut(|d| d.insert_temp(key, t.clone()));
    Some(t)
}

/// A hash of anything printable: the settings a preview depends on.
pub fn settings_key(v: &impl std::fmt::Debug) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    format!("{v:?}").hash(&mut h);
    h.finish()
}

/// Draw the before and after previews side by side, each in a `side` px
/// square; `f` gets the copy and its scale against the bitmap (so pixel
/// sizes can follow) and runs when `key` changes. Returns the copy, for
/// histograms and eyedroppers.
pub fn before_after(
    ui: &mut Ui,
    app: &App,
    key: u64,
    side: f32,
    f: impl FnOnce(&RgbaImage, f32) -> RgbaImage,
) -> Option<Arc<RgbaImage>> {
    let ctx = ui.ctx().clone();
    let Some(t) = thumb(app, &ctx) else {
        ui.label(tr("dialog.no_bitmap_preview"));
        return None;
    };
    let after_id = egui::Id::new("bitmap_preview_after");
    let full_key = key ^ (t.shape.raw().rotate_left(17)) ^ (t.len as u64).rotate_left(41);
    let after = match ctx.data(|d| d.get_temp::<After>(after_id)) {
        Some(a) if a.key == full_key => a.tex,
        _ => {
            let out = f(&t.img, t.scale);
            let tex = ctx.load_texture(
                "bitmap_preview_after",
                color_image(&out),
                TextureOptions::LINEAR,
            );
            ctx.data_mut(|d| {
                d.insert_temp(
                    after_id,
                    After {
                        key: full_key,
                        tex: tex.clone(),
                    },
                )
            });
            tex
        }
    };
    let fit = |tex: &TextureHandle| {
        let s = tex.size_vec2();
        let k = (side / s.x.max(s.y).max(1.0)).min(1.0);
        s * k
    };
    ui.horizontal(|ui| {
        for (label, tex) in [(tr("dialog.before"), &t.tex), (tr("dialog.after"), &after)] {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label).size(11.0));
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
                // A checkerboard shows transparency.
                let painter = ui.painter_at(rect);
                let cell = 8.0;
                let (nx, ny) = ((side / cell).ceil() as i32, (side / cell).ceil() as i32);
                for j in 0..ny {
                    for i in 0..nx {
                        let c = if (i + j) % 2 == 0 { 0xFF } else { 0xE6 };
                        painter.rect_filled(
                            egui::Rect::from_min_size(
                                rect.min + egui::vec2(i as f32 * cell, j as f32 * cell),
                                egui::vec2(cell, cell),
                            ),
                            0.0,
                            egui::Color32::from_gray(c),
                        );
                    }
                }
                let size = fit(tex);
                let r = egui::Rect::from_center_size(rect.center(), size);
                painter.image(
                    tex.id(),
                    r,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
                painter.rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(1.0, crate::theme::Tokens::BORDER),
                    egui::StrokeKind::Inside,
                );
            });
        }
    });
    Some(t.img)
}
