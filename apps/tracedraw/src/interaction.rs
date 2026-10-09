//! Pointer and keyboard handling on the canvas, per tool.

use crate::app::{App, CurveInProgress, Drag, Handle};
use crate::tools::Tool;
use egui::{Key, Modifiers, PointerButton, Response};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, Point, Rect, Vec2},
    Color, Command, Fill, ShapeId,
};

const HANDLE_PX: f32 = 7.0;

impl App {
    pub fn hit_test(&self, p: Point) -> Option<ShapeId> {
        let doc = self.doc();
        let page = doc.page(self.page).ok()?;
        let tol = 3.0 / self.view.zoom as f64;
        for layer in page.layers.iter().rev() {
            if !layer.visible || layer.locked {
                continue;
            }
            for s in layer.shapes.iter().rev() {
                if s.locked || !s.visible {
                    continue;
                }
                let b = s.bounds().inflate(tol, tol);
                if !b.contains(p) {
                    continue;
                }
                // Filled objects hit anywhere inside; unfilled ones only near the outline.
                if !matches!(s.fill, Fill::None)
                    || matches!(
                        s.kind,
                        ShapeKind::Text { .. } | ShapeKind::Group { .. } | ShapeKind::Table(_)
                    )
                {
                    return Some(s.id);
                }
                if crate::canvas::distance_to_path(&s.page_path(), p) <= tol * 1.5 {
                    return Some(s.id);
                }
            }
        }
        None
    }

    /// Screen-space handle positions for the selection box.
    pub fn handle_positions(&self, b: Rect) -> [(Handle, egui::Pos2); 8] {
        let r = self.view.rect_to_screen(b);
        [
            (Handle::NW, r.left_top()),
            (Handle::N, r.center_top()),
            (Handle::NE, r.right_top()),
            (Handle::W, r.left_center()),
            (Handle::E, r.right_center()),
            (Handle::SW, r.left_bottom()),
            (Handle::S, r.center_bottom()),
            (Handle::SE, r.right_bottom()),
        ]
    }

    fn handle_at(&self, screen: egui::Pos2) -> Option<Handle> {
        let b = self.selection_bounds()?;
        self.handle_positions(b)
            .iter()
            .find(|(_, p)| p.distance(screen) <= HANDLE_PX)
            .map(|(h, _)| *h)
    }

