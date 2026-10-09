//! Drawing the page, objects, selection handles, nodes, rulers and grid with
//! the egui painter. Good enough for alpha; a real renderer comes later.

use crate::app::{App, Drag, Handle};
use crate::theme::Tokens;
use crate::tools::Tool;
use crate::view::View;
use egui::{epaint, Color32, Painter, Pos2, Rect as ERect, Stroke as EStroke};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, BezPath, PathEl, Point, Rect},
    Color, Document,
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
    if app.show_page_border {
        painter.rect_stroke(
            paper,
            0.0,
            EStroke::new(1.0, Tokens::PAGE_BORDER),
            epaint::StrokeKind::Outside,
        );
    }
    if app.show_bleed {
        let b = app.doc().metadata.bleed.max(0.0);
        if b > 0.0 {
            let r = view.rect_to_screen(app.page_rect().inflate(b, b));
            painter.rect_stroke(
                r,
                0.0,
                EStroke::new(1.0, Tokens::PAGE_BORDER),
                epaint::StrokeKind::Outside,
            );
        }
    }
    if app.show_printable_area {
        // Printable area: the page minus the printer's typical 5 mm hardware margin.
        let r = view.rect_to_screen(app.page_rect().inflate(-5.0, -5.0));
        let dash = 4.0;
        for (a, b) in [
            (r.left_top(), r.right_top()),
            (r.right_top(), r.right_bottom()),
            (r.right_bottom(), r.left_bottom()),
            (r.left_bottom(), r.left_top()),
        ] {
            let len = (b - a).length();
            let n = (len / (dash * 2.0)).floor() as usize;
            for k in 0..n {
                let t0 = (k as f32 * dash * 2.0) / len;
                let t1 = ((k as f32 * dash * 2.0) + dash) / len;
                painter.line_segment(
                    [a + (b - a) * t0, a + (b - a) * t1],
                    EStroke::new(1.0, Tokens::TEXT_DIM),
                );
            }
        }
    }

    if app.show_grid {
        draw_grid(painter, rect, view);
    }
    if app.show_baseline_grid {
        // Horizontal lines from the page top downwards, inside the page only.
        let step = app.settings.baseline_grid_mm.max(0.1);
        let page = app.page_rect();
        let mut y = page.y1 - step;
        while y > page.y0 {
            let sy = view.to_screen(Point::new(0.0, y)).y;
            if sy >= rect.top() && sy <= rect.bottom() {
                painter.hline(
                    paper.x_range(),
                    sy,
                    EStroke::new(0.5, Color32::from_rgb(170, 200, 230)),
                );
            }
            y -= step;
        }
    }

    // Objects, rasterized with tiny-skia into a texture; the live drag preview
    // is applied to the selection during the render.
    let doc: &Document = app.doc();
    let preview = match &app.drag {
        Drag::Move { total, .. } if total.hypot() > 0.0 => Some(Affine::translate(*total)),
        d @ (Drag::Scale { .. } | Drag::Rotate { .. }) => app.preview_transform_of(d),
        _ => None,
    };
    if let Some(tex) = app.raster.borrow_mut().texture(
        painter.ctx(),
        doc,
        app.page,
        view,
        rect,
        preview.map(|t| (app.selection.clone(), t)),
        app.engine.revision(),
        app.wireframe,
        app.simulate_overprints,
        app.rasterize_complex_effects,
    ) {
        painter.image(
            tex,
            rect,
            ERect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    }

    // Selection.
    draw_selection(app, painter, preview);
    draw_table_cells(app, painter);
    draw_frame_links(app, painter);
    draw_symmetry_lines(app, painter, rect);
    draw_anchors(app, painter);

    // 3-point tools: base segment waiting for the third click, previewed to the pointer.
    if let (Some((a, b)), Some(c)) = (app.three_point_base, app.pointer_page) {
        let stroke = EStroke::new(1.0, Tokens::SELECTION);
        painter.line_segment([view.to_screen(a), view.to_screen(b)], stroke);
        let preview = three_point_preview(app.tool, a, b, c);
        for (pts, closed) in flatten(&preview, view) {
            if closed {
                painter.add(epaint::PathShape::closed_line(pts, stroke));
            } else {
                painter.add(epaint::PathShape::line(pts, stroke));
            }
        }
    }

    // Rubber bands and in-progress tools.
    match &app.drag {
        Drag::Box {
            start,
            current,
            from_center,
        } => {
            let page_rect = App::box_rect(*start, *current, *from_center);
            let r = view.rect_to_screen(page_rect);
            let stroke = EStroke::new(1.0, Tokens::SELECTION);
            painter.rect_stroke(r, 0.0, stroke, epaint::StrokeKind::Outside);
            if matches!(app.tool, Tool::Ellipse | Tool::ThreePointEllipse) {
                let pts = flatten(&tracedraw_core::geometry::ellipse_path(page_rect), view);
                for (p, _) in pts {
                    painter.add(epaint::PathShape::closed_line(p, stroke));
                }
            }
        }
        Drag::Marquee { start, current } | Drag::ZoomBox { start, current } => {
            let r = view.rect_to_screen(Rect::from_points(*start, *current));
            let stroke = EStroke::new(1.0, Tokens::SELECTION);
            painter.rect_stroke(r, 0.0, stroke, epaint::StrokeKind::Outside);
        }
        Drag::ThreePointBase { start, current } => {
            painter.line_segment(
                [view.to_screen(*start), view.to_screen(*current)],
                EStroke::new(1.0, Tokens::SELECTION),
            );
        }
        Drag::Connector {
            from,
            start,
            current,
        } => {
            let origin = if app.tool == Tool::Blend {
                doc.shape(*from).ok().map(|(_, s)| s.bounds().center())
            } else {
                app.nearest_anchor(*from, *start)
            };
            if let Some(o) = origin {
                painter.line_segment(
                    [view.to_screen(o), view.to_screen(*current)],
                    EStroke::new(1.0, Tokens::SELECTION),
                );
            }
        }
        Drag::TextFrame { start, current } => {
            let r = view.rect_to_screen(Rect::from_points(*start, *current));
            painter.rect_stroke(
                r,
                0.0,
                EStroke::new(1.0, Tokens::TEXT),
                epaint::StrokeKind::Outside,
            );
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
        let preview = if c.dragging_handle {
            None
        } else {
            app.pointer_page
        };
        let path = c.path(preview);
        for (pts, _) in flatten(&path, view) {
            if pts.len() > 1 {
                painter.add(epaint::PathShape::line(
                    pts,
                    EStroke::new(1.0, Tokens::SELECTION),
                ));
            }
        }
        for (p, out) in &c.nodes {
            let sp = view.to_screen(*p);
            if let Some(o) = out {
                let so = view.to_screen(*o);
                let mirrored = sp - (so - sp);
                painter.line_segment([mirrored, so], EStroke::new(1.0, Tokens::SELECTION));
                painter.circle_filled(so, 3.0, Color32::WHITE);
                painter.circle_stroke(so, 3.0, EStroke::new(1.0, Tokens::SELECTION));
                painter.circle_filled(mirrored, 3.0, Color32::WHITE);
                painter.circle_stroke(mirrored, 3.0, EStroke::new(1.0, Tokens::SELECTION));
            }
            painter.rect_stroke(
                ERect::from_center_size(sp, egui::vec2(6.0, 6.0)),
                0.0,
                EStroke::new(1.0, Tokens::TEXT),
                epaint::StrokeKind::Middle,
            );
        }
    }
    for p in &app.dimension_points {
        let sp = view.to_screen(*p);
        painter.circle_stroke(sp, 4.0, EStroke::new(1.0, Tokens::SELECTION));
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

/// Liang-Barsky clip of a screen segment against a rectangle.
fn clip_segment(a: Pos2, b: Pos2, r: ERect) -> Option<(Pos2, Pos2)> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let mut t0 = 0.0f32;
    let mut t1 = 1.0f32;
    for (p, q) in [
        (-dx, a.x - r.left()),
        (dx, r.right() - a.x),
        (-dy, a.y - r.top()),
        (dy, r.bottom() - a.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 {
            if t > t1 {
                return None;
            }
            t0 = t0.max(t);
        } else {
            if t < t0 {
                return None;
            }
            t1 = t1.min(t);
        }
    }
    Some((
        Pos2::new(a.x + dx * t0, a.y + dy * t0),
        Pos2::new(a.x + dx * t1, a.y + dy * t1),
    ))
}

/// Outline of the shape a 3-point tool would create for base `a -> b` and third point `c`.
fn three_point_preview(
    tool: Tool,
    a: Point,
    b: Point,
    c: Point,
) -> tracedraw_core::geometry::BezPath {
    use tracedraw_core::geometry::{BezPath, Vec2};
    let base = b - a;
    let len = base.hypot().max(1e-6);
    let dir = base / len;
    let normal = Vec2::new(-dir.y, dir.x);
    let height = (c - a).dot(normal);
    match tool {
        Tool::ThreePointCurve => {
            let ctrl = Point::new(2.0 * c.x - 0.5 * (a.x + b.x), 2.0 * c.y - 0.5 * (a.y + b.y));
            let mut p = BezPath::new();
            p.move_to(a);
            p.curve_to(
                a + (ctrl - a) * (2.0 / 3.0),
                b + (ctrl - b) * (2.0 / 3.0),
                b,
            );
            p
        }
        _ => {
            let local = Rect::new(0.0, height.min(0.0), len, height.max(0.0));
            let t = Affine::translate(a.to_vec2()) * Affine::rotate(base.atan2());
            let p = if tool == Tool::ThreePointEllipse {
                tracedraw_core::geometry::ellipse_path(local)
            } else {
                tracedraw_core::geometry::rect_path(local, 0.0)
            };
            t * p
        }
    }
}

/// Dashed screen-space line (guides, mirror lines).
fn dash(painter: &Painter, a: Pos2, b: Pos2, color: Color32) {
    let len = a.distance(b);
    let n = (len / 6.0).ceil().max(1.0) as usize;
    for i in (0..n).step_by(2) {
        let t0 = i as f32 / n as f32;
        let t1 = ((i + 1) as f32 / n as f32).min(1.0);
        painter.line_segment(
            [a + (b - a) * t0, a + (b - a) * t1],
            EStroke::new(1.0, color),
        );
    }
}

/// Mirror lines of selected objects with a Symmetry effect (dashed, with a
/// centre handle), as the target design shows them in symmetry mode.
fn draw_symmetry_lines(app: &App, painter: &Painter, rect: ERect) {
    let view = &app.view;
    for s in app.selected_shapes() {
        for e in &s.effects {
            let tracedraw_core::live::Effect::Symmetry {
                center,
                angle,
                lines,
            } = e
            else {
                continue;
            };
            let n = (*lines).clamp(1, 12) as usize;
            let far = 100_000.0 / view.zoom.max(0.01) as f64;
            for k in 0..n {
                let a = (angle + 180.0 * k as f64 / n as f64).to_radians();
                let d = tracedraw_core::geometry::Vec2::new(a.cos(), a.sin());
                let s0 = view.to_screen(*center - d * far);
                let s1 = view.to_screen(*center + d * far);
                if let Some((c0, c1)) = clip_segment(s0, s1, rect) {
                    dash(painter, c0, c1, Color32::from_rgb(40, 110, 220));
                }
            }
            let c = view.to_screen(*center);
            painter.circle_stroke(c, 5.0, EStroke::new(1.5, Color32::from_rgb(40, 110, 220)));
        }
    }
}

/// Connector anchors: small diamonds on the selected object with the
/// Anchor Editing tool, and on both ends while a connector is dragged.
fn draw_anchors(app: &App, painter: &Painter) {
    let view = &app.view;
    let mut shown: Vec<(tracedraw_core::ShapeId, bool)> = Vec::new();
    match (&app.tool, &app.drag) {
        (Tool::AnchorEditing, _) => {
            if let Some(id) = app.selection.first() {
                shown.push((*id, true));
            }
        }
        (
            Tool::Connector | Tool::RightAngleConnector | Tool::RoundedConnector,
            Drag::Connector { from, current, .. },
        ) => {
            shown.push((*from, false));
            if let Some(to) = app.hit_test(*current).filter(|t| t != from) {
                shown.push((to, false));
            }
        }
        _ => {}
    }
    for (id, editing) in shown {
        let n_side = crate::anchors::SIDE_ANCHORS.len();
        for (i, p) in app.anchors_of(id).into_iter().enumerate() {
            let c = view.to_screen(p);
            let custom = i >= n_side;
            let r = if custom { 5.0 } else { 4.0 };
            let pts = vec![
                c + egui::vec2(0.0, -r),
                c + egui::vec2(r, 0.0),
                c + egui::vec2(0.0, r),
                c + egui::vec2(-r, 0.0),
            ];
            let selected = editing && custom && app.anchor_sel == Some(i - n_side);
            let fill = if selected {
                Tokens::SELECTION
            } else if custom {
                Color32::WHITE
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 160)
            };
            painter.add(epaint::PathShape::convex_polygon(
                pts,
                fill,
                EStroke::new(1.0, Tokens::SELECTION),
            ));
        }
    }
}

/// Linked paragraph frames: a blue connector from the bottom of a selected
/// frame to the top of the next one, like the target design shows.
fn draw_frame_links(app: &App, painter: &Painter) {
    let view = &app.view;
    let stroke = EStroke::new(1.0, Tokens::SELECTION);
    for id in &app.selection {
        for pair in app.chain_of(*id).windows(2) {
            let (Some(a), Some(b)) = (app.doc().find_shape(pair[0]), app.doc().find_shape(pair[1]))
            else {
                continue;
            };
            let (ba, bb) = (a.bounds(), b.bounds());
            let from = view.to_screen(Point::new(ba.x1, ba.y0));
            let to = view.to_screen(Point::new(bb.x0, bb.y1));
            painter.line_segment([from, to], stroke);
            painter.circle_filled(from, 3.0, Tokens::SELECTION);
            let d = (to - from).normalized();
            let n = egui::vec2(-d.y, d.x);
            painter.add(epaint::PathShape::convex_polygon(
                vec![to, to - d * 8.0 + n * 4.0, to - d * 8.0 - n * 4.0],
                Tokens::SELECTION,
                EStroke::NONE,
            ));
        }
    }
}

/// Active table cells (Table tool): a filled highlight over each selected cell.
fn draw_table_cells(app: &App, painter: &Painter) {
    let Some(e) = &app.table_edit else {
        return;
    };
    let Ok((_, s)) = app.doc().shape(e.shape) else {
        return;
    };
    let tracedraw_core::document::ShapeKind::Table(t) = &s.kind else {
        return;
    };
    let view = &app.view;
    for cell in &t.cells {
        if !e.cells.contains(&(cell.row, cell.col)) {
            continue;
        }
        let r = s.transform.transform_rect_bbox(t.cell_rect(cell));
        let sr = view.rect_to_screen(r);
        painter.rect_filled(sr, 0.0, Tokens::SELECTION.gamma_multiply(0.25));
        painter.rect_stroke(
            sr,
            0.0,
            EStroke::new(1.5, Tokens::SELECTION),
            epaint::StrokeKind::Inside,
        );
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
        draw_nodes(app, painter);
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
    // Hint "x" in the centre, the target design's move marker.
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

/// Shape tool overlay: nodes as squares (selected filled), handles of the
/// selected nodes as lines with round ends, start node larger.
fn draw_nodes(app: &App, painter: &Painter) {
    use tracedraw_core::nodes;
    let view = &app.view;
    for s in app.selected_shapes() {
        let ShapeKind::Path { path, .. } = &s.kind else {
            painter.rect_stroke(
                view.rect_to_screen(s.bounds()),
                0.0,
                EStroke::new(1.0, Tokens::SELECTION),
                epaint::StrokeKind::Outside,
            );
            continue;
        };
        for n in nodes::nodes(path) {
            let selected = app.node_selection.contains(&(s.id, n.index));
            let p = view.to_screen(s.transform * n.pos);
            if selected {
                for c in [n.ctrl_in, n.ctrl_out].into_iter().flatten() {
                    let cp = view.to_screen(s.transform * c);
                    painter.line_segment([p, cp], EStroke::new(1.0, Tokens::SELECTION));
                    painter.circle_filled(cp, 3.0, Color32::WHITE);
                    painter.circle_stroke(cp, 3.0, EStroke::new(1.0, Tokens::SELECTION));
                }
            }
            let size = if n.is_start { 8.0 } else { 6.0 };
            let r = ERect::from_center_size(p, egui::vec2(size, size));
            if selected {
                painter.rect_filled(r, 0.0, Tokens::HANDLE);
            } else {
                painter.rect_filled(r, 0.0, Color32::WHITE);
                painter.rect_stroke(
                    r,
                    0.0,
                    EStroke::new(1.0, Tokens::HANDLE),
                    epaint::StrokeKind::Middle,
                );
            }
        }
    }
    if let Drag::NodeMarquee { start, current } = &app.drag {
        let r = view.rect_to_screen(Rect::from_points(*start, *current));
        painter.rect_stroke(
            r,
            0.0,
            EStroke::new(1.0, Tokens::SELECTION),
            epaint::StrokeKind::Outside,
        );
    }
}

/// Guidelines: dashed lines across the window; the selected one in red,
/// a guide being dragged in blue.
pub fn draw_guides(app: &App, painter: &Painter, rect: ERect) {
    if !app.show_guides {
        return;
    }
    let view = &app.view;
    let Ok(page) = app.doc().page(app.page) else {
        return;
    };
    for (i, g) in page.guides.iter().enumerate() {
        let color = if app.selected_guide == Some(i) {
            Color32::from_rgb(220, 30, 30)
        } else {
            Color32::from_rgb(0, 120, 215)
        };
        match g {
            tracedraw_core::document::Guide::Horizontal { y } => {
                let sy = view.to_screen(Point::new(0.0, *y)).y;
                dash(
                    painter,
                    Pos2::new(rect.left(), sy),
                    Pos2::new(rect.right(), sy),
                    color,
                );
            }
            tracedraw_core::document::Guide::Vertical { x } => {
                let sx = view.to_screen(Point::new(*x, 0.0)).x;
                dash(
                    painter,
                    Pos2::new(sx, rect.top()),
                    Pos2::new(sx, rect.bottom()),
                    color,
                );
            }
            tracedraw_core::document::Guide::Angled { x, y, angle } => {
                // Extend far beyond the viewport in both directions.
                let a = angle.to_radians();
                let d = tracedraw_core::geometry::Vec2::new(a.cos(), a.sin());
                let far = 100_000.0 / view.zoom.max(0.01) as f64;
                let p0 = Point::new(*x, *y) - d * far;
                let p1 = Point::new(*x, *y) + d * far;
                let s0 = view.to_screen(p0);
                let s1 = view.to_screen(p1);
                if let Some((c0, c1)) = clip_segment(s0, s1, rect) {
                    dash(painter, c0, c1, color);
                }
            }
        }
    }
    if let Drag::NewGuide { horizontal, pos } = &app.drag {
        let s = view.to_screen(*pos);
        if *horizontal {
            dash(
                painter,
                Pos2::new(rect.left(), s.y),
                Pos2::new(rect.right(), s.y),
                Color32::from_rgb(0, 120, 215),
            );
        } else {
            dash(
                painter,
                Pos2::new(s.x, rect.top()),
                Pos2::new(s.x, rect.bottom()),
                Color32::from_rgb(0, 120, 215),
            );
        }
    }
}

/// Effect nodes (envelope, perspective, mesh) of the selection when the
/// Shape tool is active: small blue squares joined by a dotted frame.
pub fn draw_effect_nodes(app: &App, painter: &Painter) {
    if app.tool != crate::tools::Tool::Shape {
        return;
    }
    for s in app.selected_shapes() {
        let nodes = crate::interaction2::effect_nodes(&s);
        if nodes.is_empty() {
            continue;
        }
        let blue = egui::Color32::from_rgb(40, 110, 220);
        // Envelope and perspective frames.
        let env: Vec<egui::Pos2> = nodes
            .iter()
            .filter(|(i, _)| *i < 100)
            .map(|(_, p)| app.view.to_screen(*p))
            .collect();
        if env.len() == 8 {
            let mut pts = env.clone();
            pts.push(env[0]);
            painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, blue)));
        }
        let persp: Vec<egui::Pos2> = nodes
            .iter()
            .filter(|(i, _)| (100..200).contains(i))
            .map(|(_, p)| app.view.to_screen(*p))
            .collect();
        if persp.len() == 4 {
            let mut pts = persp.clone();
            pts.push(persp[0]);
            painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, blue)));
        }
        // Mesh grid lines.
        if let tracedraw_core::Fill::Mesh(m) = &s.fill {
            let cols = m.cols as usize + 1;
            for r in 0..=m.rows as usize {
                let pts: Vec<egui::Pos2> = (0..cols)
                    .filter_map(|c| m.nodes.get(r * cols + c))
                    .map(|n| app.view.to_screen(s.transform * n.pos))
                    .collect();
                painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, blue)));
            }
            for c in 0..cols {
                let pts: Vec<egui::Pos2> = (0..=m.rows as usize)
                    .filter_map(|r| m.nodes.get(r * cols + c))
                    .map(|n| app.view.to_screen(s.transform * n.pos))
                    .collect();
                painter.add(egui::Shape::line(pts, egui::Stroke::new(1.0, blue)));
            }
        }
        for (i, p) in nodes {
            let sp = app.view.to_screen(p);
            let r = egui::Rect::from_center_size(sp, egui::vec2(7.0, 7.0));
            let selected = app
                .effect_node_drag
                .map(|(id, k)| id == s.id && k == i)
                .unwrap_or(false)
                || app.selected_effect_node == Some((s.id, i));
            painter.rect_filled(r, 0.0, if selected { blue } else { egui::Color32::WHITE });
            painter.rect_stroke(
                r,
                0.0,
                egui::Stroke::new(1.0, blue),
                egui::StrokeKind::Outside,
            );
        }
    }
}
