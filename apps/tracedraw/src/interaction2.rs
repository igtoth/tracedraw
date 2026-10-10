//! Input for the tools added with the full toolbox, the pending "click an
//! object" operations from menus, effect-node editing with the Shape tool,
//! and the right-click context menu trigger.

use crate::app::{App, Drag};
use crate::i18n::tr;
use crate::tools::Tool;
use egui::{Modifiers, PointerButton, Response};
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, BezPath, Point, Rect, Shape as _, Vec2},
    live::Effect,
    Color, Command, Fill, ShapeId, Stroke,
};

impl App {
    /// Menu operations waiting for a click on an object. Returns true when
    /// the click was consumed.
    pub fn pending_click(&mut self, response: &Response, p: Point) -> bool {
        let waiting = self.pending_order.is_some()
            || self.pending_copy_properties
            || self.pending_copy_effect.is_some()
            || self.pending_clone_effect.is_some()
            || self.pending_blend_path;
        if !waiting {
            return false;
        }
        if response.secondary_clicked() {
            self.pending_order = None;
            self.pending_copy_properties = false;
            self.pending_copy_effect = None;
            self.pending_clone_effect = None;
            self.pending_blend_path = false;
            self.status = tr("status.cancelled");
            return true;
        }
        if !response.clicked_by(PointerButton::Primary) {
            return true;
        }
        let target = self.hit_test(p);
        if let Some(in_front) = self.pending_order.take() {
            if let Some(t) = target {
                self.order_relative_to(t, in_front);
            }
        } else if self.pending_copy_properties {
            self.pending_copy_properties = false;
            if let Some(t) = target {
                self.copy_properties_from(t);
            }
        } else if let Some(kind) = self.pending_copy_effect.take() {
            if let Some(t) = target {
                self.copy_effect_from(t, kind);
            }
        } else if let Some(kind) = self.pending_clone_effect.take() {
            if let Some(t) = target {
                self.clone_effect_from(t, kind);
                return true;
            }
        } else if self.pending_blend_path {
            self.pending_blend_path = false;
            if let Some(t) = target {
                self.set_blend_path(t);
            }
        }
        self.status.clear();
        true
    }

