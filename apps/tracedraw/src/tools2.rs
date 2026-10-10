//! Second wave of tools: Contour, Crop, Knife, Spiral, Common Shapes,
//! Brush Strokes (calligraphic), Parallel Dimension, Connector, Table,
//! and Convert Outline To Object. These produce static results (the target design
//! keeps some of them live); see docs/parity.md.

use crate::app::{App, Drag};
use crate::i18n::tr;
use crate::tools::Tool;
use egui::{Modifiers, PointerButton, Response};
use tracedraw_core::{
    document::{Shape, ShapeKind, TextSpan},
    geometry::{Affine, BezPath, Point, Rect, Shape as _, Vec2},
    shaping, Color, Command, Fill, ShapeId, Stroke,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourDirection {
    Inside,
    Outside,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommonShape {
    RightArrow,
    Heart,
    Diamond,
    Banner,
    Callout,
    Cross,
    Lightning,
    Triangle,
}

impl CommonShape {
    pub const ALL: [CommonShape; 8] = [
        CommonShape::RightArrow,
        CommonShape::Heart,
        CommonShape::Diamond,
        CommonShape::Banner,
        CommonShape::Callout,
        CommonShape::Cross,
        CommonShape::Lightning,
        CommonShape::Triangle,
    ];
    pub fn name(self) -> String {
        tr(match self {
            CommonShape::RightArrow => "shape.arrow",
            CommonShape::Heart => "shape.heart",
            CommonShape::Diamond => "shape.diamond",
            CommonShape::Banner => "shape.banner",
            CommonShape::Callout => "shape.callout",
            CommonShape::Cross => "shape.cross",
            CommonShape::Lightning => "shape.lightning",
            CommonShape::Triangle => "shape.triangle",
        })
    }

    /// Unit-square outline (0..1), y up.
    pub fn unit_path(self) -> BezPath {
        let mut p = BezPath::new();
        let poly = |p: &mut BezPath, pts: &[(f64, f64)]| {
            for (i, (x, y)) in pts.iter().enumerate() {
                if i == 0 {
                    p.move_to((*x, *y));
                } else {
                    p.line_to((*x, *y));
                }
            }
            p.close_path();
        };
        match self {
            CommonShape::RightArrow => poly(
                &mut p,
                &[
                    (0.0, 0.3),
                    (0.6, 0.3),
                    (0.6, 0.0),
                    (1.0, 0.5),
                    (0.6, 1.0),
                    (0.6, 0.7),
                    (0.0, 0.7),
                ],
            ),
            CommonShape::Heart => {
                p.move_to((0.5, 0.0));
                p.curve_to((0.1, 0.35), (0.0, 0.55), (0.0, 0.72));
                p.curve_to((0.0, 0.95), (0.25, 1.05), (0.5, 0.78));
                p.curve_to((0.75, 1.05), (1.0, 0.95), (1.0, 0.72));
                p.curve_to((1.0, 0.55), (0.9, 0.35), (0.5, 0.0));
                p.close_path();
            }
            CommonShape::Diamond => poly(&mut p, &[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)]),
            CommonShape::Banner => poly(
                &mut p,
                &[
                    (0.0, 0.1),
                    (0.15, 0.5),
                    (0.0, 0.9),
                    (1.0, 0.9),
                    (0.85, 0.5),
                    (1.0, 0.1),
                ],
            ),
            CommonShape::Callout => {
                p.move_to((0.0, 0.25));
                p.line_to((0.0, 0.9));
                p.curve_to((0.0, 0.97), (0.03, 1.0), (0.1, 1.0));
                p.line_to((0.9, 1.0));
                p.curve_to((0.97, 1.0), (1.0, 0.97), (1.0, 0.9));
                p.line_to((1.0, 0.35));
                p.curve_to((1.0, 0.28), (0.97, 0.25), (0.9, 0.25));
                p.line_to((0.35, 0.25));
                p.line_to((0.15, 0.0));
                p.line_to((0.2, 0.25));
                p.line_to((0.1, 0.25));
                p.curve_to((0.03, 0.25), (0.0, 0.28), (0.0, 0.35));
                p.close_path();
            }
            CommonShape::Cross => poly(
                &mut p,
                &[
                    (0.35, 0.0),
                    (0.65, 0.0),
                    (0.65, 0.35),
                    (1.0, 0.35),
                    (1.0, 0.65),
                    (0.65, 0.65),
                    (0.65, 1.0),
                    (0.35, 1.0),
                    (0.35, 0.65),
                    (0.0, 0.65),
                    (0.0, 0.35),
                    (0.35, 0.35),
                ],
            ),
            CommonShape::Lightning => poly(
                &mut p,
                &[
                    (0.55, 1.0),
                    (0.15, 0.45),
                    (0.45, 0.45),
                    (0.3, 0.0),
                    (0.85, 0.6),
                    (0.55, 0.6),
                    (0.7, 1.0),
                ],
            ),
            CommonShape::Triangle => poly(&mut p, &[(0.5, 1.0), (1.0, 0.0), (0.0, 0.0)]),
        }
        p
    }
}

/// Archimedean or logarithmic spiral inscribed in `rect`.
pub fn spiral_path(rect: Rect, revolutions: u32, logarithmic: bool) -> BezPath {
    let c = rect.center();
    let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
    let turns = revolutions.max(1) as f64;
    let steps = (turns * 48.0) as usize;
    let mut pts = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let a = t * turns * std::f64::consts::TAU;
        let r = if logarithmic {
            (t * 4.0).exp() / 4.0f64.exp()
        } else {
            t
        };
        pts.push(Point::new(c.x + rx * r * a.cos(), c.y + ry * r * a.sin()));
    }
    tracedraw_core::geometry::smooth_path(&pts, false)
}

