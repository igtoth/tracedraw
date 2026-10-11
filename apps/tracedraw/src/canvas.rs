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

/// The page on screen, snapped to whole pixels.
pub fn page_screen_rect(app: &App) -> ERect {
    let r = app.view.rect_to_screen(app.page_rect());
    ERect::from_min_max(r.min.round(), r.max.round())
}

/// True when a screen point is on the page border (within 3 px) or on
/// its shadow: double-clicking there opens the page size options.
pub fn on_page_frame(app: &App, s: Pos2) -> bool {
    let paper = page_screen_rect(app);
    let near_border = paper.expand(3.0).contains(s) && !paper.shrink(3.0).contains(s);
    let shadow = paper.expand(1.0).translate(Tokens::PAGE_SHADOW_OFFSET);
    near_border || (shadow.contains(s) && !paper.contains(s))
}

pub fn draw_canvas(app: &App, painter: &Painter, rect: ERect) {
    let view = &app.view;
    painter.rect_filled(rect, 0.0, app.desktop_color());

    // Page: a flat grey shadow offset right and down, the white page, a
    // one-pixel grey border (pixel-aligned so it stays crisp).
    let paper = page_screen_rect(app);
    if app.show_page_border {
        painter.rect_filled(
            paper.expand(1.0).translate(Tokens::PAGE_SHADOW_OFFSET),
            0.0,
            Tokens::PAGE_SHADOW,
        );
    }
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
        draw_grid(app, painter, rect);
    }
    if app.show_baseline_grid {
        draw_baseline_grid(app, painter, rect, paper);
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

    draw_pixel_grid(app, painter, rect);
    draw_empty_clip_frames(app, painter);

    // Selection.
    draw_selection(app, painter, preview);
    draw_table_cells(app, painter);
    draw_frame_links(app, painter);
    draw_symmetry_lines(app, painter, rect);
    draw_anchors(app, painter);
    draw_fountain_handles(app, painter);

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
        } => match app.creation_preview(*start, *current, *from_center) {
            // Shape tools: the shape itself follows the pointer.
            Some(path) => draw_creation_preview(app, painter, &path),
            None => {
                let page_rect = App::box_rect(*start, *current, *from_center);
                let r = view.rect_to_screen(page_rect);
                let stroke = EStroke::new(1.0, Tokens::SELECTION);
                painter.rect_stroke(r, 0.0, stroke, epaint::StrokeKind::Outside);
            }
        },
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
    // Text selection and caret.
    if app.text_edit.is_some() {
        if let Some((top, bottom, quads)) = app.text_caret_geometry() {
            for q in quads {
                let pts: Vec<Pos2> = q.iter().map(|p| view.to_screen(*p)).collect();
                painter.add(egui::Shape::convex_polygon(
                    pts,
                    Color32::from_rgba_unmultiplied(0x00, 0x78, 0xD7, 70),
                    EStroke::NONE,
                ));
            }
            painter.line_segment(
                [view.to_screen(top), view.to_screen(bottom)],
                EStroke::new(1.5, Tokens::TEXT),
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
/// The outline of the object being drawn, in the colour and width of the
/// default outline (at least one pixel wide; the selection colour when new
/// objects get no outline), so the preview looks like the result.
fn draw_creation_preview(app: &App, painter: &Painter, path: &BezPath) {
    let view = &app.view;
    let stroke = match &app.default_stroke {
        Some(s) => EStroke::new((s.width as f32 * view.zoom).max(1.0), to_color32(s.color)),
        None => EStroke::new(1.0, Tokens::SELECTION),
    };
    for (pts, closed) in flatten(path, view) {
        if pts.len() < 2 {
            continue;
        }
        if closed {
            painter.add(epaint::PathShape::closed_line(pts, stroke));
        } else {
            painter.add(epaint::PathShape::line(pts, stroke));
        }
    }
}

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
/// centre handle), shown in symmetry mode.
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
/// Interactive Fill: the fountain axis (dashed, between two square
/// nodes) or the centre node of a radial, conical or square fountain.
fn draw_fountain_handles(app: &App, painter: &Painter) {
    if app.tool != Tool::InteractiveFill {
        return;
    }
    let Some((_, f, b)) = app.selected_fountain() else {
        return;
    };
    let view = &app.view;
    let handles = crate::fill_tool::fountain_handles(&f, b);
    if handles.len() == 2 {
        let a = view.to_screen(handles[0].1);
        let z = view.to_screen(handles[1].1);
        // Dashed axis.
        let d = z - a;
        let len = d.length().max(1e-6);
        let step = 6.0;
        let mut t = 0.0;
        while t < len {
            let t1 = (t + step * 0.6).min(len);
            painter.line_segment(
                [a + d * (t / len), a + d * (t1 / len)],
                EStroke::new(1.0, Tokens::SELECTION),
            );
            t += step;
        }
    }
    for (h, p) in handles {
        let sp = view.to_screen(p);
        let color = match h {
            crate::fill_tool::FountainHandle::Start | crate::fill_tool::FountainHandle::Center => {
                f.first_color()
            }
            crate::fill_tool::FountainHandle::End => f.last_color(),
        };
        let [r, g, bl] = color.to_rgb8();
        painter.rect_filled(
            ERect::from_center_size(sp, egui::vec2(9.0, 9.0)),
            0.0,
            Color32::from_rgb(r, g, bl),
        );
        painter.rect_stroke(
            ERect::from_center_size(sp, egui::vec2(9.0, 9.0)),
            0.0,
            EStroke::new(1.0, Tokens::SELECTION),
            egui::StrokeKind::Outside,
        );
    }
}

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
/// frame to the top of the next one.
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
    // Caret and selection inside the active cell.
    if let Some((top, bottom, quads)) = app.cell_caret_geometry() {
        for q in quads {
            let pts: Vec<Pos2> = q.iter().map(|p| view.to_screen(*p)).collect();
            painter.add(egui::Shape::convex_polygon(
                pts,
                Color32::from_rgba_unmultiplied(0x00, 0x78, 0xD7, 70),
                EStroke::NONE,
            ));
        }
        painter.line_segment(
            [view.to_screen(top), view.to_screen(bottom)],
            EStroke::new(1.5, Tokens::TEXT),
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
    // Options > Display > Hide bounding box for curve tools.
    if app.settings.hide_bbox_curve_tools
        && matches!(
            app.tool,
            Tool::Freehand
                | Tool::TwoPointLine
                | Tool::Bezier
                | Tool::Pen
                | Tool::BSpline
                | Tool::Polyline
                | Tool::ThreePointCurve
                | Tool::ShapeRecognition
                | Tool::BrushStrokes
        )
    {
        return;
    }

    // Every tool shows the selection handles and the centre marker right
    // after a shape is drawn; only the Pick tools switch to the rotate and
    // skew arrows.
    let pick = matches!(
        app.tool,
        Tool::Pick | Tool::FreeformPick | Tool::InteractiveFill | Tool::Text
    );
    if app.rotate_mode && pick {
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
    // Hint "x" in the centre: the move marker.
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

/// Empty clip frames show two grey diagonals on screen (Options >
/// Clip Frames > Show lines in empty clip frames).
fn draw_empty_clip_frames(app: &App, painter: &Painter) {
    if !app.settings.clip_frame.empty_lines {
        return;
    }
    let Ok(page) = app.doc().page(app.page) else {
        return;
    };
    let stroke = EStroke::new(1.0, Color32::from_gray(160));
    for s in page
        .layers
        .iter()
        .filter(|l| l.visible)
        .flat_map(|l| &l.shapes)
    {
        if let ShapeKind::ClipFrame { contents, .. } = &s.kind {
            if contents.is_empty() && s.visible {
                let r = app.view.rect_to_screen(s.bounds());
                painter.line_segment([r.left_top(), r.right_bottom()], stroke);
                painter.line_segment([r.right_top(), r.left_bottom()], stroke);
            }
        }
    }
}

/// Document grid line colour (lines) and dot colour (dots).
const GRID_LINE: Color32 = Color32::from_gray(0xDC);
const GRID_DOT: Color32 = Color32::from_gray(0x8C);

/// How many grid spacings apart drawn lines are, so they stay at least
/// `min_px` screen pixels apart: 1, 2, 5, 10, 20, 50... spacings.
pub fn grid_thinning(spacing_px: f64, min_px: f64) -> f64 {
    if !(spacing_px.is_finite() && spacing_px > 0.0) {
        return f64::INFINITY;
    }
    for k in 0..12 {
        for m in [1.0, 2.0, 5.0] {
            let f = m * 10f64.powi(k);
            if spacing_px * f >= min_px {
                return f;
            }
        }
    }
    f64::INFINITY
}

/// Grid positions (mm) along one axis between `a` and `b`: multiples of
/// `step` counted from `origin`.
pub fn grid_positions(a: f64, b: f64, origin: f64, step: f64) -> Vec<f64> {
    let (a, b) = (a.min(b), a.max(b));
    if !(step.is_finite() && step > 0.0) {
        return Vec::new();
    }
    let first = ((a - origin) / step).floor() as i64;
    let last = ((b - origin) / step).ceil() as i64;
    if last - first > 5000 {
        return Vec::new();
    }
    (first..=last).map(|i| origin + i as f64 * step).collect()
}

/// The document grid over the whole drawing window, as lines or dots,
/// passing through the ruler origin. Lines are thinned out when the zoom
/// would crowd them.
fn draw_grid(app: &App, painter: &Painter, rect: ERect) {
    let view = &app.view;
    let meta = &app.doc().metadata;
    let (grid, rulers) = (meta.grid, meta.rulers);
    let zoom = view.zoom as f64;
    let dots = grid.display == tracedraw_core::document::GridDisplay::Dots;
    let min_px = if dots { 8.0 } else { 6.0 };
    let sx = grid.spacing_x.max(1e-6) * grid_thinning(grid.spacing_x * zoom, min_px);
    let sy = grid.spacing_y.max(1e-6) * grid_thinning(grid.spacing_y * zoom, min_px);
    let tl = view.to_page(rect.left_top());
    let br = view.to_page(rect.right_bottom());
    let xs: Vec<f32> = grid_positions(tl.x, br.x, rulers.origin_x, sx)
        .into_iter()
        .map(|x| view.to_screen(Point::new(x, 0.0)).x.round() + 0.5)
        .filter(|x| *x >= rect.left() && *x <= rect.right())
        .collect();
    let ys: Vec<f32> = grid_positions(br.y, tl.y, rulers.origin_y, sy)
        .into_iter()
        .map(|y| view.to_screen(Point::new(0.0, y)).y.round() + 0.5)
        .filter(|y| *y >= rect.top() && *y <= rect.bottom())
        .collect();
    let mut px = crate::ui::rulers::Pixels::default();
    if dots {
        if xs.len() * ys.len() > 200_000 {
            return;
        }
        for x in &xs {
            for y in &ys {
                px.dot(x.floor(), y.floor(), GRID_DOT);
            }
        }
    } else {
        for x in &xs {
            px.rect(vline_rect(*x, rect), GRID_LINE);
        }
        for y in &ys {
            px.rect(hline_rect(*y, rect), GRID_LINE);
        }
    }
    px.paint(painter);
}

/// A one-pixel vertical line through the pixel containing `x`, across `r`.
fn vline_rect(x: f32, r: ERect) -> ERect {
    let x = x.floor();
    ERect::from_min_max(Pos2::new(x, r.top()), Pos2::new(x + 1.0, r.bottom()))
}

/// A one-pixel horizontal line through the pixel containing `y`.
fn hline_rect(y: f32, r: ERect) -> ERect {
    let y = y.floor();
    ERect::from_min_max(Pos2::new(r.left(), y), Pos2::new(r.right(), y + 1.0))
}

/// The baseline grid: lines across the page like a ruled notebook, from
/// the "start from top" distance down, in the grid's colour.
fn draw_baseline_grid(app: &App, painter: &Painter, rect: ERect, paper: ERect) {
    let grid = app.doc().metadata.grid;
    if grid.baseline_spacing * (app.view.zoom as f64) < 2.0 {
        return;
    }
    let color = to_color32(grid.baseline_color);
    let mut px = crate::ui::rulers::Pixels::default();
    for y in app.baseline_ys() {
        let sy = app.view.to_screen(Point::new(0.0, y)).y.round() + 0.5;
        if sy >= rect.top() && sy <= rect.bottom() {
            px.rect(hline_rect(sy, paper.intersect(rect)), color);
        }
    }
    px.paint(painter);
}

/// Screen pixels per document pixel at the current zoom.
pub fn screen_px_per_doc_px(app: &App) -> f64 {
    app.view.zoom as f64 * 25.4 / app.document_dpi().max(1.0)
}

/// The pixel grid: one cell per document pixel, aligned with the page's
/// bottom-left corner, in the Pixels view (or with pixel rulers) from 800%
/// zoom.
fn draw_pixel_grid(app: &App, painter: &Painter, rect: ERect) {
    let pixel_mode =
        app.view_mode == crate::app::ViewMode::Pixels || app.units == crate::app::Units::Pixels;
    if !app.show_pixel_grid || !pixel_mode || screen_px_per_doc_px(app) < 7.999 {
        return;
    }
    let grid = app.doc().metadata.grid;
    let [r, g, b] = grid.pixel_color.to_rgb8();
    let alpha = (grid.pixel_opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    let color = Color32::from_rgba_unmultiplied(r, g, b, alpha);
    if alpha == 0 {
        return;
    }
    let step = 25.4 / app.document_dpi().max(1.0);
    let view = &app.view;
    let tl = view.to_page(rect.left_top());
    let br = view.to_page(rect.right_bottom());
    let mut px = crate::ui::rulers::Pixels::default();
    for x in grid_positions(tl.x, br.x, 0.0, step) {
        let sx = view.to_screen(Point::new(x, 0.0)).x.round() + 0.5;
        if sx >= rect.left() && sx <= rect.right() {
            px.rect(vline_rect(sx, rect), color);
        }
    }
    for y in grid_positions(br.y, tl.y, 0.0, step) {
        let sy = view.to_screen(Point::new(0.0, y)).y.round() + 0.5;
        if sy >= rect.top() && sy <= rect.bottom() {
            px.rect(hline_rect(sy, rect), color);
        }
    }
    px.paint(painter);
}

/// One node marker: `shape` of side `size` around `p`.
fn node_marker(
    painter: &Painter,
    p: Pos2,
    size: f32,
    shape: crate::settings::NodeShape,
    fill: Option<Color32>,
    outline: Color32,
) {
    use crate::settings::NodeShape;
    let h = size / 2.0;
    let stroke = EStroke::new(1.0, outline);
    match shape {
        NodeShape::Square => {
            let r = ERect::from_center_size(p, egui::vec2(size, size));
            if let Some(f) = fill {
                painter.rect_filled(r, 0.0, f);
            }
            painter.rect_stroke(r, 0.0, stroke, epaint::StrokeKind::Inside);
        }
        NodeShape::Circle => {
            if let Some(f) = fill {
                painter.circle_filled(p, h, f);
            }
            painter.circle_stroke(p, h - 0.5, stroke);
        }
        NodeShape::Diamond => {
            let pts = vec![
                p + egui::vec2(0.0, -h - 0.5),
                p + egui::vec2(h + 0.5, 0.0),
                p + egui::vec2(0.0, h + 0.5),
                p + egui::vec2(-h - 0.5, 0.0),
            ];
            painter.add(epaint::PathShape::convex_polygon(
                pts,
                fill.unwrap_or(Color32::TRANSPARENT),
                stroke,
            ));
        }
    }
}

/// A text's Interactive spacing arrow: a bar and an arrow pointing right
/// (character spacing) or down (line spacing), centred on `c`.
fn draw_spacing_arrow(painter: &Painter, c: Pos2, horizontal: bool, color: Color32) {
    let s = EStroke::new(1.5, color);
    let v = |x: f32, y: f32| {
        if horizontal {
            c + egui::vec2(x, y)
        } else {
            c + egui::vec2(y, x)
        }
    };
    painter.circle_filled(c, 8.0, Color32::from_white_alpha(200));
    painter.line_segment([v(-6.0, -4.0), v(-6.0, 4.0)], s);
    painter.line_segment([v(-6.0, 0.0), v(5.0, 0.0)], s);
    painter.add(epaint::PathShape::convex_polygon(
        vec![v(7.0, 0.0), v(3.0, -3.5), v(3.0, 3.5)],
        color,
        EStroke::NONE,
    ));
}

/// Shape tool overlay, as Options > Nodes and Handles sets it: nodes
/// shaped by type (cusp, smooth, symmetrical), the selected ones filled
/// with the main colour, the others white or hollow, the first node of a
/// curve larger, handles of the selected nodes, and an arrow in the
/// secondary colour showing the curve's direction.
fn draw_nodes(app: &App, painter: &Painter) {
    use tracedraw_core::nodes;
    let view = &app.view;
    let prefs = app.settings.nodes;
    let [r, g, b] = prefs.main_rgb;
    let main = Color32::from_rgb(r, g, b);
    let [r, g, b] = prefs.secondary_rgb;
    let secondary = Color32::from_rgb(r, g, b);
    let size = prefs.size.px();
    for s in app.selected_shapes() {
        // Text: a node per character and the two spacing arrows.
        if matches!(s.kind, ShapeKind::Text { .. }) {
            let chars = crate::text_nodes::char_nodes(&s);
            if !chars.is_empty() {
                for (c, p) in chars {
                    let chosen = app.node_selection.contains(&(s.id, c));
                    let r = ERect::from_center_size(view.to_screen(p), egui::vec2(6.0, 6.0));
                    painter.rect_filled(r, 0.0, if chosen { main } else { Color32::WHITE });
                    painter.rect_stroke(
                        r,
                        0.0,
                        EStroke::new(1.0, main),
                        epaint::StrokeKind::Inside,
                    );
                }
                let (h, v) = app.spacing_arrow_spots(&s);
                draw_spacing_arrow(painter, h, true, main);
                draw_spacing_arrow(painter, v, false, main);
                continue;
            }
        }
        let ShapeKind::Path { path, .. } = &s.kind else {
            // Rectangles, ellipses and polygons show their own nodes.
            let own = crate::kind_nodes::nodes(&s);
            if own.is_empty() {
                painter.rect_stroke(
                    view.rect_to_screen(s.bounds()),
                    0.0,
                    EStroke::new(1.0, Tokens::SELECTION),
                    epaint::StrokeKind::Outside,
                );
            }
            for (node, p) in own {
                let chosen = matches!(
                    node,
                    crate::kind_nodes::KindNode::Corner { corner, .. }
                        if app.rect_corner_selected == Some((s.id, corner))
                );
                let fill = if chosen {
                    Some(main)
                } else if prefs.unselected_filled {
                    Some(Color32::WHITE)
                } else {
                    None
                };
                node_marker(painter, view.to_screen(p), size, prefs.cusp, fill, main);
            }
            continue;
        };
        let all = nodes::nodes(path);
        if prefs.show_direction {
            // A small arrow head just after the first node, along the
            // first segment.
            if let Some(n0) = all.iter().find(|n| n.is_start) {
                let toward = n0
                    .ctrl_out
                    .or_else(|| all.iter().find(|n| !n.is_start).map(|n| n.pos));
                if let Some(t) = toward {
                    let a = view.to_screen(s.transform * n0.pos);
                    let bpt = view.to_screen(s.transform * t);
                    let d = bpt - a;
                    if d.length() > 1.0 {
                        let d = d.normalized();
                        let n = egui::vec2(-d.y, d.x);
                        let tip = a + d * (size + 9.0);
                        let base = a + d * (size + 3.0);
                        painter.add(epaint::PathShape::convex_polygon(
                            vec![tip, base + n * 3.5, base - n * 3.5],
                            secondary,
                            EStroke::NONE,
                        ));
                    }
                }
            }
        }
        for n in all {
            let selected = app.node_selection.contains(&(s.id, n.index));
            let p = view.to_screen(s.transform * n.pos);
            if selected {
                for c in [n.ctrl_in, n.ctrl_out].into_iter().flatten() {
                    let cp = view.to_screen(s.transform * c);
                    painter.line_segment([p, cp], EStroke::new(1.0, main));
                    painter.circle_filled(cp, 3.0, Color32::WHITE);
                    painter.circle_stroke(cp, 3.0, EStroke::new(1.0, main));
                }
            }
            let shape = match nodes::node_type(path, n.index) {
                nodes::NodeType::Cusp => prefs.cusp,
                nodes::NodeType::Smooth => prefs.smooth,
                nodes::NodeType::Symmetrical => prefs.symmetrical,
            };
            let side = if n.is_start { size + 2.0 } else { size };
            let fill = if selected {
                Some(main)
            } else if prefs.unselected_filled {
                Some(Color32::WHITE)
            } else {
                None
            };
            node_marker(painter, p, side, shape, fill, main);
        }
    }
    // Node transform handles around the selected nodes.
    let handles = app.node_transform_handles();
    if let (false, Some(b)) = (handles.is_empty(), app.selected_nodes_bounds()) {
        let r = view.rect_to_screen(b);
        let corners = [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
            r.left_top(),
        ];
        dashed_polyline(
            painter,
            &corners,
            EStroke::new(1.0, Tokens::SELECTION),
            4.0,
            3.0,
        );
        let stroke = EStroke::new(1.0, Tokens::HANDLE);
        for (h, p) in handles {
            let c = view.to_screen(p);
            match app.node_transform {
                crate::node_edit::NodeTransformMode::RotateSkew if h.is_corner() => {
                    painter.circle_filled(c, 3.5, Color32::WHITE);
                    painter.circle_stroke(c, 3.5, stroke);
                }
                crate::node_edit::NodeTransformMode::RotateSkew => {
                    // A double arrow along the side.
                    let d = if matches!(h, Handle::N | Handle::S) {
                        egui::vec2(5.0, 0.0)
                    } else {
                        egui::vec2(0.0, 5.0)
                    };
                    painter.line_segment([c - d, c + d], EStroke::new(1.5, Tokens::HANDLE));
                    painter.circle_filled(c, 1.5, Tokens::HANDLE);
                }
                _ => {
                    let rr = ERect::from_center_size(c, egui::vec2(6.0, 6.0));
                    painter.rect_filled(rr, 0.0, Tokens::HANDLE);
                }
            }
        }
        if app.node_transform == crate::node_edit::NodeTransformMode::RotateSkew {
            let c = view.to_screen(b.center());
            painter.circle_stroke(c, 4.0, stroke);
            painter.circle_filled(c, 1.2, Tokens::HANDLE);
        }
    }
    if let Drag::NodeLasso { points } = &app.drag {
        let pts: Vec<Pos2> = points.iter().map(|p| view.to_screen(*p)).collect();
        dashed_polyline(
            painter,
            &pts,
            EStroke::new(1.0, Tokens::SELECTION),
            4.0,
            3.0,
        );
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

/// A line of `pattern` (on and off lengths in pixels; empty for solid).
/// Horizontal and vertical lines are drawn on whole pixels.
fn pattern_line(painter: &Painter, a: Pos2, b: Pos2, color: Color32, pattern: &[f32]) {
    let stroke = EStroke::new(1.0, color);
    let len = a.distance(b);
    let axis = (a.x - b.x).abs() < 0.01 || (a.y - b.y).abs() < 0.01;
    if axis && len > 0.0 {
        let mut px = crate::ui::rulers::Pixels::default();
        let dir = (b - a) / len;
        let mut t = 0.0;
        let mut i = 0;
        while t < len && i < 40_000 {
            let l = if pattern.is_empty() {
                len
            } else {
                pattern[i % pattern.len()].max(0.5)
            };
            if i % 2 == 0 {
                let p0 = a + dir * t;
                let p1 = a + dir * (t + l).min(len);
                let r = ERect::from_two_pos(p0, p1);
                let r = if (a.x - b.x).abs() < 0.01 {
                    let x = r.min.x.floor();
                    ERect::from_min_max(Pos2::new(x, r.min.y), Pos2::new(x + 1.0, r.max.y))
                } else {
                    let y = r.min.y.floor();
                    ERect::from_min_max(Pos2::new(r.min.x, y), Pos2::new(r.max.x, y + 1.0))
                };
                px.rect(r, color);
            }
            t += l;
            i += 1;
        }
        px.paint(painter);
        return;
    }
    if pattern.is_empty() || len <= 0.0 {
        painter.line_segment([a, b], stroke);
        return;
    }
    let dir = (b - a) / len;
    let mut t = 0.0;
    let mut i = 0;
    let mut segments = 0;
    while t < len && segments < 20_000 {
        let l = pattern[i % pattern.len()].max(0.5);
        if i % 2 == 0 {
            painter.line_segment([a + dir * t, a + dir * (t + l).min(len)], stroke);
            segments += 1;
        }
        t += l;
        i += 1;
    }
}

/// Selected guidelines are red.
const GUIDE_SELECTED: Color32 = Color32::from_rgb(0xFF, 0x00, 0x00);

/// One guideline across the drawing window.
fn draw_guide_line(
    app: &App,
    painter: &Painter,
    rect: ERect,
    line: tracedraw_core::document::GuideLine,
    color: Color32,
    pattern: &[f32],
) {
    use tracedraw_core::document::GuideLine;
    let view = &app.view;
    match line {
        GuideLine::Horizontal { y } => {
            let sy = view.to_screen(Point::new(0.0, y)).y.round() + 0.5;
            if sy >= rect.top() && sy <= rect.bottom() {
                pattern_line(
                    painter,
                    Pos2::new(rect.left(), sy),
                    Pos2::new(rect.right(), sy),
                    color,
                    pattern,
                );
            }
        }
        GuideLine::Vertical { x } => {
            let sx = view.to_screen(Point::new(x, 0.0)).x.round() + 0.5;
            if sx >= rect.left() && sx <= rect.right() {
                pattern_line(
                    painter,
                    Pos2::new(sx, rect.top()),
                    Pos2::new(sx, rect.bottom()),
                    color,
                    pattern,
                );
            }
        }
        GuideLine::Angled { .. } => {
            // Extend far beyond the viewport in both directions.
            let (o, d) = line.point_and_direction();
            let far = 100_000.0 / view.zoom.max(0.01) as f64;
            let s0 = view.to_screen(o - d * far);
            let s1 = view.to_screen(o + d * far);
            if let Some((c0, c1)) = clip_segment(s0, s1, rect) {
                pattern_line(painter, c0, c1, color, pattern);
            }
        }
    }
}

/// Guidelines across the drawing window in their colour and style; the
/// selected ones red, one being dragged where it would go, the rotation
/// handles of a guideline in rotate mode.
pub fn draw_guides(app: &App, painter: &Painter, rect: ERect) {
    if !app.show_guides {
        return;
    }
    let Ok(page) = app.doc().page(app.page) else {
        return;
    };
    for (i, g) in page.guides.iter().enumerate() {
        let g = match &app.drag {
            Drag::MoveGuide { index, guide, .. } | Drag::RotateGuide { index, guide, .. }
                if *index == i =>
            {
                guide
            }
            _ => g,
        };
        let color = if app.selected_guides.contains(&i) {
            GUIDE_SELECTED
        } else {
            to_color32(app.guide_color(g))
        };
        draw_guide_line(app, painter, rect, g.line, color, g.style.pattern());
    }
    if let Drag::NewGuide { horizontal, pos } = &app.drag {
        use tracedraw_core::document::GuideLine;
        let line = if *horizontal {
            GuideLine::Horizontal { y: pos.y }
        } else {
            GuideLine::Vertical { x: pos.x }
        };
        let color = to_color32(app.doc().metadata.guides.color);
        draw_guide_line(
            app,
            painter,
            rect,
            line,
            color,
            tracedraw_core::document::GuideStyle::default().pattern(),
        );
    }
    if let Some((_, pivot, handles)) = app.guide_rotate_handles() {
        let c = app.view.to_screen(pivot);
        let s = EStroke::new(1.0, Tokens::HANDLE);
        painter.circle_filled(c, 4.0, Color32::WHITE);
        painter.circle_stroke(c, 4.0, s);
        painter.circle_filled(c, 1.2, Tokens::HANDLE);
        for h in handles {
            let p = app.view.to_screen(h);
            let d = (p - c).normalized();
            let n = egui::vec2(-d.y, d.x);
            // A curved double arrow across the line, as the skew and
            // rotate handles of objects.
            let a = p + n * 6.0 - d * 2.0;
            let b = p - n * 6.0 - d * 2.0;
            painter.add(epaint::PathShape::line(
                vec![a, p + d * 1.5, b],
                EStroke::new(1.5, Tokens::HANDLE),
            ));
            painter.line_segment([a, a + d * 3.0], s);
            painter.line_segment([b, b + d * 3.0], s);
        }
    }
}

/// Outlines a docker previews (the Corners docker's cut): dashed in the
/// selection colour over the drawing.
pub fn draw_docker_preview(app: &App, painter: &Painter) {
    let stroke = EStroke::new(1.0, Tokens::SELECTION);
    for path in &app.docker_preview {
        for (mut pts, closed) in flatten(path, &app.view) {
            if closed {
                if let Some(first) = pts.first().copied() {
                    pts.push(first);
                }
            }
            dashed_polyline(painter, &pts, stroke, 4.0, 3.0);
        }
    }
    // The marked point (the Coordinates docker's origin point): a blue
    // node.
    if let Some(p) = app.docker_preview_point {
        let c = app.view.to_screen(p);
        let r = ERect::from_center_size(c, egui::vec2(7.0, 7.0));
        painter.rect_filled(r, 0.0, Tokens::SELECTION);
        painter.rect_stroke(
            r,
            0.0,
            EStroke::new(1.0, Color32::WHITE),
            egui::StrokeKind::Outside,
        );
    }
}

/// A polyline drawn in dashes of `on` px with `off` px gaps.
fn dashed_polyline(painter: &Painter, pts: &[Pos2], stroke: EStroke, on: f32, off: f32) {
    let period = on + off;
    let mut walked = 0.0f32;
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = a.distance(b);
        if len <= 0.0 {
            continue;
        }
        let dir = (b - a) / len;
        let mut t = 0.0f32;
        while t < len {
            let phase = (walked + t) % period;
            if phase < on {
                let end = (t + (on - phase)).min(len);
                painter.line_segment([a + dir * t, a + dir * end], stroke);
                t = end;
            } else {
                t += period - phase;
            }
        }
        walked += len;
    }
}

/// The mark of a snapping mode, about 11 px across, centred on `c`.
pub fn draw_snap_glyph(
    painter: &Painter,
    c: Pos2,
    mode: crate::snap_points::SnapMode,
    color: Color32,
) {
    use crate::snap_points::SnapMode;
    let s = EStroke::new(1.5, color);
    let h = 5.0;
    let v = egui::vec2;
    match mode {
        SnapMode::Node => {
            painter.rect_stroke(
                ERect::from_center_size(c, v(2.0 * h - 1.0, 2.0 * h - 1.0)),
                0.0,
                s,
                epaint::StrokeKind::Middle,
            );
        }
        SnapMode::Intersection => {
            painter.line_segment([c + v(-h, -h), c + v(h, h)], s);
            painter.line_segment([c + v(-h, h), c + v(h, -h)], s);
        }
        SnapMode::Midpoint => {
            painter.add(epaint::PathShape::closed_line(
                vec![c + v(0.0, -h), c + v(h, h - 1.0), c + v(-h, h - 1.0)],
                s,
            ));
        }
        SnapMode::Quadrant => {
            painter.add(epaint::PathShape::closed_line(
                vec![c + v(0.0, -h), c + v(h, 0.0), c + v(0.0, h), c + v(-h, 0.0)],
                s,
            ));
        }
        SnapMode::Tangent => {
            painter.circle_stroke(c + v(0.0, 1.5), h - 1.5, s);
            painter.line_segment([c + v(-h, -h + 2.0), c + v(h, -h + 2.0)], s);
        }
        SnapMode::Perpendicular => {
            painter.line_segment([c + v(-h, h), c + v(h, h)], s);
            painter.line_segment([c + v(0.0, h), c + v(0.0, -h)], s);
        }
        SnapMode::Edge => {
            painter.add(epaint::PathShape::closed_line(
                vec![c + v(-h, -h), c + v(h, -h), c + v(-h, h), c + v(h, h)],
                s,
            ));
        }
        SnapMode::Center => {
            painter.circle_stroke(c, h, s);
            painter.circle_filled(c, 1.5, color);
        }
        SnapMode::TextBaseline => {
            // A "T" standing on its baseline.
            painter.line_segment([c + v(-h - 1.0, h), c + v(h + 1.0, h)], s);
            let thin = EStroke::new(1.0, color);
            painter.line_segment([c + v(-h + 1.0, -h), c + v(h - 1.0, -h)], thin);
            painter.line_segment([c + v(0.0, -h), c + v(0.0, h - 2.0)], thin);
        }
    }
}

/// The point the pointer snapped to: its mode's mark and, with screen
/// tips on, the mode's name beside it (Options > Snapping).
pub fn draw_snap_mark(app: &App, painter: &Painter) {
    let Some(t) = app.snap_mark.get() else {
        return;
    };
    if !app.settings.snap.show_marks {
        return;
    }
    let c = app.view.to_screen(t.point);
    let color = Tokens::SELECTION;
    draw_snap_glyph(painter, c, t.mode, color);
    if app.settings.snap.screen_tips {
        let font = egui::FontId::proportional(11.0);
        let galley = painter.layout_no_wrap(crate::i18n::tr(t.mode.key()), font, color);
        let at = c + egui::vec2(10.0, 8.0);
        let r = ERect::from_min_size(at, galley.size()).expand2(egui::vec2(3.0, 1.0));
        painter.rect_filled(r, 2.0, Color32::from_rgba_unmultiplied(255, 255, 255, 230));
        painter.rect_stroke(
            r,
            2.0,
            EStroke::new(1.0, Tokens::BORDER),
            epaint::StrokeKind::Inside,
        );
        painter.galley(at, galley, color);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_lines_thin_out_and_follow_the_origin() {
        // 10 mm at 2 px per mm is 20 px: every line.
        assert_eq!(grid_thinning(20.0, 6.0), 1.0);
        // At 0.5 px per mm, 5 px apart: every second line.
        assert_eq!(grid_thinning(5.0, 6.0), 2.0);
        assert_eq!(grid_thinning(1.0, 6.0), 10.0);
        assert!(grid_thinning(0.0, 6.0).is_infinite());
        // Lines pass through the origin.
        let xs = grid_positions(-12.0, 25.0, 3.0, 10.0);
        assert_eq!(xs, vec![-17.0, -7.0, 3.0, 13.0, 23.0, 33.0]);
        assert!(grid_positions(0.0, 1e9, 0.0, 1e-3).is_empty());
        assert!(grid_positions(0.0, 1.0, 0.0, 0.0).is_empty());
    }

    #[test]
    fn baselines_start_from_the_top_and_stop_at_the_bottom() {
        let mut app = App::headless();
        let page = app.page_rect();
        let ys = app.baseline_ys();
        let g = app.doc().metadata.grid;
        assert!((ys[0] - (page.y1 - g.baseline_start)).abs() < 1e-9);
        assert!((ys[0] - ys[1] - g.baseline_spacing).abs() < 1e-9);
        assert!(*ys.last().unwrap() >= page.y0);
        let mut m = app.doc().metadata.clone();
        m.grid.baseline_start = 0.0;
        app.run(tracedraw_core::Command::SetMetadata { metadata: m });
        assert!((app.baseline_ys()[0] - page.y1).abs() < 1e-9);
    }
}