    pub fn canvas_input(&mut self, response: &Response, mods: Modifiers) {
        let hover = response.hover_pos();
        let pointer = hover.map(|h| self.view.to_page(h));
        self.pointer_page = pointer;
        let Some(p) = pointer else {
            if response.drag_stopped() {
                self.end_drag();
            }
            return;
        };
        let mut screen = hover.unwrap_or_default();
        // A drag starts after the pointer moved a few pixels; hit-test at the
        // press position so small targets like nodes and handles are caught.
        let mut p = p;
        if response.drag_started() {
            if let Some(origin) = response.ctx.input(|i| i.pointer.press_origin()) {
                screen = origin;
                p = self.view.to_page(origin);
            }
        }

        // Middle button always pans.
        if response.dragged_by(PointerButton::Middle) {
            self.view.pan(response.drag_delta());
            return;
        }

        // Right click: context menu (drawn by the UI at this position).
        if response.secondary_clicked() && !self.pending_clip_frame {
            if self.pending_click(response, p) {
                return;
            }
            self.open_context_menu(screen, p);
            return;
        }
        if self.pending_click(response, p) {
            return;
        }

        if self.pending_clip_frame {
            if response.clicked_by(PointerButton::Primary) {
                self.pending_clip_frame = false;
                if let Some(frame) = self.hit_test(p) {
                    let contents: Vec<ShapeId> = self
                        .selection
                        .iter()
                        .copied()
                        .filter(|id| *id != frame)
                        .collect();
                    if contents.is_empty() {
                        self.status = crate::i18n::tr("status.clip_frame_select_contents");
                    } else {
                        self.run(Command::PlaceInside { contents, frame });
                        self.select(vec![frame]);
                    }
                } else {
                    self.status = crate::i18n::tr("status.clip_frame_cancelled");
                }
            }
            if response.secondary_clicked() {
                self.pending_clip_frame = false;
            }
            return;
        }
        match self.tool {
            Tool::Pick => self.pick_input(response, p, screen, mods),
            Tool::FreeformPick => self.effects_input(response, p, mods),
            Tool::Shape => {
                if !self.effect_node_input(response, p) {
                    self.shape_input(response, p, mods)
                }
            }
            Tool::FreeTransform
            | Tool::AttractRepel
            | Tool::Smudge
            | Tool::Roughen
            | Tool::SegmentDelete
            | Tool::ShapeRecognition
            | Tool::Sketch
            | Tool::HorizontalVerticalDimension
            | Tool::AngularDimension
            | Tool::SegmentDimension
            | Tool::Callout
            | Tool::RightAngleConnector
            | Tool::RoundedConnector
            | Tool::AnchorEditing
            | Tool::BlockShadow
            | Tool::Envelope
            | Tool::Extrude
            | Tool::Distort
            | Tool::AreaFill
            | Tool::MeshFill
            | Tool::OutlinePen
            | Tool::OutlineColor => self.tools3_input(response, p, mods),
            Tool::Zoom => self.zoom_input(response, p, screen, mods),
            Tool::Pan => {
                if response.dragged() {
                    self.view.pan(response.drag_delta());
                }
            }
            t if t.is_box_tool() => self.box_input(response, p, mods),
            Tool::ThreePointRectangle | Tool::ThreePointEllipse | Tool::ThreePointCurve => {
                self.three_point_input(response, p)
            }
            Tool::Freehand => self.freehand_input(response, p),
            Tool::Bezier | Tool::Pen | Tool::Polyline | Tool::TwoPointLine | Tool::BSpline => {
                self.curve_input(response, p, mods)
            }
            Tool::Text => self.text_input(response, p),
            Tool::ColorEyedropper | Tool::AttributesEyedropper => {
                if response.clicked() {
                    match self.eyedropper_color {
                        None => {
                            if let Some(id) = self.hit_test(p) {
                                if let Ok((_, s)) = self.doc().shape(id) {
                                    if let Fill::Solid(c) = s.fill {
                                        self.eyedropper_color = Some(c);
                                        self.status = crate::i18n::trf(
                                            "status.sampled_color",
                                            &[("c", &crate::app::color_description(c))],
                                        );
                                    }
                                }
                            }
                        }
                        Some(c) => {
                            if let Some(id) = self.hit_test(p) {
                                self.run(Command::SetFill {
                                    shapes: vec![id],
                                    fill: Fill::Solid(c),
                                });
                            }
                        }
                    }
                }
            }
            Tool::InteractiveFill => self.fill_input(response, p),
            Tool::Contour
            | Tool::Crop
            | Tool::Knife
            | Tool::Spiral
            | Tool::CommonShapes
            | Tool::Table
            | Tool::BrushStrokes
            | Tool::ParallelDimension
            | Tool::Connector => self.tools2_input(response, p, mods),
            Tool::Blend | Tool::Smooth | Tool::Smear | Tool::Twirl => {
                self.effects_input(response, p, mods)
            }
            Tool::DropShadow => {
                if response.drag_started_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test(p) {
                        self.select(vec![id]);
                        self.drag = Drag::Shadow {
                            shape: id,
                            start: p,
                        };
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::Shadow { shape, start } = self.drag.clone() {
                        let offset = p - start;
                        let mut sh = self
                            .doc()
                            .shape(shape)
                            .ok()
                            .and_then(|(_, s)| s.shadow)
                            .unwrap_or(self.shadow_default);
                        sh.offset = offset;
                        self.select(vec![shape]);
                        self.set_shadow(Some(sh), true);
                        self.drag = Drag::Shadow { shape, start };
                    }
                }
                if response.clicked_by(PointerButton::Primary) {
                    match self.hit_test(p) {
                        Some(id) => self.select(vec![id]),
                        None => self.select(Vec::new()),
                    }
                }
            }
            Tool::Transparency => {
                if response.drag_started_by(PointerButton::Primary) {
                    if let Some(id) = self.hit_test(p) {
                        self.select(vec![id]);
                        self.drag = Drag::FillGradient {
                            shape: id,
                            start: p,
                            current: p,
                        };
                    }
                }
                if response.dragged_by(PointerButton::Primary) {
                    if let Drag::FillGradient { current, .. } = &mut self.drag {
                        *current = p;
                    }
                }
                if response.clicked() {
                    match self.hit_test(p) {
                        Some(id) => {
                            if !self.selection.contains(&id) {
                                self.select(vec![id]);
                            }
                        }
                        None => self.select(Vec::new()),
                    }
                }
            }
            Tool::Eraser => {
                if response.clicked() {
                    if let Some(id) = self.hit_test(p) {
                        self.run(Command::DeleteShapes { shapes: vec![id] });
                        self.selection.retain(|s| *s != id);
                    }
                }
            }
            _ => {
                if response.clicked() {
                    self.status = crate::i18n::trf(
                        "status.tool_not_implemented",
                        &[("t", &self.tool.name())],
                    );
                }
            }
        }

        if response.drag_stopped_by(PointerButton::Primary) {
            self.end_drag();
        }
    }