/// A grid of `rows` x `cols` cells covering `rect`: the outer frame and
/// the inner lines (previews of Graph Paper and Table).
pub fn grid_path(rect: Rect, rows: u32, cols: u32) -> BezPath {
    let mut p = rect.to_path(0.01);
    let (rows, cols) = (rows.max(1), cols.max(1));
    for c in 1..cols {
        let x = rect.x0 + rect.width() * c as f64 / cols as f64;
        p.move_to((x, rect.y0));
        p.line_to((x, rect.y1));
    }
    for r in 1..rows {
        let y = rect.y0 + rect.height() * r as f64 / rows as f64;
        p.move_to((rect.x0, y));
        p.line_to((rect.x1, y));
    }
    p
}

/// The Action Lines tool's speed lines across `rect`: parallel (ending at the
/// right edge, random lengths) or radial (around the centre). The lengths
/// come from a fixed pseudo-random sequence, so the live preview and the
/// object created on release are the same.
pub fn action_lines_path(rect: Rect, lines: u32, radial: bool) -> BezPath {
    let n = lines.clamp(2, 500) as usize;
    let mut path = BezPath::new();
    let mut seed: u32 = 0x9E37_79B9 ^ n as u32;
    let mut rnd = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 1000) as f64 / 1000.0
    };
    if radial {
        let c = rect.center();
        let rmax = (rect.width().min(rect.height())) / 2.0;
        for i in 0..n {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            let dir = Vec2::new(a.cos(), a.sin());
            let r0 = rmax * (0.25 + 0.35 * rnd());
            let r1 = rmax * (0.75 + 0.25 * rnd());
            path.move_to(c + dir * r0);
            path.line_to(c + dir * r1);
        }
    } else {
        for i in 0..n {
            let y = rect.y0 + rect.height() * (i as f64 + 0.5) / n as f64;
            let len = rect.width() * (0.3 + 0.7 * rnd());
            path.move_to(Point::new(rect.x1 - len, y));
            path.line_to(Point::new(rect.x1, y));
        }
    }
    path
}

/// Calligraphic stroke: the outline swept by a flat nib along the points.
pub fn calligraphic_path(points: &[Point], width: f64, angle_deg: f64) -> BezPath {
    if points.len() < 2 {
        return BezPath::new();
    }
    let a = angle_deg.to_radians();
    let nib = Vec2::new(a.cos(), a.sin()) * (width / 2.0);
    let mut left: Vec<Point> = points.iter().map(|p| *p + nib).collect();
    let mut right: Vec<Point> = points.iter().map(|p| *p - nib).collect();
    right.reverse();
    left.append(&mut right);
    let p = tracedraw_core::geometry::smooth_path(&left, true);
    shaping::simplify(&p)
}