    /// Tools that did not exist in the first toolbox.
    pub fn tools3_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        match self.tool {
            Tool::FreeTransform => self.free_transform_input(response, p, mods),
            Tool::AttractRepel | Tool::Smudge | Tool::Roughen => {
                self.brush3_input(response, p, mods)
            }
            Tool::SegmentDelete => {
                if response.clicked_by(PointerButton::Primary) {
                    self.delete_segment_at(p);
                }
            }
            Tool::ShapeRecognition | Tool::Sketch => {
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
            Tool::HorizontalVerticalDimension | Tool::SegmentDimension => {
                let p = self.snap_point(p);
                if response.clicked_by(PointerButton::Primary) {
                    self.dimension_points.push(p);
                    if self.dimension_points.len() == 3 {
                        let pts = std::mem::take(&mut self.dimension_points);
                        let (a, b) = if self.tool == Tool::HorizontalVerticalDimension {
                            // Snap to the dominant axis.
                            if (pts[1].x - pts[0].x).abs() >= (pts[1].y - pts[0].y).abs() {
                                (pts[0], Point::new(pts[1].x, pts[0].y))
                            } else {
                                (pts[0], Point::new(pts[0].x, pts[1].y))
                            }
                        } else {
                            (pts[0], pts[1])
                        };
                        self.create_dimension(a, b, pts[2]);
                    }
                }
            }
            Tool::AngularDimension => {
                let p = self.snap_point(p);
                if response.clicked_by(PointerButton::Primary) {
                    self.dimension_points.push(p);
                    if self.dimension_points.len() == 4 {
                        let pts = std::mem::take(&mut self.dimension_points);
                        self.create_angular_dimension(pts[0], pts[1], pts[2], pts[3]);
                    }
                }
            }
            Tool::Callout => {
                let p = self.snap_point(p);
                if response.clicked_by(PointerButton::Primary) {
                    self.dimension_points.push(p);
                    if self.dimension_points.len() == 3 {
                        let pts = std::mem::take(&mut self.dimension_points);
                        self.create_callout(pts[0], pts[1], pts[2]);
                    }
                }
            }
            Tool::AnchorEditing => self.anchor_input(response, p),
            Tool::RightAngleConnector | Tool::RoundedConnector => {
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
            Tool::BlockShadow => {
                if response.drag_started_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test(p) {
                        self.select(vec![id]);
                        self.drag = Drag::ContourDrag { start: p };
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::ContourDrag { start } = self.drag {
                        let offset = p - start;
                        let color = self.block_shadow_color;
                        let gap = self.block_shadow_gap;
                        self.push_effect(Effect::BlockShadow { offset, color, gap }, true);
                    }
                }
            }
            Tool::Envelope => {
                if response.clicked_by(PointerButton::Primary) {
                    match self.hit_test(p) {
                        Some(id) => {
                            self.select(vec![id]);
                            let has_env = self
                                .doc()
                                .find_shape(id)
                                .map(|s| {
                                    s.effects
                                        .iter()
                                        .any(|e| matches!(e, Effect::Envelope { .. }))
                                })
                                .unwrap_or(false);
                            if !has_env {
                                self.apply_envelope_preset(99);
                            }
                            self.set_tool(Tool::Shape);
                        }
                        None => self.select(Vec::new()),
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
                        let depth = p - start;
                        self.extrude.depth = depth;
                        let e = self.extrude.clone();
                        self.push_effect(
                            Effect::Extrude {
                                depth,
                                vanishing: None,
                                amount: e.amount,
                                shade_from: e.shade.then_some(e.shade_from),
                                shade_to: e.shade.then_some(e.shade_to),
                                light_angle: e.light_angle,
                                light_intensity: e.light_intensity,
                                bevel: e.bevel,
                            },
                            true,
                        );
                    }
                }
                if response.clicked_by(PointerButton::Primary) {
                    match self.hit_test(p) {
                        Some(id) => self.select(vec![id]),
                        None => self.select(Vec::new()),
                    }
                }
            }
            Tool::Distort => {
                if response.drag_started_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test(p) {
                        self.select(vec![id]);
                        self.drag = Drag::ContourDrag { start: p };
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::ContourDrag { start } = self.drag {
                        let d = p - start;
                        let how = match self.distort_mode {
                            crate::tools2::DistortMode::PushPull => {
                                tracedraw_core::effects::Distort::PushPull { amount: d.x * 2.0 }
                            }
                            crate::tools2::DistortMode::Zipper => {
                                tracedraw_core::effects::Distort::Zipper {
                                    amplitude: d.hypot() / 4.0,
                                    frequency: self.distort_frequency,
                                }
                            }
                            crate::tools2::DistortMode::Twister => {
                                tracedraw_core::effects::Distort::Twister {
                                    angle_deg: d.x * 3.0,
                                }
                            }
                        };
                        self.push_effect(Effect::Distort(how), true);
                    }
                }
                if response.clicked_by(PointerButton::Primary) {
                    match self.hit_test(p) {
                        Some(id) => self.select(vec![id]),
                        None => self.select(Vec::new()),
                    }
                }
            }
            Tool::AreaFill => {
                if response.clicked_by(PointerButton::Primary) {
                    self.area_fill_at(p);
                }
            }
            Tool::MeshFill => {
                if response.clicked_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test(p) {
                        self.select(vec![id]);
                        self.apply_mesh_fill(id);
                    }
                }
            }
            Tool::OutlinePen => {
                self.set_tool(Tool::Pick);
                self.show_dockers = true;
                self.docker_tab = crate::app::DockerTab::Properties;
                self.properties_open = Some(1);
            }
            Tool::OutlineColor => {
                self.set_tool(Tool::Pick);
                self.show_dockers = true;
                self.docker_tab = crate::app::DockerTab::Color;
            }
            _ => {}
        }
    }

    fn free_transform_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        use crate::app::FreeTransformMode as M;
        if response.drag_started_by(PointerButton::Primary) {
            if let Some(id) = self.hit_test(p) {
                if !self.selection.contains(&id) {
                    self.select(vec![id]);
                }
            }
            if !self.selection.is_empty() {
                if self.free_transform_duplicate {
                    // Transform a copy, leave the original where it is.
                    let offset = self.duplicate_offset;
                    self.duplicate_offset = Vec2::ZERO;
                    self.duplicate();
                    self.duplicate_offset = offset;
                }
                self.drag = Drag::Rotate {
                    center: p,
                    start_angle: 0.0,
                    current_angle: 0.0,
                };
                self.free_transform_last = p;
                self.free_transform_reflected = false;
            }
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::Rotate { center, .. } = self.drag {
                let last = self.free_transform_last;
                self.free_transform_last = p;
                let mode = if mods.alt {
                    M::Scale
                } else if mods.ctrl {
                    M::Skew
                } else if mods.shift {
                    M::Reflection
                } else {
                    self.free_transform_mode
                };
                let first = !self.free_transform_reflected;
                if mode == M::Reflection {
                    self.free_transform_reflected = true;
                }
                let t = free_transform_step(mode, center, last, p, first);
                if t != Affine::IDENTITY {
                    self.transform_selection(t);
                }
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            match self.hit_test(p) {
                Some(id) => self.select(vec![id]),
                None => self.select(Vec::new()),
            }
        }
    }

