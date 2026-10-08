//! Draw a document page with the egui painter.
//!
//! This is the "good enough for alpha" renderer: paths are flattened to
//! polylines and filled with egui's tessellator, which handles convex and
//! simple concave shapes. A proper scanline/GPU renderer replaces it later.

use crate::view::View;
use egui::{epaint, Color32, Painter, Pos2, Stroke as EStroke};
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, BezPath, PathEl},
    Color, Document, Fill, PageId, ShapeId,
};

const FLATTEN_TOLERANCE: f64 = 0.05; // mm

pub fn to_color32(c: Color) -> Color32 {
    let [r, g, b] = c.to_rgb8();
    Color32::from_rgb(r, g, b)
}

/// Flatten a path (already in page space) into screen-space polylines,
/// one per subpath, with a flag telling whether the subpath was closed.
pub fn flatten(path: &BezPath, view: &View) -> Vec<(Vec<Pos2>, bool)> {
    let mut out: Vec<(Vec<Pos2>, bool)> = Vec::new();
    let mut cur: Vec<Pos2> = Vec::new();
    let mut closed = false;
    let tol = FLATTEN_TOLERANCE * view.zoom.max(0.1) as f64;
    kurbo_flatten(path, tol / view.zoom as f64, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if !cur.is_empty() {
                out.push((std::mem::take(&mut cur), closed));
            }
            closed = false;
            cur.push(view.to_screen(p));
        }
        PathEl::LineTo(p) => cur.push(view.to_screen(p)),
        PathEl::ClosePath => closed = true,
        _ => {}
    });
    if !cur.is_empty() {
        out.push((cur, closed));
    }
    out
}

fn kurbo_flatten(path: &BezPath, tol: f64, f: &mut dyn FnMut(PathEl)) {
    kurbo::flatten(path.elements().iter().copied(), tol, f);
}

pub struct RenderOptions {
    pub selected: Vec<ShapeId>,
}

pub fn draw_page(painter: &Painter, doc: &Document, page: PageId, view: &View, opts: &RenderOptions) {
    let Ok(p) = doc.page(page) else { return };
    // Paper with a drop shadow.
    let paper = view.rect_to_screen(p.rect());
    painter.rect_filled(paper.translate(egui::vec2(4.0, 4.0)), 0.0, Color32::from_black_alpha(60));
    painter.rect_filled(paper, 0.0, Color32::WHITE);
    painter.rect_stroke(paper, 0.0, EStroke::new(1.0, Color32::from_gray(120)), epaint::StrokeKind::Outside);

    for layer in &p.layers {
        if !layer.visible {
            continue;
        }
        for shape in &layer.shapes {
            draw_shape(painter, shape, Affine::IDENTITY, view);
        }
    }

    // Selection handles: classic eight black squares on the bounds.
    for id in &opts.selected {
        if let Ok((_, s)) = doc.shape(*id) {
            let b = view.rect_to_screen(s.bounds());
            painter.rect_stroke(b, 0.0, EStroke::new(1.0, Color32::from_rgb(0, 120, 215)), epaint::StrokeKind::Outside);
            let h = 3.5;
            let pts = [
                b.left_top(),
                b.center_top(),
                b.right_top(),
                b.left_center(),
                b.right_center(),
                b.left_bottom(),
                b.center_bottom(),
                b.right_bottom(),
            ];
            for pt in pts {
                painter.rect_filled(egui::Rect::from_center_size(pt, egui::vec2(2.0 * h, 2.0 * h)), 0.0, Color32::BLACK);
            }
        }
    }
}

pub fn draw_shape(painter: &Painter, shape: &Shape, parent: Affine, view: &View) {
    if !shape.visible {
        return;
    }
    let transform = parent * shape.transform;
    if let ShapeKind::Group { children } = &shape.kind {
        for c in children {
            draw_shape(painter, c, transform, view);
        }
        return;
    }
    let path = transform * shape.local_path();
    let polys = flatten(&path, view);

    let fill = match &shape.fill {
        Fill::None => None,
        Fill::Solid(c) => Some(to_color32(*c)),
        // Gradients: approximate with the mid colour until the GPU renderer.
        Fill::Linear { from, to, .. } | Fill::Radial { from, to, .. } => {
            let a = to_color32(*from);
            let b = to_color32(*to);
            Some(Color32::from_rgb(
                ((a.r() as u16 + b.r() as u16) / 2) as u8,
                ((a.g() as u16 + b.g() as u16) / 2) as u8,
                ((a.b() as u16 + b.b() as u16) / 2) as u8,
            ))
        }
    };
    let stroke = shape.stroke.as_ref().map(|s| {
        // Hairlines stay one pixel wide at any zoom.
        let px = if s.width <= tracedraw_core::Stroke::HAIRLINE + 1e-9 { 1.0 } else { (s.width as f32 * view.zoom).max(0.75) };
        EStroke::new(px, to_color32(s.color))
    });

    for (pts, closed) in polys {
        if pts.len() < 2 {
            continue;
        }
        if let Some(c) = fill {
            if pts.len() >= 3 {
                painter.add(epaint::PathShape::convex_polygon(pts.clone(), c, EStroke::NONE));
            }
        }
        if let Some(st) = stroke {
            if closed {
                painter.add(epaint::PathShape::closed_line(pts, st));
            } else {
                painter.add(epaint::PathShape::line(pts, st));
            }
        }
    }
}