impl App {
    pub fn tools2_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        match self.tool {
            Tool::Contour => {
                if response.drag_started_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test(p) {
                        self.select(vec![id]);
                        self.drag = Drag::ContourDrag { start: p };
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::ContourDrag { start, .. } = self.drag {
                        let d = (p - start).hypot();
                        self.contour_offset = (d / self.contour_steps.max(1) as f64).max(0.1);
                        self.contour_direction = if self
                            .selection_bounds()
                            .map(|b| b.contains(p))
                            .unwrap_or(false)
                        {
                            ContourDirection::Inside
                        } else {
                            ContourDirection::Outside
                        };
                    }
                }
                if response.clicked_by(PointerButton::Primary) {
                    match self.hit_test(p) {
                        Some(id) => self.select(vec![id]),
                        None => self.select(Vec::new()),
                    }
                }
            }
            Tool::Crop | Tool::Knife => {
                let p = self.snap_point(p);
                if response.drag_started_by(PointerButton::Primary) {
                    self.drag = Drag::Box {
                        start: p,
                        current: p,
                        from_center: false,
                    };
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::Box { current, .. } = &mut self.drag {
                        *current = p;
                    }
                }
            }
            Tool::Table => {
                // Click inside an existing table edits that cell; anywhere else
                // drags out a new table.
                let over_table = self.hit_test(p).and_then(|id| {
                    let (_, s) = self.doc().shape(id).ok()?;
                    matches!(s.kind, ShapeKind::Table(_)).then_some((id, s.transform))
                });
                if response.clicked_by(PointerButton::Primary) {
                    if let Some((id, t)) = over_table {
                        let local = t.inverse() * p;
                        self.table_click(id, local, mods.shift);
                        self.select(vec![id]);
                        self.status = tr("table.type_hint");
                        return;
                    }
                    self.table_edit = None;
                }
                if response.drag_started_by(PointerButton::Primary) && over_table.is_some() {
                    return;
                }
                self.box_like_input(response, p, mods)
            }
            Tool::Spiral | Tool::CommonShapes => self.box_like_input(response, p, mods),
            Tool::BrushStrokes => {
                if response.drag_started_by(PointerButton::Primary) {
                    self.drag = Drag::Freehand { points: vec![p] };
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::Freehand { points } = &mut self.drag {
                        if points
                            .last()
                            .map(|l| (*l - p).hypot() > 0.3)
                            .unwrap_or(true)
                        {
                            points.push(p);
                        }
                    }
                }
            }
            Tool::ParallelDimension => {
                let p = self.snap_point(p);
                if response.clicked_by(PointerButton::Primary) {
                    self.dimension_points.push(p);
                    if self.dimension_points.len() == 3 {
                        let pts = std::mem::take(&mut self.dimension_points);
                        self.create_dimension(pts[0], pts[1], pts[2]);
                    }
                }
            }
            Tool::Connector => {
                if response.drag_started_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test_inside(p) {
                        self.drag = Drag::Connector {
                            from: id,
                            start: p,
                            current: p,
                        };
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::Connector { current, .. } = &mut self.drag {
                        *current = p;
                    }
                }
            }
            _ => {}
        }
    }

    fn box_like_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        let p = self.snap_point(p);
        if response.drag_started_by(PointerButton::Primary) {
            self.drag = Drag::Box {
                start: p,
                current: p,
                from_center: mods.shift,
            };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::Box {
                start,
                current,
                from_center,
            } = &mut self.drag
            {
                let mut q = p;
                if mods.ctrl {
                    let d = q - *start;
                    let m = d.x.abs().max(d.y.abs());
                    q = *start + Vec2::new(m * d.x.signum(), m * d.y.signum());
                }
                *from_center = mods.shift;
                *current = q;
            }
        }
    }

    /// The object a box drag with the current tool would create, as page
    /// space outlines, so the shape itself follows the pointer while
    /// dragging (the target design never shows an empty box for shape
    /// tools). `None` when the drag does not create an object outline
    /// (crop, zoom, text frames).
    pub fn creation_preview(
        &self,
        start: Point,
        current: Point,
        from_center: bool,
    ) -> Option<BezPath> {
        use tracedraw_core::geometry::{
            ellipse_arc_path, ellipse_path, polygon_path, polygon_rect_for_bounds, rect_path,
        };
        let r = App::box_rect(start, current, from_center);
        let path = match self.tool {
            Tool::Rectangle => rect_path(r, self.rect_radius),
            Tool::Ellipse => match &self.ellipse_arc {
                None => ellipse_path(r),
                Some(a) => ellipse_arc_path(r, a.start_deg, a.end_deg, a.pie),
            },
            Tool::Polygon => polygon_path(
                polygon_rect_for_bounds(r, self.polygon_points, 0.0),
                self.polygon_points,
                0.0,
            ),
            Tool::Star => polygon_path(
                polygon_rect_for_bounds(r, self.polygon_points, self.star_sharpness),
                self.polygon_points,
                self.star_sharpness,
            ),
            Tool::Spiral => spiral_path(r, self.spiral_revolutions, self.spiral_logarithmic),
            Tool::CommonShapes => {
                Affine::new([r.width(), 0.0, 0.0, r.height(), r.x0, r.y0])
                    * self.common_shape.unit_path()
            }
            Tool::GraphPaper => grid_path(r, self.graph_rows, self.graph_cols),
            Tool::ActionLines => action_lines_path(r, self.action_lines_count, self.action_lines_radial),
            Tool::Table => grid_path(r, self.table_rows, self.table_cols),
            Tool::Knife => {
                let mut p = BezPath::new();
                p.move_to(start);
                p.line_to(current);
                p
            }
            _ => return None,
        };
        Some(path)
    }

    /// Box drags for the second-wave tools, called from `end_drag`.
    pub fn finish_tools2_box(&mut self, start: Point, current: Point) -> bool {
        let rect = Rect::from_points(start, current);
        match self.tool {
            Tool::Spiral => {
                if rect.width() > 0.1 && rect.height() > 0.1 {
                    let path = spiral_path(rect, self.spiral_revolutions, self.spiral_logarithmic);
                    if let Some(id) = self.new_shape(ShapeKind::Path {
                        path,
                        closed: false,
                    }) {
                        self.select(vec![id]);
                    }
                }
                true
            }
            Tool::CommonShapes => {
                if rect.width() > 0.1 && rect.height() > 0.1 {
                    let unit = self.common_shape.unit_path();
                    let path =
                        Affine::new([rect.width(), 0.0, 0.0, rect.height(), rect.x0, rect.y0])
                            * unit;
                    if let Some(id) = self.new_shape(ShapeKind::Path { path, closed: true }) {
                        self.select(vec![id]);
                    }
                }
                true
            }
            Tool::Table => {
                if rect.width() > 1.0 && rect.height() > 1.0 {
                    self.create_table(rect);
                }
                true
            }
            Tool::Crop => {
                if rect.width() > 0.1 && rect.height() > 0.1 {
                    self.crop_to(rect);
                }
                true
            }
            Tool::Knife => {
                if (current - start).hypot() > 0.5 {
                    self.knife(start, current);
                }
                true
            }
            _ => false,
        }
    }

    /// Clip every object (or the selection) to a rectangle.
    pub fn crop_to(&mut self, rect: Rect) {
        let page = self.page;
        let targets: Vec<Shape> = if self.selection.is_empty() {
            self.doc()
                .page(page)
                .map(|p| {
                    p.layers
                        .iter()
                        .filter(|l| !l.locked)
                        .flat_map(|l| l.shapes.clone())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            self.selected_shapes()
        };
        let clip = rect.to_path(0.01);
        let mut cmds = Vec::new();
        let mut removed = Vec::new();
        for s in targets {
            if matches!(s.kind, ShapeKind::Bitmap { .. }) {
                continue;
            }
            let b = s.bounds();
            if rect.contains_rect(b) {
                continue;
            }
            let pp = s.page_path();
            let clipped = shaping::overlay(&pp, &clip, shaping::Op::Intersect);
            if clipped.elements().is_empty() {
                removed.push(s.id);
            } else {
                let local = s.transform.inverse() * clipped;
                cmds.push(Command::SetShapeKind {
                    shape: s.id,
                    kind: ShapeKind::Path {
                        path: local,
                        closed: true,
                    },
                });
            }
        }
        if !removed.is_empty() {
            cmds.push(Command::DeleteShapes { shapes: removed });
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Crop", &cmds);
        }
        let doc = self.engine.document();
        let keep: Vec<ShapeId> = self
            .selection
            .iter()
            .copied()
            .filter(|id| doc.shape(*id).is_ok())
            .collect();
        self.selection = keep;
    }

    /// Cut objects along a line into two pieces each.
    pub fn knife(&mut self, a: Point, b: Point) {
        let targets: Vec<Shape> = if self.selection.is_empty() {
            self.doc()
                .page(self.page)
                .map(|p| {
                    p.layers
                        .iter()
                        .filter(|l| !l.locked)
                        .flat_map(|l| l.shapes.clone())
                        .collect()
                })
                .unwrap_or_default()
        } else {
            self.selected_shapes()
        };
        let dir = (b - a).normalize();
        let n = Vec2::new(-dir.y, dir.x);
        let big = 10_000.0;
        let half = |sign: f64| -> BezPath {
            let o = a - dir * big;
            let e = a + dir * big;
            let mut p = BezPath::new();
            p.move_to(o);
            p.line_to(e);
            p.line_to(e + n * big * sign);
            p.line_to(o + n * big * sign);
            p.close_path();
            p
        };
        let (left, right) = (half(1.0), half(-1.0));
        let mut cmds = Vec::new();
        let mut new_ids = Vec::new();
        let Some(layer) = self.active_layer() else {
            return;
        };
        for s in targets {
            if matches!(s.kind, ShapeKind::Bitmap { .. } | ShapeKind::Group { .. }) {
                continue;
            }
            let bb = s.bounds();
            if !crosses(bb, a, b) {
                continue;
            }
            let pp = s.page_path();
            let p1 = shaping::overlay(&pp, &left, shaping::Op::Intersect);
            let p2 = shaping::overlay(&pp, &right, shaping::Op::Intersect);
            if p1.elements().is_empty() || p2.elements().is_empty() {
                continue;
            }
            cmds.push(Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Path {
                    path: s.transform.inverse() * p1,
                    closed: true,
                },
            });
            let id = self.engine.new_shape_id();
            let mut piece = Shape::new(
                id,
                ShapeKind::Path {
                    path: s.transform.inverse() * p2,
                    closed: true,
                },
            );
            piece.transform = s.transform;
            piece.fill = s.fill.clone();
            piece.stroke = s.stroke.clone();
            piece.opacity = s.opacity;
            cmds.push(Command::AddShape {
                layer,
                shape: piece,
            });
            new_ids.push(s.id);
            new_ids.push(id);
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Knife", &cmds);
            self.select(new_ids);
        }
    }

    pub fn apply_contour(&mut self) {
        let shapes = self.selected_shapes();
        let Some(layer) = self.active_layer() else {
            return;
        };
        let steps = self.contour_steps.max(1);
        let mut cmds = Vec::new();
        let mut groups = Vec::new();
        for s in shapes {
            if matches!(s.kind, ShapeKind::Bitmap { .. } | ShapeKind::Group { .. }) {
                continue;
            }
            let base = s.page_path();
            let from = match &s.fill {
                Fill::Solid(c) => *c,
                _ => Color::WHITE,
            };
            let to = self.contour_color;
            let sign = if self.contour_direction == ContourDirection::Outside {
                1.0
            } else {
                -1.0
            };
            let mut ids = vec![s.id];
            let (_, idx) = self.doc().locate(s.id).unwrap_or((layer, 0));
            let mut new_shapes = Vec::new();
            for i in 1..=steps {
                let d = sign * self.contour_offset * i as f64;
                let path = shaping::offset(&base, d);
                if path.elements().is_empty() {
                    break;
                }
                let t = i as f32 / steps as f32;
                let color = lerp_color(from, to, t);
                let id = self.engine.new_shape_id();
                let mut c = Shape::new(id, ShapeKind::Path { path, closed: true });
                c.fill = Fill::Solid(color);
                c.stroke = s.stroke.clone().map(|mut st| {
                    st.color = color;
                    st
                });
                c.opacity = s.opacity;
                ids.push(id);
                new_shapes.push(c);
            }
            // Outside contours go behind the object (outermost first); inside ones on top.
            let outside = self.contour_direction == ContourDirection::Outside;
            for (k, c) in new_shapes.into_iter().enumerate() {
                let cid = c.id;
                cmds.push(Command::AddShape { layer, shape: c });
                let index = if outside {
                    idx.saturating_sub(0)
                } else {
                    idx + 1 + k
                };
                cmds.push(Command::Reorder {
                    shape: cid,
                    layer,
                    index,
                });
            }
            groups.push(ids);
        }
        if cmds.is_empty() {
            return;
        }
        for g in &groups {
            cmds.push(Command::Group { shapes: g.clone() });
        }
        if let Err(e) = self.engine.run_batch("Contour", &cmds) {
            self.status = e.to_string();
        }
        // Select the resulting groups.
        let doc = self.doc();
        if let Ok(p) = doc.page(self.page) {
            let gids: Vec<ShapeId> = p
                .layers
                .iter()
                .flat_map(|l| &l.shapes)
                .filter(|s| matches!(s.kind, ShapeKind::Group { .. }))
                .map(|s| s.id)
                .collect();
            let n = groups.len();
            self.selection = gids.into_iter().rev().take(n).collect();
        }
    }

    pub fn create_dimension(&mut self, a: Point, b: Point, offset_at: Point) {
        let dir = (b - a).normalize();
        let n = Vec2::new(-dir.y, dir.x);
        let off = (offset_at - a).dot(n);
        let (a2, b2) = (a + n * off, b + n * off);
        let len = (b - a).hypot();
        let Some(layer) = self.active_layer() else {
            return;
        };
        let mut path = BezPath::new();
        // Extension lines, dimension line and arrowheads.
        for (p, q) in [(a, a2 + n * 2.0), (b, b2 + n * 2.0)] {
            path.move_to(p);
            path.line_to(q);
        }
        path.move_to(a2);
        path.line_to(b2);
        let arrow = |path: &mut BezPath, tip: Point, d: Vec2| {
            let back = tip - d * 3.0;
            path.move_to(tip);
            path.line_to(back + n * 1.0);
            path.line_to(back - n * 1.0);
            path.close_path();
        };
        arrow(&mut path, a2, -dir);
        arrow(&mut path, b2, dir);
        let line_id = self.engine.new_shape_id();
        let mut line = Shape::new(
            line_id,
            ShapeKind::Path {
                path,
                closed: false,
            },
        );
        line.fill = Fill::Solid(Color::BLACK);
        line.stroke = Some(Stroke::hairline(Color::BLACK));
        let text_id = self.engine.new_shape_id();
        let label = format!("{:.2} {}", self.units.from_mm(len), self.units.short());
        let mid = a2.midpoint(b2) + n * 1.5;
        let mut text = Shape::new(
            text_id,
            ShapeKind::Text {
                spans: vec![TextSpan {
                    text: label,
                    ..TextSpan::new("", self.text_font.clone(), 10.0)
                }],
                origin: Point::ZERO,
                frame: None,
                align: tracedraw_core::TextAlign::Left,
                para: Default::default(),
                on_path: None,
            },
        );
        let angle = dir.atan2();
        text.transform = Affine::translate(mid.to_vec2())
            * Affine::rotate(angle)
            * Affine::translate((-len / 4.0, 0.0));
        text.fill = Fill::Solid(Color::BLACK);
        text.stroke = None;
        let cmds = vec![
            Command::AddShape { layer, shape: line },
            Command::AddShape { layer, shape: text },
            Command::Group {
                shapes: vec![line_id, text_id],
            },
        ];
        let _ = self.engine.run_batch("Dimension", &cmds);
    }

    pub fn finish_connector(&mut self, from: ShapeId, start: Point, at: Point) {
        let Some(to) = self.hit_test_inside(at).filter(|id| *id != from) else {
            return;
        };
        // Leave from the anchor nearest the drag start, arrive at the one
        // nearest the drop (custom anchors from the Anchor Editing tool).
        let Some((pa, pb)) = self.connector_anchors(from, start, to, at) else {
            return;
        };
        let mut path = BezPath::new();
        path.move_to(pa);
        path.line_to(pb);
        if let Some(id) = self.new_shape(ShapeKind::Path {
            path,
            closed: false,
        }) {
            let _ = self.engine.run(&Command::SetShapeName {
                shape: id,
                name: Some("Connector".into()),
            });
            self.select(vec![id]);
        }
    }

    pub fn create_table(&mut self, rect: Rect) {
        let (rows, cols) = (self.table_rows.max(1), self.table_cols.max(1));
        let mut table = tracedraw_core::Table::new(rect, rows, cols, self.table_border.clone());
        table.cell_fill = self.table_fill.clone();
        if let Some(id) = self.new_shape(ShapeKind::Table(table)) {
            self.select(vec![id]);
        }
    }

    /// Ctrl+Shift+Q: replace an outline by a filled object of its shape.
    pub fn convert_outline_to_object(&mut self) {
        use i_overlay::mesh::float::stroke::offset::StrokeOffset;
        use i_overlay::mesh::float::style::{LineCap as ICap, LineJoin as IJoin, StrokeStyle};
        let shapes = self.selected_shapes();
        let Some(layer) = self.active_layer() else {
            return;
        };
        let mut cmds = Vec::new();
        let mut ids = Vec::new();
        for s in shapes {
            let Some(st) = &s.stroke else { continue };
            let width = if st.width <= Stroke::HAIRLINE + 1e-9 {
                0.25
            } else {
                st.width
            };
            let pp = s.page_path();
            let mut out = BezPath::new();
            for contour in polylines(&pp) {
                let closed = contour.1;
                let pts = contour.0;
                let style = StrokeStyle::new(width)
                    .line_join(match st.join {
                        tracedraw_core::LineJoin::Round => IJoin::Round(0.05),
                        tracedraw_core::LineJoin::Bevel => IJoin::Bevel,
                        tracedraw_core::LineJoin::Miter => IJoin::Miter(4.0),
                    })
                    .start_cap(match st.cap {
                        tracedraw_core::LineCap::Round => ICap::Round(0.05),
                        tracedraw_core::LineCap::Square => ICap::Square,
                        tracedraw_core::LineCap::Butt => ICap::Butt,
                    })
                    .end_cap(match st.cap {
                        tracedraw_core::LineCap::Round => ICap::Round(0.05),
                        tracedraw_core::LineCap::Square => ICap::Square,
                        tracedraw_core::LineCap::Butt => ICap::Butt,
                    });
                let shapes_out = pts.stroke(style, closed);
                for shape in shapes_out {
                    for c in shape {
                        for (i, p) in c.iter().enumerate() {
                            let pt = Point::new(p[0], p[1]);
                            if i == 0 {
                                out.move_to(pt);
                            } else {
                                out.line_to(pt);
                            }
                        }
                        out.close_path();
                    }
                }
            }
            if out.elements().is_empty() {
                continue;
            }
            let id = self.engine.new_shape_id();
            let mut o = Shape::new(
                id,
                ShapeKind::Path {
                    path: out,
                    closed: true,
                },
            );
            o.fill = Fill::Solid(st.color);
            o.stroke = None;
            o.name = Some("Outline".into());
            cmds.push(Command::AddShape { layer, shape: o });
            cmds.push(Command::SetStroke {
                shapes: vec![s.id],
                stroke: None,
            });
            ids.push(id);
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Convert Outline To Object", &cmds);
            self.select(ids);
        }
    }
}