    fn brush3_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        if response.drag_started_by(PointerButton::Primary) {
            if self.selection.is_empty() {
                if let Some(id) = self.hit_test(p) {
                    self.select(vec![id]);
                }
            }
            self.convert_to_curves();
            self.drag = Drag::Node { last: p };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::Node { last } = self.drag {
                let delta = p - last;
                let radius = self.brush_radius;
                let mut cmds = Vec::new();
                for s in self.selected_shapes() {
                    let ShapeKind::Path { path, closed } = &s.kind else {
                        continue;
                    };
                    let inv = s.transform.inverse();
                    let lp = inv * p;
                    let new_path = match self.tool {
                        Tool::AttractRepel => attract_repel(
                            path,
                            lp,
                            radius,
                            if mods.shift { -1.0 } else { 1.0 } * delta.hypot() * 0.3,
                        ),
                        Tool::Smudge => tracedraw_core::effects::smear(
                            &tracedraw_core::effects::smooth(path, lp, radius, 0.2),
                            lp,
                            (inv * (Point::ZERO + delta)) - (inv * Point::ZERO),
                            radius,
                        ),
                        _ => roughen(path, lp, radius, self.roughen_amount),
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
                        Tool::AttractRepel => "Attract",
                        Tool::Smudge => "Smudge",
                        _ => "Roughen",
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

    /// Segment Delete: remove the segment of a curve under the click.
    pub fn delete_segment_at(&mut self, p: Point) {
        let tol = 3.0 / self.view.zoom as f64;
        let shapes: Vec<Shape> = self
            .doc()
            .page(self.page)
            .map(|pg| {
                pg.layers
                    .iter()
                    .filter(|l| l.visible && !l.locked)
                    .flat_map(|l| l.shapes.clone())
                    .collect()
            })
            .unwrap_or_default();
        for s in shapes.iter().rev() {
            let path = s.page_path();
            let Some((seg_idx, _, d)) = tracedraw_core::nodes::nearest_segment(&path, p) else {
                continue;
            };
            if d > tol {
                continue;
            }
            // Split the path: remove element seg_idx, start a new subpath after it.
            let els: Vec<tracedraw_core::geometry::PathEl> = path.elements().to_vec();
            let mut out = BezPath::new();
            let mut last: Option<Point> = None;
            for (i, el) in els.iter().enumerate() {
                use tracedraw_core::geometry::PathEl;
                if i == seg_idx {
                    // Skip this segment; the next element starts a new subpath.
                    let end = match el {
                        PathEl::LineTo(q) | PathEl::QuadTo(_, q) | PathEl::CurveTo(_, _, q) => {
                            Some(*q)
                        }
                        _ => None,
                    };
                    if let Some(e) = end {
                        out.move_to(e);
                        last = Some(e);
                    }
                    continue;
                }
                match el {
                    PathEl::ClosePath => {
                        // Closing becomes a line back to the start when the removed
                        // segment was elsewhere; simplest: open the path.
                    }
                    other => {
                        out.push(*other);
                        if let PathEl::MoveTo(q)
                        | PathEl::LineTo(q)
                        | PathEl::QuadTo(_, q)
                        | PathEl::CurveTo(_, _, q) = other
                        {
                            last = Some(*q);
                        }
                    }
                }
            }
            let _ = last;
            let local = s.transform.inverse() * out;
            let cmds = vec![Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Path {
                    path: local,
                    closed: false,
                },
            }];
            let _ = self.engine.run_batch("Segment Delete", &cmds);
            return;
        }
    }

    /// Shape Recognition: recognise the sketched stroke as a basic shape.
    pub fn finish_shape_recognition(&mut self, points: Vec<Point>) {
        if points.len() < 3 {
            return;
        }
        let pts = tracedraw_core::geometry::simplify(&points, 1.0);
        let b = tracedraw_core::geometry::polyline_path(&pts, false).bounding_box();
        let closed =
            (points[0] - points[points.len() - 1]).hypot() < b.width().max(b.height()) * 0.25;
        let kind = if !closed && pts.len() <= 3 {
            // Straight line.
            let mut path = BezPath::new();
            path.move_to(points[0]);
            path.line_to(points[points.len() - 1]);
            ShapeKind::Path {
                path,
                closed: false,
            }
        } else if closed && (3..=5).contains(&pts.len()) && pts.len() != 4 {
            ShapeKind::Polygon {
                rect: tracedraw_core::geometry::polygon_rect_for_bounds(b, 3, 0.0),
                points: 3,
                sharpness: 0.0,
            }
        } else if closed && pts.len() == 4 || closed && pts.len() == 5 {
            ShapeKind::Rect {
                rect: b,
                radius: 0.0,
            }
        } else if closed {
            // Circularity: compare the area with the bounding ellipse.
            let poly = tracedraw_core::geometry::polyline_path(&points, true);
            let area = poly.area().abs();
            let ell = std::f64::consts::PI * b.width() * b.height() / 4.0;
            if (area / ell.max(1e-9) - 1.0).abs() < 0.2 {
                ShapeKind::Ellipse { rect: b, arc: None }
            } else {
                ShapeKind::Path {
                    path: tracedraw_core::geometry::smooth_path(&pts, true),
                    closed: true,
                }
            }
        } else {
            ShapeKind::Path {
                path: tracedraw_core::geometry::smooth_path(&pts, false),
                closed: false,
            }
        };
        if let Some(id) = self.new_shape(kind) {
            self.select(vec![id]);
        }
    }

    /// Sketch: smooth the stroke and merge it with a selected curve
    /// whose end is near the stroke's start.
    pub fn finish_sketch(&mut self, points: Vec<Point>) {
        let pts = tracedraw_core::geometry::simplify(&points, 0.8);
        if pts.len() < 2 {
            return;
        }
        let path = tracedraw_core::geometry::smooth_path(&pts, false);
        if let Some(sel) = self
            .selected_shapes()
            .into_iter()
            .find(|s| matches!(s.kind, ShapeKind::Path { closed: false, .. }))
        {
            let existing = sel.page_path();
            let end = existing.elements().iter().rev().find_map(|el| match el {
                tracedraw_core::geometry::PathEl::LineTo(q)
                | tracedraw_core::geometry::PathEl::CurveTo(_, _, q) => Some(*q),
                _ => None,
            });
            if let Some(e) = end {
                if (e - pts[0]).hypot() < 5.0 {
                    let mut joined = existing.clone();
                    for el in path.elements().iter().skip(1) {
                        joined.push(*el);
                    }
                    let local = sel.transform.inverse() * joined;
                    self.run(Command::SetShapeKind {
                        shape: sel.id,
                        kind: ShapeKind::Path {
                            path: local,
                            closed: false,
                        },
                    });
                    return;
                }
            }
        }
        if let Some(id) = self.new_shape(ShapeKind::Path {
            path,
            closed: false,
        }) {
            self.select(vec![id]);
        }
    }

    pub fn create_angular_dimension(&mut self, center: Point, a: Point, b: Point, label_at: Point) {
        let a0 = (a - center).atan2();
        let a1 = (b - center).atan2();
        let mut sweep = a1 - a0;
        if sweep < 0.0 {
            sweep += std::f64::consts::TAU;
        }
        let r = (label_at - center).hypot().max(1.0);
        let mut path = BezPath::new();
        path.move_to(center);
        path.line_to(center + Vec2::new(a0.cos(), a0.sin()) * (r + 2.0));
        path.move_to(center);
        path.line_to(center + Vec2::new(a1.cos(), a1.sin()) * (r + 2.0));
        let n = 24;
        for i in 0..=n {
            let t = a0 + sweep * i as f64 / n as f64;
            let q = center + Vec2::new(t.cos(), t.sin()) * r;
            if i == 0 {
                path.move_to(q);
            } else {
                path.line_to(q);
            }
        }
        let Some(layer) = self.active_layer() else {
            return;
        };
        let id = self.engine.new_shape_id();
        let mut line = Shape::new(
            id,
            ShapeKind::Path {
                path,
                closed: false,
            },
        );
        line.fill = Fill::None;
        line.stroke = Some(Stroke::hairline(Color::BLACK));
        let tid = self.engine.new_shape_id();
        let mid = a0 + sweep / 2.0;
        let lp = center + Vec2::new(mid.cos(), mid.sin()) * (r + 3.0);
        let mut text = Shape::new(
            tid,
            ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new(
                    format!("{:.1}°", sweep.to_degrees()),
                    self.text_font.clone(),
                    10.0,
                )],
                origin: lp,
                frame: None,
                align: tracedraw_core::TextAlign::Left,
                para: Default::default(),
                on_path: None,
            },
        );
        text.fill = Fill::Solid(Color::BLACK);
        text.stroke = None;
        let cmds = vec![
            Command::AddShape { layer, shape: line },
            Command::AddShape { layer, shape: text },
            Command::Group {
                shapes: vec![id, tid],
            },
        ];
        let _ = self.engine.run_batch("Angular Dimension", &cmds);
    }

    pub fn create_callout(&mut self, tip: Point, elbow: Point, text_at: Point) {
        let mut path = BezPath::new();
        path.move_to(tip);
        path.line_to(elbow);
        path.line_to(text_at);
        let Some(layer) = self.active_layer() else {
            return;
        };
        let id = self.engine.new_shape_id();
        let mut line = Shape::new(
            id,
            ShapeKind::Path {
                path,
                closed: false,
            },
        );
        line.fill = Fill::None;
        line.stroke = Some(Stroke {
            end_arrow: tracedraw_core::Arrowhead::None,
            start_arrow: tracedraw_core::Arrowhead::Arrow,
            ..Stroke::hairline(Color::BLACK)
        });
        let tid = self.engine.new_shape_id();
        let mut text = Shape::new(
            tid,
            ShapeKind::Text {
                spans: vec![tracedraw_core::TextSpan::new(
                    tr("tool.callout_text"),
                    self.text_font.clone(),
                    10.0,
                )],
                origin: text_at + Vec2::new(1.0, 1.0),
                frame: None,
                align: tracedraw_core::TextAlign::Left,
                para: Default::default(),
                on_path: None,
            },
        );
        text.fill = Fill::Solid(Color::BLACK);
        text.stroke = None;
        let cmds = vec![
            Command::AddShape { layer, shape: line },
            Command::AddShape { layer, shape: text },
            Command::Group {
                shapes: vec![id, tid],
            },
        ];
        let _ = self.engine.run_batch("Callout", &cmds);
        self.begin_text_edit(tid);
    }

    /// Right-angle and rounded connectors: an elbow path between the
    /// facing sides of two objects.
    pub fn finish_elbow_connector(
        &mut self,
        from: ShapeId,
        start: Point,
        at: Point,
        rounded: bool,
    ) {
        let Some(to) = self.hit_test_inside(at).filter(|id| *id != from) else {
            return;
        };
        let Some((pa, pb)) = self.connector_anchors(from, start, to, at) else {
            return;
        };
        let horizontal = (pb.x - pa.x).abs() > (pb.y - pa.y).abs();
        let mid = if horizontal {
            Point::new((pa.x + pb.x) / 2.0, pa.y)
        } else {
            Point::new(pa.x, (pa.y + pb.y) / 2.0)
        };
        let c1 = mid;
        let c2 = if horizontal {
            Point::new(mid.x, pb.y)
        } else {
            Point::new(pb.x, mid.y)
        };
        let mut path = BezPath::new();
        path.move_to(pa);
        if rounded {
            let r = 3.0_f64.min((pb - pa).hypot() / 4.0);
            let dir1 = (c1 - pa).normalize();
            let dir2 = (c2 - c1).normalize();
            let dir3 = (pb - c2).normalize();
            path.line_to(c1 - dir1 * r);
            path.quad_to(c1, c1 + dir2 * r);
            path.line_to(c2 - dir2 * r);
            path.quad_to(c2, c2 + dir3 * r);
            path.line_to(pb);
        } else {
            path.line_to(c1);
            path.line_to(c2);
            path.line_to(pb);
        }
        if let Some(id) = self.new_shape(ShapeKind::Path {
            path,
            closed: false,
        }) {
            let _ = self.engine.run(&Command::SetShapeName {
                shape: id,
                name: Some("Connector".into()),
            });
            let _ = self.engine.run(&Command::SetFill {
                shapes: vec![id],
                fill: Fill::None,
            });
            self.select(vec![id]);
        }
    }

    /// Area Fill: flood the enclosed area under the click (all visible
    /// outlines count as walls) and create a filled object from it.
    pub fn area_fill_at(&mut self, p: Point) {
        let page = self.page_rect();
        if !page.contains(p) {
            return;
        }
        let res = 4.0; // pixels per mm
        let w = (page.width() * res).ceil() as usize;
        let h = (page.height() * res).ceil() as usize;
        if w == 0 || h == 0 || w * h > 40_000_000 {
            return;
        }
        // Walls: every outline (stroked or not) of every visible object.
        let mut walls = vec![false; w * h];
        let shapes: Vec<Shape> = self
            .doc()
            .layers_for_page(self.page)
            .unwrap_or_default()
            .iter()
            .filter(|l| l.visible)
            .flat_map(|l| l.shapes.clone())
            .collect();
        let mut pts: Vec<Point> = Vec::new();
        let stamp = |a: Point, b: Point, walls: &mut Vec<bool>| {
            let n = ((b - a).hypot() * res).ceil().max(1.0) as usize;
            for i in 0..=n {
                let q = a + (b - a) * (i as f64 / n as f64);
                let x = (q.x * res) as i64;
                let y = ((page.height() - q.y) * res) as i64;
                if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                    walls[y as usize * w + x as usize] = true;
                }
            }
        };
        fn collect(s: &Shape, parent: Affine, out: &mut Vec<BezPath>) {
            let t = parent * s.transform;
            match &s.kind {
                ShapeKind::Group { children } => children.iter().for_each(|c| collect(c, t, out)),
                _ => out.push(t * s.local_path()),
            }
        }
        let mut paths = Vec::new();
        for s in &shapes {
            collect(s, Affine::IDENTITY, &mut paths);
        }
        for path in paths {
            pts.clear();
            let mut start: Option<Point> = None;
            let mut last: Option<Point> = None;
            kurbo::flatten(path.elements().iter().copied(), 0.05, |el| match el {
                kurbo::PathEl::MoveTo(q) => {
                    start = Some(q);
                    last = Some(q);
                }
                kurbo::PathEl::LineTo(q) => {
                    if let Some(l) = last {
                        stamp(l, q, &mut walls);
                    }
                    last = Some(q);
                }
                kurbo::PathEl::ClosePath => {
                    if let (Some(l), Some(s)) = (last, start) {
                        stamp(l, s, &mut walls);
                    }
                }
                _ => {}
            });
        }
        // Flood fill from the click.
        let sx = (p.x * res) as usize;
        let sy = ((page.height() - p.y) * res) as usize;
        if sx >= w || sy >= h || walls[sy * w + sx] {
            return;
        }
        let mut filled = vec![false; w * h];
        let mut stack = vec![(sx, sy)];
        let mut count = 0usize;
        while let Some((x, y)) = stack.pop() {
            let i = y * w + x;
            if filled[i] || walls[i] {
                continue;
            }
            filled[i] = true;
            count += 1;
            if x > 0 {
                stack.push((x - 1, y));
            }
            if x + 1 < w {
                stack.push((x + 1, y));
            }
            if y > 0 {
                stack.push((x, y - 1));
            }
            if y + 1 < h {
                stack.push((x, y + 1));
            }
        }
        if count == 0 || count as f64 > (w * h) as f64 * 0.98 {
            // Leaked to the whole page: nothing enclosed here.
            self.status = tr("status.area_fill_open");
            return;
        }
        // Include the wall pixels touching the region so the fill meets the outlines.
        let mut region = filled.clone();
        for y in 0..h {
            for x in 0..w {
                if walls[y * w + x] {
                    let near = [
                        (x.wrapping_sub(1), y),
                        (x + 1, y),
                        (x, y.wrapping_sub(1)),
                        (x, y + 1),
                    ];
                    if near
                        .iter()
                        .any(|(nx, ny)| *nx < w && *ny < h && filled[ny * w + nx])
                    {
                        region[y * w + x] = true;
                    }
                }
            }
        }
        let rings = crate::trace::contours(&region, w, h);
        let mut path = BezPath::new();
        for ring in rings {
            let simplified = tracedraw_core::geometry::simplify(&ring, 1.0);
            if simplified.len() < 3 {
                continue;
            }
            let pts: Vec<Point> = simplified
                .iter()
                .map(|q| Point::new(q.x / res, page.height() - q.y / res))
                .collect();
            path.extend(tracedraw_core::geometry::polyline_path(&pts, true));
        }
        if path.elements().is_empty() {
            return;
        }
        let Some(layer) = self.active_layer() else {
            return;
        };
        let id = self.engine.new_shape_id();
        let mut s = Shape::new(id, ShapeKind::Path { path, closed: true });
        s.fill = match &self.default_fill {
            Fill::None => Fill::Solid(self.area_fill_color),
            other => other.clone(),
        };
        s.stroke = self.default_stroke.clone();
        s.name = Some("Area Fill".into());
        self.run(Command::AddShape { layer, shape: s });
        self.select(vec![id]);
    }

