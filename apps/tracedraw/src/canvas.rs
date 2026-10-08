//! Drawing the page, objects, selection handles, nodes, rulers and grid with
//! the egui painter. Good enough for alpha; a real renderer comes later.

use crate::app::{App, Drag, Handle};
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::view::View;
use egui::{epaint, Color32, Painter, Pos2, Rect as ERect, Stroke as EStroke};
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, BezPath, PathEl, Point, Rect},
    Color, Document, Fill,
};

pub fn to_color32(c: Color) -> Color32 {
    let [r, g, b] = c.to_rgb8();
    Color32::from_rgb(r, g, b)
}

pub fn flatten(path: &BezPath, view: &View) -> Vec<(Vec<Pos2>, bool)> {
    let mut out: Vec<(Vec<Pos2>, bool)> = Vec::new();
    let mut cur: Vec<Pos2> = Vec::new();
    let mut closed = false;
    let tol = 0.25 / view.zoom.max(0.05) as f64;
    kurbo::flatten(path.elements().iter().copied(), tol, &mut |el| match el {
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

/// Distance from a point to the nearest segment of a flattened path (mm).
pub fn distance_to_path(path: &BezPath, p: Point) -> f64 {
    let mut best = f64::MAX;
    let mut last: Option<Point> = None;
    let mut start: Option<Point> = None;
    kurbo::flatten(path.elements().iter().copied(), 0.05, &mut |el| match el {
        PathEl::MoveTo(q) => {
            last = Some(q);
            start = Some(q);
        }
        PathEl::LineTo(q) => {
            if let Some(a) = last {
                best = best.min(seg_dist(a, q, p));
            }
            last = Some(q);
        }
        PathEl::ClosePath => {
            if let (Some(a), Some(s)) = (last, start) {
                best = best.min(seg_dist(a, s, p));
            }
        }
        _ => {}
    });
    best
}

fn seg_dist(a: Point, b: Point, p: Point) -> f64 {
    let ab = b - a;
    let l2 = ab.hypot2();
    if l2 < 1e-12 {
        return (p - a).hypot();
    }
    let t = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
    (p - (a + ab * t)).hypot()
}

fn fill_color(fill: &Fill) -> Option<Color32> {
    match fill {
        Fill::None => None,
        Fill::Solid(c) => Some(to_color32(*c)),
        Fill::Linear { from, to, .. } | Fill::Radial { from, to, .. } => {
            let a = to_color32(*from);
            let b = to_color32(*to);
            Some(Color32::from_rgb(
                ((a.r() as u16 + b.r() as u16) / 2) as u8,
                ((a.g() as u16 + b.g() as u16) / 2) as u8,
                ((a.b() as u16 + b.b() as u16) / 2) as u8,
            ))
        }
    }
}

pub fn draw_shape(painter: &Painter, shape: &Shape, parent: Affine, view: &View) {
    if !shape.visible {
        return;
    }
    let transform = parent * shape.transform;
    match &shape.kind {
        ShapeKind::Group { children } => {
            for c in children {
                draw_shape(painter, c, transform, view);
            }
        }
        ShapeKind::Text { spans, origin } => {
            let size_pt = spans.first().map(|s| s.size_pt).unwrap_or(24.0);
            let px = (size_pt * 25.4 / 72.0 * view.zoom as f64) as f32;
            let text: String = spans.iter().map(|s| s.text.as_str()).collect();
            let color = fill_color(&shape.fill).unwrap_or(Color32::BLACK);
            let origin_page = transform * *origin;
            let pos = view.to_screen(origin_page);
            // Angle from the transform's x axis.
            let c = transform.as_coeffs();
            let angle = (-c[1]).atan2(c[0]) as f32;
            let galley =
                painter.layout_no_wrap(text, egui::FontId::proportional(px.max(1.0)), color);
            let h = galley.size().y;
            let mut shape = epaint::TextShape::new(pos - egui::vec2(0.0, h), galley, color);
            shape.angle = angle;
            painter.add(shape);
        }
        _ => {
            let path = transform * shape.local_path();
            let polys = flatten(&path, view);
            let fill = fill_color(&shape.fill);
            let stroke = shape.stroke.as_ref().map(|s| {
                let px = if s.width <= tracedraw_core::Stroke::HAIRLINE + 1e-9 {
                    1.0
                } else {
                    (s.width as f32 * view.zoom).max(0.75)
                };
                EStroke::new(px, to_color32(s.color))
            });
            for (pts, closed) in polys {
                if pts.len() < 2 {
                    continue;
                }
                if let Some(c) = fill {
                    if pts.len() >= 3 {
                        painter.add(epaint::PathShape::convex_polygon(
                            pts.clone(),
                            c,
                            EStroke::NONE,
                        ));
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
    }
}

pub fn draw_canvas(app: &App, painter: &Painter, rect: ERect) {
    let view = &app.view;
    painter.rect_filled(rect, 0.0, Tokens::DESKTOP);

    // Page.
    let paper = view.rect_to_screen(app.page_rect());
    painter.rect_filled(
        paper.translate(egui::vec2(3.0, 3.0)),
        0.0,
        Tokens::PAGE_SHADOW,
    );
    painter.rect_filled(paper, 0.0, Tokens::PAGE);
    painter.rect_stroke(
        paper,
        0.0,
        EStroke::new(1.0, Tokens::PAGE_BORDER),
        epaint::StrokeKind::Outside,
    );

    if app.show_grid {
        draw_grid(painter, rect, view);
    }

    // Objects, with the live preview of the drag applied to the selection.
    let doc: &Document = app.doc();
    let preview = match &app.drag {
        Drag::Move { total, .. } if total.hypot() > 0.0 => Some(Affine::translate(*total)),
        d @ (Drag::Scale { .. } | Drag::Rotate { .. }) => app.preview_transform_of(d),
        _ => None,
    };
    if let Ok(page) = doc.page(app.page) {
        for layer in &page.layers {
            if !layer.visible {
                continue;
            }
            for s in &layer.shapes {
                let t = match preview {
                    Some(t) if app.selection.contains(&s.id) => t,
                    _ => Affine::IDENTITY,
                };
                draw_shape(painter, s, t, view);
            }
        }
    }

    // Selection.
    draw_selection(app, painter, preview);

    // Rubber bands and in-progress tools.
    match &app.drag {
        Drag::Box { start, current }
        | Drag::Marquee { start, current }
        | Drag::ZoomBox { start, current } => {
            let r = view.rect_to_screen(Rect::from_points(*start, *current));
            let stroke = EStroke::new(1.0, Tokens::SELECTION);
            painter.rect_stroke(r, 0.0, stroke, epaint::StrokeKind::Outside);
            if matches!(app.drag, Drag::Box { .. })
                && matches!(app.tool, Tool::Ellipse | Tool::ThreePointEllipse)
            {
                let pts = flatten(
                    &tracedraw_core::geometry::ellipse_path(Rect::from_points(*start, *current)),
                    view,
                );
                for (p, _) in pts {
                    painter.add(epaint::PathShape::closed_line(p, stroke));
                }
            }
        }
        Drag::Freehand { points } => {
            let pts: Vec<Pos2> = points.iter().map(|p| view.to_screen(*p)).collect();
            if pts.len() > 1 {
                painter.add(epaint::PathShape::line(
                    pts,
                    EStroke::new(1.0, Tokens::TEXT),
                ));
            }
        }
        Drag::FillGradient { start, current, .. } => {
            painter.line_segment(
                [view.to_screen(*start), view.to_screen(*current)],
                EStroke::new(1.0, Tokens::SELECTION),
            );
            painter.rect_filled(
                ERect::from_center_size(view.to_screen(*start), egui::vec2(8.0, 8.0)),
                0.0,
                Color32::BLACK,
            );
            painter.rect_filled(
                ERect::from_center_size(view.to_screen(*current), egui::vec2(8.0, 8.0)),
                0.0,
                Color32::WHITE,
            );
        }
        _ => {}
    }
    if let Some(c) = &app.curve {
        let mut pts: Vec<Pos2> = c.points.iter().map(|p| view.to_screen(*p)).collect();
        if let Some(h) = app.pointer_page {
            pts.push(view.to_screen(h));
        }
        if pts.len() > 1 {
            painter.add(epaint::PathShape::line(
                pts.clone(),
                EStroke::new(1.0, Tokens::SELECTION),
            ));
        }
        for p in pts {
            painter.rect_stroke(
                ERect::from_center_size(p, egui::vec2(6.0, 6.0)),
                0.0,
                EStroke::new(1.0, Tokens::TEXT),
                epaint::StrokeKind::Middle,
            );
        }
    }
    // Text caret.
    if let Some(te) = &app.text_edit {
        if let Ok((_, s)) = doc.shape(te.shape) {
            let b = view.rect_to_screen(s.bounds());
            let x = b.right();
            painter.line_segment(
                [Pos2::new(x, b.top()), Pos2::new(x, b.bottom())],
                EStroke::new(1.0, Tokens::TEXT),
            );
        }
    }
}

fn draw_selection(app: &App, painter: &Painter, preview: Option<Affine>) {
    let shapes = app.selected_shapes();
    if shapes.is_empty() {
        return;
    }
    let view = &app.view;
    let bounds = shapes
        .iter()
        .map(|s| {
            let mut s = s.clone();
            if let Some(t) = preview {
                s.transform = t * s.transform;
            }
            s.bounds()
        })
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::ZERO);
    let r = view.rect_to_screen(bounds);

    if app.tool == Tool::Shape {
        // Nodes of the selected curves.
        for s in &shapes {
            if let ShapeKind::Path { path, .. } = &s.kind {
                let mut prev: Option<Point> = None;
                for el in path.elements() {
                    match el {
                        PathEl::MoveTo(q) | PathEl::LineTo(q) => {
                            let p = view.to_screen(s.transform * *q);
                            painter.rect_stroke(
                                ERect::from_center_size(p, egui::vec2(6.0, 6.0)),
                                0.0,
                                EStroke::new(1.0, Tokens::TEXT),
                                epaint::StrokeKind::Middle,
                            );
                            prev = Some(*q);
                        }
                        PathEl::CurveTo(c1, c2, q) => {
                            let p = view.to_screen(s.transform * *q);
                            // Control handles as thin lines.
                            if let Some(pp) = prev {
                                painter.line_segment(
                                    [
                                        view.to_screen(s.transform * pp),
                                        view.to_screen(s.transform * *c1),
                                    ],
                                    EStroke::new(0.5, Tokens::SELECTION),
                                );
                            }
                            painter.line_segment(
                                [p, view.to_screen(s.transform * *c2)],
                                EStroke::new(0.5, Tokens::SELECTION),
                            );
                            painter.rect_stroke(
                                ERect::from_center_size(p, egui::vec2(6.0, 6.0)),
                                0.0,
                                EStroke::new(1.0, Tokens::TEXT),
                                epaint::StrokeKind::Middle,
                            );
                            prev = Some(*q);
                        }
                        PathEl::QuadTo(_, q) => {
                            let p = view.to_screen(s.transform * *q);
                            painter.rect_stroke(
                                ERect::from_center_size(p, egui::vec2(6.0, 6.0)),
                                0.0,
                                EStroke::new(1.0, Tokens::TEXT),
                                epaint::StrokeKind::Middle,
                            );
                            prev = Some(*q);
                        }
                        PathEl::ClosePath => {}
                    }
                }
            } else {
                painter.rect_stroke(
                    view.rect_to_screen(s.bounds()),
                    0.0,
                    EStroke::new(1.0, Tokens::SELECTION),
                    epaint::StrokeKind::Outside,
                );
            }
        }
        return;
    }

    if !matches!(
        app.tool,
        Tool::Pick | Tool::FreeformPick | Tool::InteractiveFill | Tool::Text
    ) {
        painter.rect_stroke(
            r,
            0.0,
            EStroke::new(1.0, Tokens::SELECTION),
            epaint::StrokeKind::Outside,
        );
        return;
    }

    if app.rotate_mode {
        // Rotation arrows at corners, skew arrows at edges, centre pivot.
        let c = r.center();
        painter.circle_stroke(c, 5.0, EStroke::new(1.0, Tokens::HANDLE));
        painter.line_segment(
            [c - egui::vec2(8.0, 0.0), c + egui::vec2(8.0, 0.0)],
            EStroke::new(1.0, Tokens::HANDLE),
        );
        painter.line_segment(
            [c - egui::vec2(0.0, 8.0), c + egui::vec2(0.0, 8.0)],
            EStroke::new(1.0, Tokens::HANDLE),
        );
        for (h, p) in app.handle_positions(bounds) {
            draw_rotate_handle(painter, p, h, r);
        }
    } else {
        for (_, p) in app.handle_positions(bounds) {
            painter.rect_filled(
                ERect::from_center_size(p, egui::vec2(7.0, 7.0)),
                0.0,
                Tokens::HANDLE,
            );
        }
        if shapes.len() > 1 {
            painter.rect_stroke(
                r,
                0.0,
                EStroke::new(1.0, Tokens::SELECTION),
                epaint::StrokeKind::Outside,
            );
        }
    }
    // Hint "x" in the centre, the the editor move marker.
    let c = r.center();
    painter.line_segment(
        [c - egui::vec2(4.0, 4.0), c + egui::vec2(4.0, 4.0)],
        EStroke::new(1.0, Tokens::HANDLE),
    );
    painter.line_segment(
        [c - egui::vec2(4.0, -4.0), c + egui::vec2(4.0, -4.0)],
        EStroke::new(1.0, Tokens::HANDLE),
    );
}

fn draw_rotate_handle(painter: &Painter, p: Pos2, h: Handle, r: ERect) {
    let s = EStroke::new(1.5, Tokens::HANDLE);
    if h.is_corner() {
        // Curved double arrow approximated by a short arc.
        let c = r.center();
        let dir = (p - c).normalized();
        let n = egui::vec2(-dir.y, dir.x);
        let a = p + n * 7.0 - dir * 2.0;
        let b = p - n * 7.0 - dir * 2.0;
        painter.add(epaint::PathShape::line(vec![a, p + dir * 2.0, b], s));
        painter.line_segment([a, a + dir * 3.0], s);
        painter.line_segment([b, b + dir * 3.0], s);
    } else {
        let horizontal = matches!(h, Handle::N | Handle::S);
        let d = if horizontal {
            egui::vec2(7.0, 0.0)
        } else {
            egui::vec2(0.0, 7.0)
        };
        painter.line_segment([p - d, p + d], s);
        let t = if horizontal {
            egui::vec2(0.0, 3.0)
        } else {
            egui::vec2(3.0, 0.0)
        };
        painter.line_segment([p + d, p + d * 0.6 + t], s);
        painter.line_segment([p - d, p - d * 0.6 - t], s);
    }
}

fn draw_grid(painter: &Painter, rect: ERect, view: &View) {
    // 10 mm grid, lighter 5 mm subdivision when zoomed in.
    let step = if view.zoom > 6.0 { 5.0 } else { 10.0 };
    let tl = view.to_page(rect.left_top());
    let br = view.to_page(rect.right_bottom());
    let x0 = (tl.x / step).floor() * step;
    let y0 = (br.y / step).floor() * step;
    let mut x = x0;
    while x <= br.x {
        let sx = view.to_screen(Point::new(x, 0.0)).x;
        painter.vline(
            sx,
            rect.y_range(),
            EStroke::new(0.5, Color32::from_gray(190)),
        );
        x += step;
    }
    let mut y = y0;
    while y <= tl.y {
        let sy = view.to_screen(Point::new(0.0, y)).y;
        painter.hline(
            rect.x_range(),
            sy,
            EStroke::new(0.5, Color32::from_gray(190)),
        );
        y += step;
    }
}

/// Horizontal and vertical rulers, in the document units.
pub fn draw_rulers(app: &App, painter: &Painter, top: ERect, left: ERect) {
    let view = &app.view;
    painter.rect_filled(top, 0.0, Tokens::RULER_BG);
    painter.rect_filled(left, 0.0, Tokens::RULER_BG);
    painter.hline(
        top.x_range(),
        top.bottom() - 0.5,
        EStroke::new(1.0, Tokens::BORDER),
    );
    painter.vline(
        left.right() - 0.5,
        left.y_range(),
        EStroke::new(1.0, Tokens::BORDER),
    );

    let unit_mm = app.units.mm();
    let px_per_unit = view.zoom as f64 * unit_mm;
    // Choose a major step so labels are at least ~60 px apart.
    let candidates = [
        1.0, 2.0, 5.0, 10.0, 20.0, 25.0, 50.0, 100.0, 200.0, 500.0, 1000.0,
    ];
    let major = candidates
        .iter()
        .copied()
        .find(|c| c * px_per_unit >= 60.0)
        .unwrap_or(1000.0);
    let minor = major / 10.0;
    let font = egui::FontId::proportional(9.0);

    // Horizontal.
    let a = app.units.from_mm(view.to_page(top.left_top()).x);
    let b = app.units.from_mm(view.to_page(top.right_top()).x);
    let mut v = (a / minor).floor() * minor;
    while v <= b {
        let x = view.to_screen(Point::new(app.units.to_mm(v), 0.0)).x;
        let is_major = ((v / major).round() * major - v).abs() < minor * 0.01;
        let h = if is_major {
            top.height() * 0.55
        } else {
            top.height() * 0.25
        };
        painter.vline(
            x,
            (top.bottom() - h)..=top.bottom(),
            EStroke::new(1.0, Tokens::RULER_TICK),
        );
        if is_major {
            painter.text(
                Pos2::new(x + 2.0, top.top() + 1.0),
                egui::Align2::LEFT_TOP,
                fmt_tick(v),
                font.clone(),
                Tokens::TEXT_DIM,
            );
        }
        v += minor;
    }
    // Vertical (Y up: larger values toward the top).
    let a = app.units.from_mm(view.to_page(left.left_bottom()).y);
    let b = app.units.from_mm(view.to_page(left.left_top()).y);
    let mut v = (a / minor).floor() * minor;
    while v <= b {
        let y = view.to_screen(Point::new(0.0, app.units.to_mm(v))).y;
        let is_major = ((v / major).round() * major - v).abs() < minor * 0.01;
        let w = if is_major {
            left.width() * 0.55
        } else {
            left.width() * 0.25
        };
        painter.hline(
            (left.right() - w)..=left.right(),
            y,
            EStroke::new(1.0, Tokens::RULER_TICK),
        );
        if is_major {
            let galley = painter.layout_no_wrap(fmt_tick(v), font.clone(), Tokens::TEXT_DIM);
            let mut ts = epaint::TextShape::new(
                Pos2::new(left.left() + 1.0, y - 2.0),
                galley,
                Tokens::TEXT_DIM,
            );
            ts.angle = -std::f32::consts::FRAC_PI_2;
            painter.add(ts);
        }
        v += minor;
    }
    // Pointer position markers.
    if let Some(p) = app.pointer_page {
        let s = view.to_screen(p);
        painter.vline(s.x, top.y_range(), EStroke::new(1.0, Tokens::SELECTION));
        painter.hline(left.x_range(), s.y, EStroke::new(1.0, Tokens::SELECTION));
    }
}

fn fmt_tick(v: f64) -> String {
    if (v - v.round()).abs() < 1e-6 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}