fn crosses(bb: Rect, a: Point, b: Point) -> bool {
    // Conservative: the segment's bounding box overlaps the object bounds.
    !bb.intersect(Rect::from_points(a, b).inflate(0.01, 0.01))
        .is_zero_area()
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let [r1, g1, b1] = a.to_rgb8();
    let [r2, g2, b2] = b.to_rgb8();
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color::rgb8(l(r1, r2), l(g1, g2), l(b1, b2))
}

/// Flattened subpaths of a page path as point lists with a closed flag.
fn polylines(path: &BezPath) -> Vec<(Vec<[f64; 2]>, bool)> {
    use tracedraw_core::geometry::PathEl;
    let mut out: Vec<(Vec<[f64; 2]>, bool)> = Vec::new();
    let mut cur: Vec<[f64; 2]> = Vec::new();
    let mut closed = false;
    kurbo::flatten(path.elements().iter().copied(), 0.05, &mut |el| match el {
        PathEl::MoveTo(p) => {
            if cur.len() >= 2 {
                out.push((std::mem::take(&mut cur), closed));
            } else {
                cur.clear();
            }
            closed = false;
            cur.push([p.x, p.y]);
        }
        PathEl::LineTo(p) => cur.push([p.x, p.y]),
        PathEl::ClosePath => closed = true,
        _ => {}
    });
    if cur.len() >= 2 {
        out.push((cur, closed));
    }
    out
}