    pub fn apply_mesh_fill(&mut self, id: ShapeId) {
        let Some(s) = self.doc().find_shape(id).cloned() else {
            return;
        };
        let b = s.local_path().bounding_box();
        let base = s.fill.preview_color().unwrap_or(Color::WHITE);
        let mesh =
            tracedraw_core::style::Mesh::new(b, self.mesh_rows.max(1), self.mesh_cols.max(1), base);
        self.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Mesh(mesh),
        });
        self.set_tool(Tool::Shape);
    }

    /// Effect nodes (envelope, perspective, mesh) under the pointer, for the
    /// Shape tool: (shape, node index).
    pub fn effect_node_at(&self, p: Point) -> Option<(ShapeId, usize)> {
        let tol = 5.0 / self.view.zoom as f64;
        for s in self.selected_shapes() {
            for (i, q) in effect_nodes(&s) {
                if (q - p).hypot() <= tol {
                    return Some((s.id, i));
                }
            }
        }
        None
    }

    /// Drag an effect node; returns true when handled.
    pub fn effect_node_input(&mut self, response: &Response, p: Point) -> bool {
        if response.clicked_by(PointerButton::Primary) {
            if let Some(hit) = self.effect_node_at(p) {
                self.selected_effect_node = Some(hit);
                return true;
            }
        }
        if response.double_clicked() {
            // Double-click inside a mesh adds a grid line there.
            for s in self.selected_shapes() {
                if let Fill::Mesh(m) = &s.fill {
                    let lp = s.transform.inverse() * p;
                    let mut m2 = m.clone();
                    m2.add_line(lp);
                    self.run(Command::SetFill {
                        shapes: vec![s.id],
                        fill: Fill::Mesh(m2),
                    });
                    return true;
                }
            }
        }
        if response.drag_started_by(PointerButton::Primary) {
            if let Some(hit) = self.effect_node_at(p) {
                self.effect_node_drag = Some(hit);
                self.selected_effect_node = Some(hit);
                self.drag = Drag::Node { last: p };
                return true;
            }
        }
        let Some((id, index)) = self.effect_node_drag else {
            return false;
        };
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::Node { last } = self.drag {
                let delta = p - last;
                if let Some(s) = self.doc().find_shape(id).cloned() {
                    let inv = s.transform.inverse();
                    let ld = (inv * (Point::ZERO + delta)) - (inv * Point::ZERO);
                    let mut effects = s.effects.clone();
                    let mut fill = s.fill.clone();
                    move_effect_node(&mut effects, &mut fill, index, ld);
                    if self.engine.undo_label() == Some("Edit Effect") {
                        let _ = self.engine.undo();
                    }
                    let _ = self
                        .engine
                        .run_with_label(&Command::SetEffects { shape: id, effects }, "Edit Effect");
                    if fill != s.fill {
                        let _ = self.engine.run_with_label(
                            &Command::SetFill {
                                shapes: vec![id],
                                fill,
                            },
                            "Edit Effect",
                        );
                    }
                }
                self.drag = Drag::Node { last: p };
            }
            return true;
        }
        if response.drag_stopped() {
            self.effect_node_drag = None;
            self.drag = Drag::None;
            return true;
        }
        false
    }

    /// Right-click: remember where, the UI draws the context menu.
    pub fn open_context_menu(&mut self, screen: egui::Pos2, p: Point) {
        if self.tool == Tool::Pick || self.tool == Tool::FreeformPick {
            if let Some(id) = self.hit_test(p) {
                if !self.selection.contains(&id) {
                    self.select(vec![id]);
                }
            }
        }
        self.context_menu = Some((screen, p));
    }
}

