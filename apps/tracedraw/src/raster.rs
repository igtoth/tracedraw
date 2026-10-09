//! Keeps the rasterized page as an egui texture, re-rendering only when the
//! document, view, canvas size or drag preview changed.

use crate::view::View;
use egui::{ColorImage, Context, Rect, TextureHandle, TextureId, TextureOptions};
use tracedraw_core::{geometry::Affine, Document, PageId, ShapeId};
use tracedraw_render::{render_page, Preview, RenderOptions, ViewTransform};

#[derive(Default)]
pub struct Raster {
    texture: Option<TextureHandle>,
    key: Option<Key>,
}

#[derive(PartialEq, Clone)]
struct Key {
    revision: u64,
    page: PageId,
    zoom: f32,
    origin: egui::Pos2,
    size: (u32, u32),
    preview: Option<(Vec<ShapeId>, [f64; 6])>,
    wireframe: bool,
    simulate_overprints: bool,
    complex_effects: bool,
}

impl Raster {
    /// Drop the cached texture so the next frame re-renders.
    pub fn invalidate(&mut self) {
        self.key = None;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn texture(
        &mut self,
        ctx: &Context,
        doc: &Document,
        page: PageId,
        view: &View,
        rect: Rect,
        preview: Option<(Vec<ShapeId>, Affine)>,
        revision: u64,
        wireframe: bool,
        simulate_overprints: bool,
        complex_effects: bool,
    ) -> Option<TextureId> {
        let ppp = ctx.pixels_per_point();
        let w = (rect.width() * ppp).round().max(1.0) as u32;
        let h = (rect.height() * ppp).round().max(1.0) as u32;
        let key = Key {
            revision,
            page,
            zoom: view.zoom,
            origin: view.origin,
            size: (w, h),
            preview: preview.as_ref().map(|(s, t)| (s.clone(), t.as_coeffs())),
            wireframe,
            simulate_overprints,
            complex_effects,
        };
        if self.key.as_ref() == Some(&key) {
            if let Some(t) = &self.texture {
                return Some(t.id());
            }
        }
        let vt = ViewTransform {
            zoom: (view.zoom * ppp) as f64,
            origin_x: ((view.origin.x - rect.min.x) * ppp) as f64,
            origin_y: ((view.origin.y - rect.min.y) * ppp) as f64,
        };
        let opts = RenderOptions {
            width: w,
            height: h,
            view: vt,
            preview: preview.map(|(shapes, transform)| Preview { shapes, transform }),
            wireframe,
            simulate_overprints,
            complex_effects,
        };
        let pixmap = render_page(doc, page, &opts)?;
        let pixels: Vec<egui::Color32> = pixmap
            .pixels()
            .iter()
            .map(|p| {
                egui::Color32::from_rgba_premultiplied(p.red(), p.green(), p.blue(), p.alpha())
            })
            .collect();
        let image = ColorImage {
            size: [w as usize, h as usize],
            source_size: egui::vec2(w as f32, h as f32),
            pixels,
        };
        match &mut self.texture {
            Some(t) => t.set(image, TextureOptions::LINEAR),
            None => self.texture = Some(ctx.load_texture("page", image, TextureOptions::LINEAR)),
        }
        self.key = Some(key);
        self.texture.as_ref().map(|t| t.id())
    }
}
