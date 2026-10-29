//! Copies of the selected bitmap for the dialogs' previews (see
//! `before_after.rs`): a copy at most so many pixels across, made once
//! per source and size, and results kept until their settings change.

use crate::app::App;
use egui::{ColorImage, TextureHandle, TextureOptions};
use image::RgbaImage;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tracedraw_core::{document::ShapeKind, BitmapEffect, BitmapFxStack, ShapeId};

/// The largest side of the preview copy, pixels.
pub const THUMB: u32 = 240;

/// A copy of a bitmap for previews and its texture.
#[derive(Clone)]
pub struct Thumb {
    pub shape: ShapeId,
    /// What the copy was made from (see [`Source`]) and its size.
    pub key: u64,
    pub img: Arc<RgbaImage>,
    /// The copy's scale against the bitmap.
    pub scale: f32,
    pub tex: TextureHandle,
}

#[derive(Clone)]
struct After {
    key: u64,
    img: Arc<RgbaImage>,
    tex: TextureHandle,
}

fn color_image(img: &RgbaImage) -> ColorImage {
    ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw())
}

/// The first selected bitmap: its id, pixels per inch and PNG bytes; else
/// the first selected object as a picture (what bitmap effects start
/// from: its picture with its effects when it has some, else itself).
pub fn selected_bitmap(app: &App) -> Option<(ShapeId, f32, tracedraw_core::SharedBytes)> {
    let shapes = app.selected_shapes();
    let bitmap = shapes.iter().find_map(|s| match &s.kind {
        ShapeKind::Bitmap {
            rect,
            width_px,
            png,
            ..
        } => {
            let mm = rect.width().abs().max(1e-6);
            Some((s.id, (*width_px as f64 / (mm / 25.4)) as f32, png.clone()))
        }
        _ => None,
    });
    bitmap.or_else(|| {
        let s = shapes.first()?;
        if let Some(fx) = s.raster_fx.as_ref().filter(|f| !f.png.is_empty()) {
            let mm = fx.rect.width().abs().max(1e-6);
            return Some((
                s.id,
                (fx.width_px as f64 / (mm / 25.4)) as f32,
                fx.png.clone(),
            ));
        }
        let (png, w, _, rect) = app.vector_base_cached(s)?;
        let mm = rect.width().abs().max(1e-6);
        Some((s.id, (w as f64 / (mm / 25.4)) as f32, png))
    })
}

/// What a preview starts from.
pub enum Source<'a> {
    /// The first selected bitmap as it shows now.
    Shown,
    /// A bitmap's original pixels through its visible effects before
    /// `index`: where the effect being edited starts.
    Stack {
        shape: ShapeId,
        stack: &'a BitmapFxStack,
        index: usize,
    },
}

/// A copy of `img` at most `side` pixels across and its scale.
fn shrink(img: RgbaImage, side: u32) -> (RgbaImage, f32) {
    let (w, h) = img.dimensions();
    let scale = (side as f32 / w.max(h).max(1) as f32).min(1.0);
    if scale >= 1.0 {
        return (img, 1.0);
    }
    let small = image::imageops::resize(
        &img,
        ((w as f32 * scale).round() as u32).max(1),
        ((h as f32 * scale).round() as u32).max(1),
        image::imageops::FilterType::Triangle,
    );
    (small, scale)
}

/// A copy of the source at most `side` pixels across, made once per
/// source and size.
pub fn copy_of(app: &App, ctx: &egui::Context, src: &Source, side: u32) -> Option<Thumb> {
    let cache = egui::Id::new(("bitmap_preview_thumb", side));
    let (shape, key, png, before): (ShapeId, u64, tracedraw_core::SharedBytes, &[BitmapEffect]) =
        match src {
            Source::Shown => {
                let (id, _, png) = selected_bitmap(app)?;
                let key = settings_key(&("shown", png.len(), side));
                (id, key, png, &[])
            }
            Source::Stack {
                shape,
                stack,
                index,
            } => {
                let before = &stack.effects[..(*index).min(stack.effects.len())];
                // With no effects before, the same copy as the bitmap
                // shown: the dialogs' live preview moves between the two
                // without making it again.
                let key = if before.is_empty() {
                    settings_key(&("shown", stack.png.len(), side))
                } else {
                    settings_key(&("stack", stack.png.len(), before, side))
                };
                (*shape, key, stack.png.clone(), before)
            }
        };
    if let Some(t) = ctx.data(|d| d.get_temp::<Thumb>(cache)) {
        if t.shape == shape && t.key == key {
            return Some(t);
        }
    }
    let (mut small, scale) = shrink(crate::bitmap_fx::decode(&png)?, side);
    for e in before.iter().filter(|e| e.visible) {
        small = crate::fx::apply_effect(&small, &crate::fx::scaled(e, scale));
    }
    let tex = ctx.load_texture(
        "bitmap_preview_before",
        color_image(&small),
        TextureOptions::LINEAR,
    );
    let t = Thumb {
        shape,
        key,
        img: Arc::new(small),
        scale,
        tex,
    };
    ctx.data_mut(|d| d.insert_temp(cache, t.clone()));
    Some(t)
}

/// An image and its texture kept under `name` until `key` changes;
/// `make` runs only then.
pub fn cached(
    ctx: &egui::Context,
    name: &str,
    key: u64,
    make: impl FnOnce() -> RgbaImage,
) -> (TextureHandle, Arc<RgbaImage>) {
    let id = egui::Id::new(("bitmap_preview_cache", name));
    if let Some(a) = ctx.data(|d| d.get_temp::<After>(id)) {
        if a.key == key {
            return (a.tex, a.img);
        }
    }
    let img = Arc::new(make());
    let tex = ctx.load_texture(name, color_image(&img), TextureOptions::LINEAR);
    ctx.data_mut(|d| {
        d.insert_temp(
            id,
            After {
                key,
                img: img.clone(),
                tex: tex.clone(),
            },
        )
    });
    (tex, img)
}

/// A hash of anything printable: the settings a preview depends on.
pub fn settings_key(v: &impl std::fmt::Debug) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    format!("{v:?}").hash(&mut h);
    h.finish()
}