/// Page-space positions of the editable nodes of a shape's effects and
/// mesh fill, with a global index: envelope 0..8, perspective 100..104,
/// mesh 200 + k.
pub fn effect_nodes(s: &Shape) -> Vec<(usize, Point)> {
    let mut out = Vec::new();
    for e in &s.effects {
        match e {
            Effect::Envelope { nodes, .. } => {
                for (i, n) in nodes.iter().enumerate() {
                    out.push((i, s.transform * *n));
                }
            }
            Effect::Perspective { corners } => {
                for (i, c) in corners.iter().enumerate() {
                    out.push((100 + i, s.transform * *c));
                }
            }
            _ => {}
        }
    }
    if let Fill::Mesh(m) = &s.fill {
        for (k, n) in m.nodes.iter().enumerate() {
            out.push((200 + k, s.transform * n.pos));
        }
    }
    out
}

fn move_effect_node(effects: &mut [Effect], fill: &mut Fill, index: usize, d: Vec2) {
    for e in effects.iter_mut() {
        match e {
            Effect::Envelope { nodes, .. } if index < 100 => {
                if let Some(n) = nodes.get_mut(index) {
                    *n += d;
                }
            }
            Effect::Perspective { corners } if (100..104).contains(&index) => {
                corners[index - 100] += d;
            }
            _ => {}
        }
    }
    if let Fill::Mesh(m) = fill {
        if index >= 200 {
            if let Some(n) = m.nodes.get_mut(index - 200) {
                n.pos += d;
            }
        }
    }
}