// ---------------------------------------------------------------------------
// Effects tools: Blend, Extrude, Distort; brush tools: Smooth, Smear, Twirl;
// Freehand Pick lasso.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DistortMode {
    PushPull,
    Zipper,
    Twister,
}

impl App {
    pub fn effects_input(&mut self, response: &Response, p: Point, _mods: Modifiers) {
        match self.tool {
            Tool::Blend => {
                if response.drag_started_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test_inside(p) {
                        self.drag = Drag::Connector {
                            from: id,
                            start: p,
                            current: p,
                        };
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::Connector { current, .. } = &mut self.drag {
                        *current = p;
                    }
                }
            }
            Tool::Extrude => {
                if response.drag_started_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test(p) {
                        self.select(vec![id]);
                        self.drag = Drag::ContourDrag { start: p };
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::ContourDrag { start } = self.drag {
                        self.extrude_depth = p - start;
                    }
                }
                if response.clicked_by(PointerButton::Primary) {
                    match self.hit_test(p) {
                        Some(id) => self.select(vec![id]),
                        None => self.select(Vec::new()),
                    }
                }
            }
            Tool::Distort | Tool::Envelope => {
                if response.clicked_by(PointerButton::Primary) {
                    match self.hit_test(p) {
                        Some(id) => self.select(vec![id]),
                        None => self.select(Vec::new()),
                    }
                }
            }
            Tool::Smooth | Tool::Smear | Tool::Twirl => self.brush_input(response, p),
            Tool::FreeformPick => {
                if response.drag_started_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test(p) {
                        if !self.selection.contains(&id) {
                            self.select(vec![id]);
                        }
                        self.drag = Drag::Move {
                            last: p,
                            total: Vec2::ZERO,
                            start_bounds: self.selection_bounds().unwrap_or(Rect::ZERO),
                        };
                    } else {
                        self.drag = Drag::Freehand { points: vec![p] };
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    match &mut self.drag {
                        Drag::Freehand { points } => points.push(p),
                        Drag::Move { last, total, .. } => {
                            let d = p - *last;
                            *last = p;
                            *total += d;
                        }
                        _ => {}
                    }
                }
                if response.clicked_by(PointerButton::Primary) {
                    match self.hit_test(p) {
                        Some(id) => self.select(vec![id]),
                        None => self.select(Vec::new()),
                    }
                }
            }
            _ => {}
        }
    }