    fn pick_input(&mut self, response: &Response, p: Point, screen: egui::Pos2, mods: Modifiers) {
        if response.drag_started_by(PointerButton::Primary) {
            if let Some(h) = self.handle_at(screen) {
                let b = self.selection_bounds().unwrap_or(Rect::ZERO);
                if self.rotate_mode {
                    if h.is_corner() {
                        let c = b.center();
                        let a = (p - c).atan2();
                        self.drag = Drag::Rotate {
                            center: c,
                            start_angle: a,
                            current_angle: a,
                        };
                    }
                } else {
                    let anchor = match h {
                        Handle::N => Point::new(b.center().x, b.y0),
                        Handle::S => Point::new(b.center().x, b.y1),
                        Handle::E => Point::new(b.x0, b.center().y),
                        Handle::W => Point::new(b.x1, b.center().y),
                        Handle::NE => Point::new(b.x0, b.y0),
                        Handle::NW => Point::new(b.x1, b.y0),
                        Handle::SE => Point::new(b.x0, b.y1),
                        Handle::SW => Point::new(b.x1, b.y1),
                    };
                    self.drag = Drag::Scale {
                        handle: h,
                        anchor,
                        start_bounds: b,
                        current: p,
                    };
                }
                return;
            }
            if self.hit_test(p).is_none() {
                if let Some(gi) = self.guide_at(p) {
                    self.selected_guide = Some(gi);
                    self.drag = Drag::MoveGuide { index: gi };
                    return;
                }
            }
            match self.hit_test(p) {
                Some(id) => {
                    if mods.shift {
                        if !self.selection.contains(&id) {
                            self.selection.push(id);
                        }
                    } else if !self.selection.contains(&id) {
                        self.select(vec![id]);
                    }
                    self.drag = Drag::Move {
                        last: p,
                        total: Vec2::ZERO,
                        start_bounds: self.selection_bounds().unwrap_or(Rect::ZERO),
                    };
                }
                None => {
                    if !mods.shift {
                        self.select(Vec::new());
                    }
                    self.selected_guide = None;
                    self.drag = Drag::Marquee {
                        start: p,
                        current: p,
                    };
                }
            }
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::MoveGuide { index } = self.drag {
                let guide = match self
                    .doc()
                    .page(self.page)
                    .ok()
                    .and_then(|pg| pg.guides.get(index).copied())
                {
                    Some(g) => g.through(p),
                    None => return,
                };
                self.move_guide(index, guide);
                return;
            }
            let snapped_move = match &self.drag {
                Drag::Move {
                    last,
                    total,
                    start_bounds,
                } => {
                    let raw = *total + (p - *last);
                    Some((p, self.snap_move(*start_bounds, raw)))
                }
                _ => None,
            };
            match &mut self.drag {
                Drag::Move { last, total, .. } => {
                    if let Some((np, nt)) = snapped_move {
                        *last = np;
                        *total = nt;
                    }
                }
                Drag::Marquee { current, .. } | Drag::Scale { current, .. } => *current = p,
                Drag::Rotate {
                    center,
                    current_angle,
                    ..
                } => *current_angle = (p - *center).atan2(),
                _ => {}
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            if self.handle_at(screen).is_some() {
                return;
            }
            if self.hit_test(p).is_none() {
                if let Some(gi) = self.guide_at(p) {
                    self.selected_guide = Some(gi);
                    self.select(Vec::new());
                    return;
                }
            }
            self.selected_guide = None;
            match self.hit_test(p) {
                Some(id) if mods.shift => {
                    if let Some(i) = self.selection.iter().position(|s| *s == id) {
                        self.selection.remove(i);
                    } else {
                        self.selection.push(id);
                    }
                }
                Some(id) => {
                    if self.selection == vec![id] {
                        // Second click: toggle rotate/skew handles.
                        self.rotate_mode = !self.rotate_mode;
                    } else {
                        self.select(vec![id]);
                    }
                }
                None => self.select(Vec::new()),
            }
        }
        if response.double_clicked_by(PointerButton::Primary) {
            if let Some(id) = self.hit_test(p) {
                if let Ok((_, s)) = self.doc().shape(id) {
                    if matches!(s.kind, ShapeKind::Text { .. }) {
                        self.set_tool(Tool::Text);
                    } else {
                        self.select(vec![id]);
                        self.set_tool(Tool::Shape);
                    }
                }
            }
        }
    }

    fn shape_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        self.shape_tool_input(response, p, mods);
    }