/// Pull (amount > 0) or push (amount < 0) points within `radius` of `at`.
fn attract_repel(path: &BezPath, at: Point, radius: f64, amount: f64) -> BezPath {
    use tracedraw_core::geometry::PathEl;
    let f = |p: Point| -> Point {
        let v = at - p;
        let d = v.hypot();
        if d > radius || d < 1e-9 {
            return p;
        }
        let k = (1.0 - d / radius) * amount;
        p + v * (k / d).min(0.9)
    };
    let mut out = BezPath::new();
    for el in path.elements() {
        out.push(match *el {
            PathEl::MoveTo(p) => PathEl::MoveTo(f(p)),
            PathEl::LineTo(p) => PathEl::LineTo(f(p)),
            PathEl::QuadTo(a, p) => PathEl::QuadTo(f(a), f(p)),
            PathEl::CurveTo(a, b, p) => PathEl::CurveTo(f(a), f(b), f(p)),
            PathEl::ClosePath => PathEl::ClosePath,
        });
    }
    out
}

/// Jagged edge within `radius` of `at`.
fn roughen(path: &BezPath, at: Point, radius: f64, amount: f64) -> BezPath {
    let pts = tracedraw_core::effects::resample(path, 200);
    if pts.len() < 3 {
        return path.clone();
    }
    let mut seed = 17u32;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 1000) as f64 / 500.0 - 1.0
    };
    let n = pts.len();
    let out: Vec<Point> = (0..n)
        .map(|i| {
            let p = pts[i];
            let d = (p - at).hypot();
            if d > radius {
                return p;
            }
            let prev = pts[(i + n - 1) % n];
            let next = pts[(i + 1) % n];
            let t = (next - prev).normalize();
            let nrm = Vec2::new(-t.y, t.x);
            p + nrm * (rnd() * amount * (1.0 - d / radius))
        })
        .collect();
    tracedraw_core::geometry::polyline_path(&out[..n - 1], true)
}