    fn brush_input(&mut self, response: &Response, p: Point) {
        if response.drag_started_by(PointerButton::Primary) {
            if self.selection.is_empty() {
                if let Some(id) = self.hit_test(p) {
                    self.select(vec![id]);
                }
            }
            // Brushes work on curves; convert first.
            self.convert_to_curves();
            self.drag = Drag::Node { last: p };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::Node { last } = self.drag {
                let delta = p - last;
                let radius = self.brush_radius;
                let shapes = self.selected_shapes();
                let mut cmds = Vec::new();
                for s in shapes {
                    let ShapeKind::Path { path, closed } = &s.kind else {
                        continue;
                    };
                    let inv = s.transform.inverse();
                    let lp = inv * p;
                    let ld = (inv * (Point::ZERO + delta)) - (inv * Point::ZERO);
                    let new_path = match self.tool {
                        Tool::Smear => tracedraw_core::effects::smear(path, lp, ld, radius),
                        Tool::Twirl => {
                            tracedraw_core::effects::twirl(path, lp, delta.hypot() * 0.05, radius)
                        }
                        _ => tracedraw_core::effects::smooth(path, lp, radius, 0.3),
                    };
                    cmds.push(Command::SetShapeKind {
                        shape: s.id,
                        kind: ShapeKind::Path {
                            path: new_path,
                            closed: *closed,
                        },
                    });
                }
                if !cmds.is_empty() {
                    let label = match self.tool {
                        Tool::Smear => "Smear",
                        Tool::Twirl => "Twirl",
                        _ => "Smooth",
                    };
                    if self.engine.undo_label() == Some(label) {
                        let _ = self.engine.undo();
                    }
                    let _ = self.engine.run_batch(label, &cmds);
                }
                self.drag = Drag::Node { last: p };
            }
        }
    }