    fn text_input(&mut self, response: &Response, p: Point) {
        if response.drag_started_by(PointerButton::Primary) {
            if self.text_edit.is_some() {
                self.finish_text();
            }
            self.drag = Drag::TextFrame {
                start: p,
                current: p,
            };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::TextFrame { current, .. } = &mut self.drag {
                *current = p;
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            match self.hit_test(p) {
                Some(id)
                    if matches!(
                        self.doc()
                            .shape(id)
                            .map(|(_, s)| matches!(s.kind, ShapeKind::Text { .. })),
                        Ok(true)
                    ) =>
                {
                    let text = match self.doc().shape(id).map(|(_, s)| s.kind.clone()) {
                        Ok(ShapeKind::Text { spans, .. }) => {
                            spans.iter().map(|s| s.text.as_str()).collect::<String>()
                        }
                        _ => String::new(),
                    };
                    self.finish_text();
                    self.text_edit = Some(crate::app::TextEdit { shape: id, text });
                    self.select(vec![id]);
                    self.sync_text_defaults_from(id);
                }
                _ => {
                    let p = self.snap_point(p);
                    self.start_text(p, None);
                }
            }
        }
    }

    fn zoom_input(&mut self, response: &Response, p: Point, screen: egui::Pos2, mods: Modifiers) {
        if response.drag_started_by(PointerButton::Primary) {
            self.drag = Drag::ZoomBox {
                start: p,
                current: p,
            };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::ZoomBox { current, .. } = &mut self.drag {
                *current = p;
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            self.view
                .zoom_at(screen, if mods.shift { 0.5 } else { 2.0 });
        }
        if response.secondary_clicked() {
            self.view.zoom_at(screen, 0.5);
        }
    }

    fn box_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        let p = self.snap_point(p);
        if response.drag_started_by(PointerButton::Primary) {
            self.drag = Drag::Box {
                start: p,
                current: p,
            };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::Box { start, current } = &mut self.drag {
                let mut q = p;
                if mods.ctrl {
                    // Constrain to a square/circle.
                    let d = q - *start;
                    let m = d.x.abs().max(d.y.abs());
                    q = *start + Vec2::new(m * d.x.signum(), m * d.y.signum());
                }
                if mods.shift {
                    // Draw from the centre.
                    let d = q - *start;
                    *current = *start + d;
                    return;
                }
                *current = q;
            }
        }
        if response.double_clicked_by(PointerButton::Primary) && self.tool == Tool::Rectangle {
            // the target design: double-click the rectangle tool draws a page frame.
            let r = self.page_rect();
            if let Some(id) = self.new_shape(ShapeKind::Rect {
                rect: r,
                radius: 0.0,
            }) {
                self.select(vec![id]);
            }
        }
    }

    /// 3-point rectangle, ellipse and curve: drag the base segment, then
    /// click the third point (Esc cancels, see `keyboard`).
    fn three_point_input(&mut self, response: &Response, p: Point) {
        let p = self.snap_point(p);
        if self.three_point_base.is_some() {
            if response.clicked_by(PointerButton::Primary) {
                if let Some((a, b)) = self.three_point_base.take() {
                    self.finish_three_point(a, b, p);
                }
            }
            return;
        }
        if response.drag_started_by(PointerButton::Primary) {
            self.drag = Drag::ThreePointBase {
                start: p,
                current: p,
            };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::ThreePointBase { current, .. } = &mut self.drag {
                *current = p;
            }
        }
    }

    fn freehand_input(&mut self, response: &Response, p: Point) {
        if response.drag_started_by(PointerButton::Primary) {
            self.drag = Drag::Freehand { points: vec![p] };
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::Freehand { points } = &mut self.drag {
                if points
                    .last()
                    .map(|l| (*l - p).hypot() > 0.2)
                    .unwrap_or(true)
                {
                    points.push(p);
                }
            }
        }
    }

    /// Bezier/Pen: click places a cusp node, click-and-drag pulls out a
    /// smooth handle. Polyline and 2-point line place straight segments.
    /// Double-click or Enter finishes; clicking the start node closes.
    fn curve_input(&mut self, response: &Response, p: Point, _mods: Modifiers) {
        let p = self.snap_point(p);
        let smooth = matches!(self.tool, Tool::Bezier | Tool::Pen | Tool::BSpline);
        if response.double_clicked_by(PointerButton::Primary) {
            self.finish_curve();
            return;
        }
        let start_close = self
            .curve
            .as_ref()
            .and_then(|c| c.nodes.first().map(|n| n.0))
            .map(|s| {
                (s - p).hypot() <= 5.0 / self.view.zoom as f64
                    && self
                        .curve
                        .as_ref()
                        .map(|c| c.nodes.len() >= 3)
                        .unwrap_or(false)
            })
            .unwrap_or(false);
        if response.drag_started_by(PointerButton::Primary)
            || response.clicked_by(PointerButton::Primary)
        {
            if start_close {
                // Close the curve onto its start node.
                if let Some(c) = self.curve.take() {
                    let mut path = c.path(None);
                    path.close_path();
                    if let Some(id) = self.new_shape(ShapeKind::Path { path, closed: true }) {
                        self.select(vec![id]);
                    }
                }
                return;
            }
            let c = self.curve.get_or_insert_with(|| CurveInProgress {
                nodes: Vec::new(),
                smooth,
                dragging_handle: false,
            });
            c.smooth = smooth;
            if response.drag_started_by(PointerButton::Primary) {
                c.nodes.push((p, None));
                c.dragging_handle = true;
            } else if !c.dragging_handle {
                c.nodes.push((p, None));
            }
            c.dragging_handle = response.drag_started_by(PointerButton::Primary);
            if self.tool == Tool::TwoPointLine && c.nodes.len() == 2 {
                self.finish_curve();
            }
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Some(c) = &mut self.curve {
                if c.dragging_handle && smooth {
                    if let Some(last) = c.nodes.last_mut() {
                        last.1 = Some(p);
                    }
                }
            }
        }
        if response.drag_stopped_by(PointerButton::Primary) {
            if let Some(c) = &mut self.curve {
                c.dragging_handle = false;
            }
        }
    }

    fn fill_input(&mut self, response: &Response, p: Point) {
        if response.drag_started_by(PointerButton::Primary) {
            if let Some(id) = self.hit_test(p) {
                self.select(vec![id]);
                self.drag = Drag::FillGradient {
                    shape: id,
                    start: p,
                    current: p,
                };
            }
        }
        if response.dragged_by(PointerButton::Primary) {
            if let Drag::FillGradient { current, .. } = &mut self.drag {
                *current = p;
            }
        }
        if response.clicked_by(PointerButton::Primary) {
            if let Some(id) = self.hit_test(p) {
                self.select(vec![id]);
                let fill = match &self.default_fill {
                    Fill::None => Fill::Solid(Color::cmyk_pct(0.0, 0.0, 0.0, 20.0)),
                    f => f.clone(),
                };
                self.run(Command::SetFill {
                    shapes: vec![id],
                    fill,
                });
            }
        }
    }

    pub fn end_drag(&mut self) {
        let drag = std::mem::replace(&mut self.drag, Drag::None);
        match drag {
            Drag::Anchor { .. } => {}
            Drag::Box { start, current } => {
                if !self.finish_tools2_box(start, current) {
                    self.create_box_shape(start, current);
                }
            }
            Drag::Connector { from, current, .. } if self.tool == Tool::Blend => {
                if let Some(to) = self.hit_test(current).filter(|id| *id != from) {
                    self.selection = vec![from, to];
                    let steps = self.blend_steps;
                    self.blend_selection(steps, 0.0, 0.0, 0.0);
                }
            }
            Drag::Connector {
                from,
                start,
                current,
            } if self.tool == Tool::RightAngleConnector => {
                self.finish_elbow_connector(from, start, current, false)
            }
            Drag::Connector {
                from,
                start,
                current,
            } if self.tool == Tool::RoundedConnector => {
                self.finish_elbow_connector(from, start, current, true)
            }
            Drag::Connector {
                from,
                start,
                current,
            } => self.finish_connector(from, start, current),
            Drag::Freehand { points } if self.tool == Tool::ShapeRecognition => {
                self.finish_shape_recognition(points)
            }
            Drag::Freehand { points } if self.tool == Tool::Sketch => {
                self.finish_sketch(points)
            }
            Drag::ThreePointBase { start, current } => {
                if (current - start).hypot() > 0.05 {
                    self.three_point_base = Some((start, current));
                }
            }
            Drag::TextFrame { start, current } => {
                let r = Rect::from_points(start, current);
                if r.width() > 2.0 && r.height() > 2.0 {
                    self.start_text(
                        Point::new(r.x0, r.y0),
                        Some(tracedraw_core::geometry::Size::new(r.width(), r.height())),
                    );
                }
            }
            Drag::Move { total, .. } => {
                if total.hypot() > 1e-6 {
                    self.transform_selection(Affine::translate(total));
                }
            }
            Drag::Scale { .. } | Drag::Rotate { .. } => {
                if let Some(t) = self.preview_transform_of(&drag) {
                    self.transform_selection(t);
                }
            }
            Drag::Marquee { start, current } => {
                let r = Rect::from_points(start, current);
                if r.width() > 0.1 || r.height() > 0.1 {
                    if let Ok(page) = self.doc().page(self.page) {
                        let mut ids = self.selection.clone();
                        for s in page
                            .layers
                            .iter()
                            .filter(|l| l.visible && !l.locked)
                            .flat_map(|l| &l.shapes)
                        {
                            if r.contains_rect(s.bounds()) && !ids.contains(&s.id) {
                                ids.push(s.id);
                            }
                        }
                        self.select(ids);
                    }
                }
            }
            Drag::Freehand { points } if self.tool == Tool::BrushStrokes => {
                let shapes = self.media_shapes(&points);
                if let Some(layer) = self.active_layer() {
                    let ids: Vec<ShapeId> = shapes.iter().map(|s| s.id).collect();
                    let cmds: Vec<Command> = shapes
                        .into_iter()
                        .map(|s| Command::AddShape { layer, shape: s })
                        .collect();
                    if !cmds.is_empty() {
                        let _ = self.engine.run_batch("Brush Strokes", &cmds);
                        self.selection = ids;
                    }
                }
            }
            Drag::Freehand { points } if self.tool == Tool::FreeformPick => {
                self.finish_lasso(points)
            }
            Drag::Freehand { points } => {
                let tol = 0.6 / self.view.zoom as f64 * 2.0;
                let pts = tracedraw_core::geometry::simplify(&points, tol.max(0.15));
                if pts.len() >= 2 {
                    let path = tracedraw_core::geometry::smooth_path(&pts, false);
                    if let Some(id) = self.new_shape(ShapeKind::Path {
                        path,
                        closed: false,
                    }) {
                        self.select(vec![id]);
                    }
                }
            }
            Drag::FillGradient {
                shape,
                start,
                current,
            } if self.tool == Tool::Transparency => {
                if (current - start).hypot() > 0.5 {
                    let angle = (current - start).atan2().to_degrees();
                    self.transparency_default.kind = 1;
                    self.transparency_default.mask =
                        Fill::linear(Color::WHITE, Color::BLACK, angle);
                    self.selection = vec![shape];
                    let s = self.transparency_default.clone();
                    self.apply_transparency(&s);
                }
            }
            Drag::FillGradient {
                shape,
                start,
                current,
            } => {
                if (current - start).hypot() > 0.5 {
                    let from = match &self.default_fill {
                        Fill::Solid(c) => *c,
                        _ => Color::cmyk_pct(0.0, 0.0, 0.0, 100.0),
                    };
                    let angle = (current - start).atan2().to_degrees();
                    self.run(Command::SetFill {
                        shapes: vec![shape],
                        fill: Fill::linear(from, Color::WHITE, angle),
                    });
                }
            }
            Drag::ZoomBox { start, current } => {
                let r = Rect::from_points(start, current);
                if r.width() > 0.5 && r.height() > 0.5 {
                    self.view.fit(r, self.canvas_rect);
                }
            }
            Drag::NodeMarquee { start, current } => self.finish_node_marquee(start, current),
            Drag::NewGuide { .. } => self.finish_guide_drag(),
            Drag::ContourDrag { .. }
            | Drag::Shadow { .. }
            | Drag::MoveGuide { .. }
            | Drag::Node { .. }
            | Drag::Handle { .. }
            | Drag::None => {}
        }
    }

    /// Transform the selection would get if the drag ended now (for preview
    /// and for commit). Translation is handled separately by `Drag::Move`.
    pub fn preview_transform_of(&self, drag: &Drag) -> Option<Affine> {
        match drag {
            Drag::Scale {
                handle,
                anchor,
                start_bounds,
                current,
            } => {
                let b = *start_bounds;
                let (w, h) = (b.width().max(1e-6), b.height().max(1e-6));
                let (mut sx, mut sy) = (1.0, 1.0);
                match handle {
                    Handle::E | Handle::W => {
                        sx = (current.x - anchor.x) / (if *handle == Handle::E { w } else { -w })
                    }
                    Handle::N | Handle::S => {
                        sy = (current.y - anchor.y) / (if *handle == Handle::N { h } else { -h })
                    }
                    Handle::NE => {
                        sx = (current.x - anchor.x) / w;
                        sy = (current.y - anchor.y) / h;
                    }
                    Handle::NW => {
                        sx = (anchor.x - current.x) / w;
                        sy = (current.y - anchor.y) / h;
                    }
                    Handle::SE => {
                        sx = (current.x - anchor.x) / w;
                        sy = (anchor.y - current.y) / h;
                    }
                    Handle::SW => {
                        sx = (anchor.x - current.x) / w;
                        sy = (anchor.y - current.y) / h;
                    }
                }
                if handle.is_corner() && !self.rotate_mode {
                    // Corners scale proportionally.
                    let s = if sx.abs() > sy.abs() {
                        sx.abs()
                    } else {
                        sy.abs()
                    };
                    sx = s * sx.signum();
                    sy = s * sy.signum();
                }
                if sx.abs() < 1e-3 || sy.abs() < 1e-3 {
                    return None;
                }
                Some(
                    Affine::translate(anchor.to_vec2())
                        * Affine::scale_non_uniform(sx, sy)
                        * Affine::translate(-anchor.to_vec2()),
                )
            }
            Drag::Rotate {
                center,
                start_angle,
                current_angle,
            } => {
                let a = current_angle - start_angle;
                Some(
                    Affine::translate(center.to_vec2())
                        * Affine::rotate(a)
                        * Affine::translate(-center.to_vec2()),
                )
            }
            _ => None,
        }
    }

    pub fn keyboard(&mut self, ctx: &egui::Context) {
        // Table cell typing has priority while a cell is active.
        if self.table_edit.is_some() && self.tool == Tool::Table {
            let mut text = self
                .table_edit
                .as_ref()
                .map(|e| e.text.clone())
                .unwrap_or_default();
            let mut changed = false;
            let mut finish = false;
            let mut tab: Option<bool> = None;
            ctx.input(|i| {
                for ev in &i.events {
                    match ev {
                        egui::Event::Text(t) => {
                            text.push_str(t);
                            changed = true;
                        }
                        egui::Event::Key {
                            key: Key::Backspace,
                            pressed: true,
                            ..
                        } => {
                            text.pop();
                            changed = true;
                        }
                        egui::Event::Key {
                            key: Key::Tab,
                            pressed: true,
                            modifiers,
                            ..
                        } => tab = Some(modifiers.shift),
                        egui::Event::Key {
                            key: Key::Enter,
                            pressed: true,
                            modifiers,
                            ..
                        } if !modifiers.shift => tab = Some(false),
                        egui::Event::Key {
                            key: Key::Escape,
                            pressed: true,
                            ..
                        } => finish = true,
                        _ => {}
                    }
                }
            });
            if changed {
                if let Some(e) = self.table_edit.as_mut() {
                    e.text = text;
                }
                self.table_commit_text();
            }
            if let Some(back) = tab {
                self.table_next_cell(back);
            }
            if finish {
                self.table_commit_text();
                self.table_edit = None;
            }
            if changed || tab.is_some() || finish {
                return;
            }
        }
        // Text typing has priority.
        if let Some(mut te) = self.text_edit.clone() {
            let mut changed = false;
            let mut finish = false;
            ctx.input(|i| {
                for ev in &i.events {
                    match ev {
                        egui::Event::Text(t) => {
                            te.text.push_str(t);
                            changed = true;
                        }
                        egui::Event::Key {
                            key: Key::Backspace,
                            pressed: true,
                            ..
                        } => {
                            te.text.pop();
                            changed = true;
                        }
                        egui::Event::Key {
                            key: Key::Enter,
                            pressed: true,
                            modifiers,
                            ..
                        } if !modifiers.shift => {
                            te.text.push('\n');
                            changed = true;
                        }
                        egui::Event::Key {
                            key: Key::Escape,
                            pressed: true,
                            ..
                        } => finish = true,
                        _ => {}
                    }
                }
            });
            if changed {
                self.text_edit = Some(te);
                self.update_text();
            }
            if finish {
                self.finish_text();
                self.set_tool(Tool::Pick);
            }
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let input = ctx.input(|i| i.clone());
        let pressed =
            |k: Key, m: Modifiers| input.key_pressed(k) && input.modifiers.matches_logically(m);
        let cmd = Modifiers::COMMAND;

        if pressed(Key::Z, cmd) {
            self.undo();
        }
        if pressed(Key::Z, cmd | Modifiers::SHIFT) {
            self.redo();
        }
        if pressed(Key::S, cmd) {
            self.save(false);
        }
        if pressed(Key::S, cmd | Modifiers::SHIFT) {
            self.save(true);
        }
        if pressed(Key::S, Modifiers::ALT) && !self.selection.is_empty() {
            self.create_symmetry();
        }
        // Docker shortcuts (Ctrl+F9 Contour, Alt+F3 Lens, ...): toggle.
        for tab in crate::app::DockerTab::ALL {
            let label = tab.shortcut();
            if label.is_empty() || !label.contains("F") || label == "Ctrl+F" {
                continue;
            }
            let Some((k, shift)) = Tool::parse_shortcut(label) else {
                continue;
            };
            let mut m = Modifiers::NONE;
            if label.contains("Ctrl+") {
                m |= cmd;
            }
            if label.contains("Alt+") {
                m |= Modifiers::ALT;
            }
            if shift {
                m |= Modifiers::SHIFT;
            }
            if pressed(k, m) {
                self.toggle_docker(tab);
            }
        }
        if pressed(Key::O, cmd) {
            self.open_dialog();
        }
        if pressed(Key::N, cmd) {
            self.new_document();
        }
        if pressed(Key::E, cmd) {
            self.export();
        }
        if pressed(Key::I, cmd) {
            self.import();
        }
        if pressed(Key::G, cmd) {
            self.group_selection();
        }
        if pressed(Key::U, cmd) {
            self.ungroup_selection();
        }
        if pressed(Key::Q, cmd | Modifiers::SHIFT) {
            self.convert_outline_to_object();
        } else if pressed(Key::Q, cmd) {
            self.convert_to_curves();
        }
        if pressed(Key::A, cmd) {
            if self.tool == Tool::Shape && !self.selection.is_empty() {
                self.select_all_nodes();
            } else {
                self.select_all();
            }
        }
        if pressed(Key::L, cmd) {
            self.combine();
        }
        if pressed(Key::J, cmd) {
            self.dialog = crate::ui::dialogs::Dialog::Options;
        }
        if pressed(Key::Tab, Modifiers::NONE) || pressed(Key::Tab, Modifiers::SHIFT) {
            self.cycle_selection(!input.modifiers.shift);
        }
        if pressed(Key::K, cmd) {
            self.break_apart();
        }
        if pressed(Key::C, cmd) {
            self.copy_with_system();
        }
        if pressed(Key::X, cmd) {
            self.cut();
        }
        if pressed(Key::V, cmd) {
            self.paste_any();
        }
        if pressed(Key::D, cmd) {
            self.duplicate();
        }
        if pressed(Key::Home, cmd | Modifiers::SHIFT) {
            self.order(0);
        }
        if pressed(Key::End, cmd | Modifiers::SHIFT) {
            self.order(3);
        }
        if pressed(Key::PageUp, cmd) {
            self.order(1);
        }
        if pressed(Key::PageDown, cmd) {
            self.order(2);
        }
        if pressed(Key::Delete, Modifiers::NONE) || pressed(Key::Backspace, Modifiers::NONE) {
            if self.tool == Tool::Shape && !self.node_selection.is_empty() {
                self.delete_selected_nodes();
            } else if self.tool == Tool::AnchorEditing && self.delete_selected_anchor() {
                // The anchor went, the object stays.
            } else if let Some(gi) = self.selected_guide {
                self.delete_guide(gi);
            } else {
                self.delete_selection();
            }
        }
        if pressed(Key::F4, Modifiers::SHIFT) {
            self.zoom_to_page();
        }
        if pressed(Key::F4, Modifiers::NONE) {
            self.zoom_to_fit();
        }
        if pressed(Key::F2, Modifiers::SHIFT) {
            self.zoom_to_selection();
        }
        if pressed(Key::F2, Modifiers::NONE) {
            self.set_tool(Tool::Zoom);
        }
        if pressed(Key::Plus, Modifiers::NONE) || pressed(Key::Equals, Modifiers::NONE) {
            let c = self.canvas_rect.center();
            self.view.zoom_at(c, 1.25);
        }
        if pressed(Key::Minus, Modifiers::NONE) {
            let c = self.canvas_rect.center();
            self.view.zoom_at(c, 0.8);
        }
        if pressed(Key::Enter, Modifiers::NONE) {
            self.finish_curve();
        }
        if pressed(Key::Escape, Modifiers::NONE) {
            self.curve = None;
            self.dimension_points.clear();
            self.eyedropper_color = None;
            self.select(Vec::new());
            if self.tool != Tool::Pick {
                self.set_tool(Tool::Pick);
            }
        }
        if pressed(Key::PageDown, Modifiers::NONE) {
            let i = self.page_index();
            self.goto_page(i + 1);
        }
        if pressed(Key::PageUp, Modifiers::NONE) {
            let i = self.page_index();
            if i > 0 {
                self.goto_page(i - 1);
            }
        }
        // Align shortcuts (the target design: plain letters with a selection).
        if input.modifiers.is_none() && !self.selection.is_empty() {
            use crate::ops::Align;
            for (k, a) in [
                (Key::L, Align::Left),
                (Key::R, Align::Right),
                (Key::T, Align::Top),
                (Key::B, Align::Bottom),
                (Key::E, Align::CenterH),
                (Key::C, Align::CenterV),
                (Key::P, Align::CenterPage),
            ] {
                if input.key_pressed(k) {
                    self.align(a);
                    return;
                }
            }
        }
        // Tool shortcuts: user overrides from Options, then the defaults.
        if input.modifiers.is_none() || input.modifiers.shift_only() {
            for group in crate::tools::GROUPS {
                for t in group.tools {
                    let label = self
                        .settings
                        .shortcuts
                        .iter()
                        .find(|(id, _)| id == t.id())
                        .map(|(_, v)| v.clone())
                        .or_else(|| t.shortcut().map(|s| s.to_string()));
                    let Some(label) = label else { continue };
                    if let Some((k, shift)) = Tool::parse_shortcut(&label) {
                        if shift == input.modifiers.shift && input.key_pressed(k) {
                            self.set_tool(*t);
                        }
                    }
                }
            }
            // Arrow nudge.
            let step = if input.modifiers.shift {
                self.nudge_mm * 10.0
            } else {
                self.nudge_mm
            };
            let mut d = Vec2::ZERO;
            if input.key_pressed(Key::ArrowLeft) {
                d.x -= step;
            }
            if input.key_pressed(Key::ArrowRight) {
                d.x += step;
            }
            if input.key_pressed(Key::ArrowUp) {
                d.y += step;
            }
            if input.key_pressed(Key::ArrowDown) {
                d.y -= step;
            }
            if d.hypot() > 0.0 {
                self.nudge(d);
            }
        }
    }
}