pub fn _unused(_: Rect) {}

/// One drag step of the Free Transform tool: the affine that takes the
/// selection from the pointer at `last` to the pointer at `p`, about the
/// press point `center`. `first` is true for the first step of a drag.
pub fn free_transform_step(
    mode: crate::app::FreeTransformMode,
    center: Point,
    last: Point,
    p: Point,
    first: bool,
) -> Affine {
    use crate::app::FreeTransformMode as M;
    let about =
        |t: Affine| Affine::translate(center.to_vec2()) * t * Affine::translate(-center.to_vec2());
    match mode {
        M::Scale => {
            // Scale about the press point by the ratio of distances.
            let d0 = (last - center).hypot().max(1e-6);
            let d1 = (p - center).hypot().max(1e-6);
            about(Affine::scale(d1 / d0))
        }
        M::Skew => {
            // Horizontal motion skews along x, vertical along y.
            let d = p - last;
            about(Affine::skew(d.x / 50.0, d.y / 50.0))
        }
        M::Reflection => {
            // Mirror across the line from the press point through the
            // pointer. The first step reflects; later steps turn the mirror
            // line, which is a rotation by twice the angle change.
            let a1 = (p - center).atan2();
            if first {
                let (sn, cs) = (2.0 * a1).sin_cos();
                about(Affine::new([cs, sn, sn, -cs, 0.0, 0.0]))
            } else {
                let a0 = (last - center).atan2();
                Affine::rotate_about(2.0 * (a1 - a0), center)
            }
        }
        M::Rotation => {
            let a0 = (last - center).atan2();
            let a1 = (p - center).atan2();
            Affine::rotate_about(a1 - a0, center)
        }
    }
}