    /// Lasso finished (Freehand Pick): select objects whose bounds centre
    /// lies inside the drawn polygon.
    pub fn finish_lasso(&mut self, points: Vec<Point>) {
        if points.len() < 3 {
            return;
        }
        let poly = tracedraw_core::effects::polygon(&points);
        let Ok(page) = self.doc().page(self.page) else {
            return;
        };
        let mut ids = Vec::new();
        for s in page
            .layers
            .iter()
            .filter(|l| l.visible && !l.locked)
            .flat_map(|l| &l.shapes)
        {
            if s.locked {
                continue;
            }
            if point_in_path(&poly, s.bounds().center()) {
                ids.push(s.id);
            }
        }
        self.select(ids);
    }

    pub fn apply_extrude(&mut self) {
        let shapes = self.selected_shapes();
        let Some(layer) = self.active_layer() else {
            return;
        };
        let depth = self.extrude_depth;
        if depth.hypot() < 0.1 {
            self.status = tr("status.extrude_drag_hint");
            return;
        }
        let mut cmds = Vec::new();
        for s in shapes {
            if matches!(s.kind, ShapeKind::Bitmap { .. } | ShapeKind::Group { .. }) {
                continue;
            }
            let (sides, back) = tracedraw_core::effects::extrude(&s.page_path(), depth);
            let base = match &s.fill {
                Fill::Solid(c) => *c,
                _ => Color::cmyk_pct(0.0, 0.0, 0.0, 30.0),
            };
            let (_, idx) = self.doc().locate(s.id).unwrap_or((layer, 0));
            let mut ids = Vec::new();
            let back_id = self.engine.new_shape_id();
            let mut bs = Shape::new(
                back_id,
                ShapeKind::Path {
                    path: back,
                    closed: true,
                },
            );
            bs.fill = Fill::Solid(lerp_color(base, Color::BLACK, 0.5));
            bs.stroke = None;
            cmds.push(Command::AddShape { layer, shape: bs });
            cmds.push(Command::Reorder {
                shape: back_id,
                layer,
                index: idx,
            });
            ids.push(back_id);
            // Shade side faces by their direction relative to the light (top-left).
            let light = Vec2::new(-0.6, 0.8);
            for side in sides {
                use tracedraw_core::geometry::Shape as _;
                let c = side.bounding_box().center();
                let dir = (c - s.bounds().center()).normalize();
                let shade = 0.25 + 0.35 * (1.0 - dir.dot(light)).clamp(0.0, 1.0) / 2.0;
                let id = self.engine.new_shape_id();
                let mut f = Shape::new(
                    id,
                    ShapeKind::Path {
                        path: side,
                        closed: true,
                    },
                );
                f.fill = Fill::Solid(lerp_color(base, Color::BLACK, shade as f32));
                f.stroke = None;
                cmds.push(Command::AddShape { layer, shape: f });
                cmds.push(Command::Reorder {
                    shape: id,
                    layer,
                    index: idx,
                });
                ids.push(id);
            }
            ids.push(s.id);
            cmds.push(Command::Group { shapes: ids });
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Extrude", &cmds);
        }
    }

