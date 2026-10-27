//! Input for the tools added with the full toolbox, the pending "click an
//! object" operations from menus, effect-node editing with the Shape tool,
//! and the right-click context menu trigger.

use crate::app::{App, Drag};
use crate::i18n::tr;
use crate::tools::Tool;
use egui::{Modifiers, PointerButton, Response};
use tracedraw_core::{
    document::{Shape, ShapeKind},
    geometry::{Affine, BezPath, Point, Rect, Vec2},
    live::Effect,
    Command, Fill, ShapeId, Stroke,
};

impl App {
    /// Menu operations waiting for a click on an object. Returns true when
    /// the click was consumed.
    pub fn pending_click(&mut self, response: &Response, p: Point) -> bool {
        let waiting = self.pending_order.is_some()
            || self.pending_copy_properties
            || self.pending_shaping.is_some()
            || self.pending_merge_frame
            || self.pending_copy_effect.is_some()
            || self.pending_copy_envelope
            || self.pending_clone_effect.is_some()
            || self.pending_blend_path
            || self.pending_text_path
            || self.pending_palette_sample;
        if !waiting {
            return false;
        }
        if response.secondary_clicked() {
            self.pending_order = None;
            self.pending_copy_properties = false;
            self.pending_shaping = None;
            self.pending_merge_frame = false;
            self.pending_copy_effect = None;
            self.pending_copy_envelope = false;
            self.pending_clone_effect = None;
            self.pending_blend_path = false;
            self.pending_text_path = false;
            self.pending_palette_sample = false;
            self.status = tr("status.cancelled");
            return true;
        }
        if !response.clicked_by(PointerButton::Primary) {
            return true;
        }
        if self.pending_palette_sample {
            // The document palette's eyedropper: the colour under the
            // pointer joins the palette; Ctrl keeps sampling.
            let more = response.ctx.input(|i| i.modifiers.ctrl);
            self.pending_palette_sample = more;
            if let Some(c) = self.sample_color_at(p, false) {
                self.add_to_document_palette(c);
            }
            return true;
        }
        if self.pending_merge_frame {
            // The shape under the pointer, also one inside a group.
            self.pending_merge_frame = false;
            match self.hits_at(p, true).first().copied() {
                Some(id) => {
                    self.select(vec![id]);
                    self.place_merge_field_inside();
                }
                None => self.status = tr("status.cancelled"),
            }
            return true;
        }
        if self.pending_shaping.is_some() {
            // The target is found under the selected objects too.
            let target = self.hit_test_where(p, |id| self.selection.contains(&id));
            self.shaping_onto(target);
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
        } else if self.pending_copy_envelope {
            self.pending_copy_envelope = false;
            if let Some(t) = target {
                self.copy_envelope_from(t);
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
        } else if self.pending_text_path {
            self.pending_text_path = false;
            // The path the preview showed, near the pointer or under it.
            if let Some(t) = self.text_path_target(p).or(target) {
                self.fit_text_to_picked_path(t, Some(p));
                return true;
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
                self.brush_input(response, p, mods)
            }
            Tool::VariableOutline => self.variable_outline_input(response, p, mods),
            Tool::SegmentDelete => {
                if response.clicked_by(PointerButton::Primary) {
                    self.delete_virtual_segment_at(p);
                }
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
            Tool::HorizontalVerticalDimension | Tool::SegmentDimension | Tool::AngularDimension => {
                self.dimension_input(response, p)
            }
            Tool::Callout => self.callout_input(response, p),
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
            Tool::Envelope => self.envelope_tool_input(response, p),
            Tool::Extrude => self.extrude_input(response, p),
            Tool::Distort => self.distort_input(response, p),
            Tool::AreaFill => {
                if response.clicked_by(PointerButton::Primary) {
                    self.area_fill_at(p);
                }
            }
            Tool::MeshFill => self.mesh_tool_input(response, p),
            Tool::OutlinePen => {
                // The tool opens the Outline Pen dialog and gives the
                // canvas back to the tool before it.
                let back = match self.previous_tool {
                    Tool::OutlinePen | Tool::OutlineColor => Tool::Pick,
                    t => t,
                };
                self.set_tool(back);
                self.open_outline_pen();
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

    /// Segment Delete: remove the segment of a curve under the click.
    /// Shape Recognition: recognise the sketched stroke as a basic shape.
    /// Sketch: smooth the stroke and merge it with a selected curve
    /// whose end is near the stroke's start.
    pub fn finish_sketch(&mut self, points: Vec<Point>) {
        let sketch = self.sketch;
        let pts = tracedraw_core::geometry::simplify(&points, sketch.tolerance());
        if pts.len() < 2 {
            return;
        }
        let path = tracedraw_core::geometry::smooth_path(&pts, false);
        if let Some(sel) = self
            .selected_shapes()
            .into_iter()
            .filter(|_| sketch.join)
            .find(|s| matches!(s.kind, ShapeKind::Path { closed: false, .. }))
        {
            let existing = sel.page_path();
            let end = existing.elements().iter().rev().find_map(|el| match el {
                tracedraw_core::geometry::PathEl::LineTo(q)
                | tracedraw_core::geometry::PathEl::CurveTo(_, _, q) => Some(*q),
                _ => None,
            });
            if let Some(e) = end {
                if (e - pts[0]).hypot() < sketch.join_distance {
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

    /// Right-angle and rounded connectors: an elbow path between the
    /// facing sides of two objects.
    /// The right-angle connector tools' drop: a connector at right angles
    /// (`rounded`: with rounded corners) to the object under the drop.
    pub fn finish_elbow_connector(
        &mut self,
        from: ShapeId,
        start: Point,
        at: Point,
        rounded: bool,
    ) {
        use crate::connectors::ConnectorKind;
        if let Some(to) = self.hit_test_inside(at).filter(|id| *id != from) {
            let kind = if rounded {
                ConnectorKind::Rounded
            } else {
                ConnectorKind::RightAngle
            };
            self.connect(from, start, to, at, kind);
        }
    }

    /// Area Fill: find the enclosed area under the click (all visible
    /// outlines count as walls) and create a filled object from it. The
    /// area can be anywhere, on the page or off it on the desktop, and a
    /// drawing of any size (#499).
    pub fn area_fill_at(&mut self, p: Point) {
        let shapes: Vec<Shape> = self
            .doc()
            .layers_for_page(self.page)
            .unwrap_or_default()
            .iter()
            .filter(|l| l.visible)
            .flat_map(|l| l.shapes.clone())
            .collect();
        fn collect(s: &Shape, parent: Affine, out: &mut Vec<BezPath>) {
            let t = parent * s.transform;
            if !s.effects.is_empty() {
                // What the object draws: its warped outline and the
                // objects its effects add.
                let mut on_page = s.clone();
                on_page.transform_by(parent);
                let ev = tracedraw_core::live::evaluate(&on_page);
                for part in std::iter::once(ev.main).chain(ev.below).chain(ev.above) {
                    out.push(part.page_path());
                }
                return;
            }
            match &s.kind {
                ShapeKind::Group { children } => children.iter().for_each(|c| collect(c, t, out)),
                _ => out.push(t * s.local_path()),
            }
        }
        let mut paths = Vec::new();
        for s in &shapes {
            collect(s, Affine::IDENTITY, &mut paths);
        }
        // The area the outlines close off around the click, its sides
        // their own curves (#734); a click in an open area or on an
        // outline makes nothing.
        let path = match tracedraw_core::shaping::enclosed_area(&paths, p) {
            tracedraw_core::shaping::Enclosed::Area(path) => path,
            tracedraw_core::shaping::Enclosed::Open => {
                self.status = tr("status.area_fill_open");
                return;
            }
            tracedraw_core::shaping::Enclosed::OnOutline => return,
        };
        if path.elements().is_empty() {
            return;
        }
        let Some(layer) = self.active_layer() else {
            return;
        };
        let id = self.engine.new_shape_id();
        let mut s = Shape::new(id, ShapeKind::Path { path, closed: true });
        // The bar's choices: the default fill (a colour when the default
        // is none, so the area shows), its own colour, or none; the
        // default outline, its own, or none.
        s.fill = match self.area_fill_mode {
            1 => Fill::Solid(self.area_fill_color),
            2 => Fill::None,
            _ => match self.default_fill() {
                Fill::None => Fill::Solid(self.area_fill_color),
                other => other,
            },
        };
        s.stroke = match self.area_outline_mode {
            1 => Some(Stroke::new(
                self.area_outline_color,
                self.area_outline_width.max(0.0),
            )),
            2 => None,
            _ => self.default_stroke(),
        };
        s.name = Some("Area Fill".into());
        self.run(Command::AddShape { layer, shape: s });
        self.select(vec![id]);
    }

    /// Effect nodes (perspective, mesh) under the pointer, for the Shape
    /// and Mesh Fill tools: (shape, node index).
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
            // Double-click inside a mesh: a node there, on new grid lines
            // through the point (#804).
            for s in self.selected_shapes() {
                if let Fill::Mesh(m) = &s.fill {
                    let inv = s.transform.inverse();
                    let lp = inv * p;
                    // Four pixels on screen, in the mesh's own space.
                    let tol = ((inv * Point::new(4.0 / self.view.zoom as f64, 0.0))
                        - (inv * Point::ZERO))
                        .hypot();
                    let mut m2 = m.clone();
                    if m2.add_lines_at(lp, tol) {
                        let node = mesh_node_near(&m2, lp, tol);
                        self.run(Command::SetFill {
                            shapes: vec![s.id],
                            fill: Fill::Mesh(m2),
                        });
                        if let Some(i) = node {
                            self.selected_effect_node =
                                Some((s.id, crate::mesh_tool::MESH_NODE_BASE + i));
                        }
                    }
                    return true;
                }
            }
        }
        if response.drag_started_by(PointerButton::Primary) {
            if let Some(hit) = self.effect_node_at(p) {
                self.effect_node_drag = Some(hit);
                self.selected_effect_node = Some(hit);
                self.effect_node_stepped = false;
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
                    // One undo step for the whole drag.
                    if self.effect_node_stepped && self.engine.undo_label() == Some("Edit Effect") {
                        let _ = self.engine.undo();
                    }
                    self.effect_node_stepped = true;
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
        self.context_locked = None;
        if self.tool == Tool::Pick || self.tool == Tool::FreeformPick {
            if let Some(id) = self.locked_at(p) {
                // A locked object on top: its own menu, to unlock it.
                self.context_locked = Some(id);
            } else if let Some(id) = self.hit_test(p) {
                if !self.selection.contains(&id) {
                    self.select(vec![id]);
                }
            } else if let Some(i) = self.guide_at(p) {
                // A guideline under the pointer gets its own menu.
                if !self.selected_guides.contains(&i) {
                    self.select(Vec::new());
                    self.selected_guides = vec![i];
                }
            }
        }
        if self.tool == Tool::Shape {
            // A right click on a node takes it (unless it is chosen
            // already); on a segment, the node it ends at, so that the
            // menu's To Line and To Curve act on that segment; on another
            // object, that object.
            if let Some(hit) = self.node_at(p) {
                let key = (hit.shape, hit.index);
                if !self.node_selection.contains(&key) {
                    self.node_selection = vec![key];
                }
            } else if let Some((id, i, _)) = self.segment_at(p) {
                self.node_selection = vec![(id, i)];
            } else if let Some(id) = self.hit_test(p) {
                if !self.selection.contains(&id) {
                    self.select(vec![id]);
                }
            }
        }
        self.context_menu = Some((screen, p));
    }
}

/// The node of `m` within `tol` of `p` (both in the mesh's space).
fn mesh_node_near(m: &tracedraw_core::Mesh, p: Point, tol: f64) -> Option<usize> {
    m.nearest(p).filter(|i| {
        m.nodes
            .get(*i)
            .is_some_and(|n| (n.pos - p).hypot() <= tol.max(1e-6))
    })
}

/// Page-space positions of the editable nodes of a shape's perspective
/// and mesh fill, with a global index: perspective 100..104, mesh 200 + k
/// (envelopes have their own editing, see `envelope_edit`).
pub fn effect_nodes(s: &Shape) -> Vec<(usize, Point)> {
    let mut out = Vec::new();
    for e in &s.effects {
        if let Effect::Perspective { corners } = e {
            for (i, c) in corners.iter().enumerate() {
                out.push((100 + i, s.transform * *c));
            }
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
        if let Effect::Perspective { corners } = e {
            if let Some(c) = index.checked_sub(100).and_then(|i| corners.get_mut(i)) {
                *c += d;
            }
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