#[cfg(test)]
mod free_transform_tests {
    use super::free_transform_step;
    use crate::app::FreeTransformMode as M;
    use tracedraw_core::geometry::Point;

    fn close(a: Point, b: Point) -> bool {
        (a - b).hypot() < 1e-6
    }

    #[test]
    fn reflection_mirrors_across_the_pointer_line_and_follows_it() {
        let c = Point::new(10.0, 10.0);
        // Mirror line along x: (20, 15) lands on (20, 5).
        let t = free_transform_step(M::Reflection, c, c, Point::new(30.0, 10.0), true);
        assert!(close(t * Point::new(20.0, 15.0), Point::new(20.0, 5.0)));
        // Turning the line by 45 degrees rotates the already mirrored
        // object by 90 degrees: (20, 5) goes to (15, 20).
        let t2 = free_transform_step(
            M::Reflection,
            c,
            Point::new(30.0, 10.0),
            Point::new(30.0, 30.0),
            false,
        );
        assert!(close(t2 * Point::new(20.0, 5.0), Point::new(15.0, 20.0)));
        // The composite equals a direct reflection across the 45 degree line.
        let direct = free_transform_step(M::Reflection, c, c, Point::new(30.0, 30.0), true);
        assert!(close(
            (t2 * t) * Point::new(20.0, 15.0),
            direct * Point::new(20.0, 15.0)
        ));
    }

    #[test]
    fn scale_rotation_and_skew_keep_the_press_point_fixed() {
        let c = Point::new(5.0, 5.0);
        for m in [M::Scale, M::Rotation, M::Skew] {
            let t = free_transform_step(m, c, Point::new(15.0, 5.0), Point::new(15.0, 15.0), true);
            assert!(close(t * c, c), "{m:?}");
        }
        let t = free_transform_step(
            M::Scale,
            c,
            Point::new(15.0, 5.0),
            Point::new(25.0, 5.0),
            true,
        );
        assert!(close(t * Point::new(15.0, 5.0), Point::new(25.0, 5.0)));
        let t = free_transform_step(
            M::Rotation,
            c,
            Point::new(15.0, 5.0),
            Point::new(5.0, 15.0),
            true,
        );
        assert!(close(t * Point::new(15.0, 5.0), Point::new(5.0, 15.0)));
    }
}