    pub fn apply_distort(&mut self) {
        use tracedraw_core::effects::{distort, Distort};
        let how = match self.distort_mode {
            DistortMode::PushPull => Distort::PushPull {
                amount: self.distort_amount,
            },
            DistortMode::Zipper => Distort::Zipper {
                amplitude: self.distort_amount.abs() / 10.0,
                frequency: self.distort_frequency,
            },
            DistortMode::Twister => Distort::Twister {
                angle_deg: self.distort_amount * 3.6,
            },
        };
        let shapes = self.selected_shapes();
        let cmds: Vec<Command> = shapes
            .iter()
            .filter(|s| !matches!(s.kind, ShapeKind::Bitmap { .. } | ShapeKind::Group { .. }))
            .map(|s| {
                let pp = s.page_path();
                let d = distort(&pp, how);
                Command::SetShapeKind {
                    shape: s.id,
                    kind: ShapeKind::Path {
                        path: s.transform.inverse() * d,
                        closed: true,
                    },
                }
            })
            .collect();
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Distort", &cmds);
        }
    }
}

/// Even-odd point-in-polygon on a flattened path.
pub fn point_in_path(path: &BezPath, p: Point) -> bool {
    let pts = tracedraw_core::effects::resample(path, 256);
    let mut inside = false;
    let n = pts.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (pts[i], pts[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y + 1e-12) + a.x
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(p: &BezPath) -> Rect {
        p.bounding_box()
    }

    /// Every shape tool previews the object it is about to create, sized
    /// to the dragged box, while the drag is still in progress.
    #[test]
    fn shape_tools_preview_the_shape_being_drawn() {
        let mut app = App::headless();
        let (a, b) = (Point::new(10.0, 20.0), Point::new(60.0, 50.0));
        let full = Rect::from_points(a, b);
        for tool in [
            Tool::Rectangle,
            Tool::Ellipse,
            Tool::Polygon,
            Tool::Star,
            Tool::CommonShapes,
            Tool::GraphPaper,
            Tool::Table,
        ] {
            app.tool = tool;
            let p = app
                .creation_preview(a, b, false)
                .unwrap_or_else(|| panic!("{tool:?} has a preview"));
            let r = bounds(&p);
            assert!(
                (r.width() - full.width()).abs() < 0.5 && (r.height() - full.height()).abs() < 0.5,
                "{tool:?}: {r:?}"
            );
        }
        // The spiral fills the box but ends inside its corners.
        app.tool = Tool::Spiral;
        let r = bounds(&app.creation_preview(a, b, false).expect("spiral"));
        assert!(full.inflate(0.01, 0.01).contains_rect(r) && r.width() > full.width() * 0.8);
        // Shift: the box grows from the start point as its centre.
        app.tool = Tool::Rectangle;
        let r = bounds(&app.creation_preview(a, b, true).expect("rect"));
        assert!((r.center() - a).hypot() < 1e-6);
        // Crop and zoom boxes are not objects.
        app.tool = Tool::Crop;
        assert!(app.creation_preview(a, b, false).is_none());
    }

    #[test]
    fn knife_preview_follows_the_drag_direction() {
        let mut app = App::headless();
        app.tool = Tool::Knife;
        let (a, b) = (Point::new(10.0, 50.0), Point::new(60.0, 20.0));
        let p = app.creation_preview(a, b, false).expect("knife line");
        let els = p.elements();
        assert!(matches!(els[0], kurbo::PathEl::MoveTo(q) if q == a));
        assert!(matches!(els[1], kurbo::PathEl::LineTo(q) if q == b));
    }

    #[test]
    fn action_lines_preview_matches_the_created_object() {
        let r = Rect::new(0.0, 0.0, 80.0, 40.0);
        assert_eq!(action_lines_path(r, 12, false), action_lines_path(r, 12, false));
        assert_eq!(action_lines_path(r, 12, true).elements().len(), 24);
        let g = grid_path(r, 4, 3);
        // Frame (5 elements) plus 2 vertical and 3 horizontal lines.
        assert_eq!(g.elements().len(), 5 + 2 * 2 + 3 * 2);
    }
}
