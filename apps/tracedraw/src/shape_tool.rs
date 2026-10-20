//! The Shape tool: select nodes, drag nodes and handles, add and delete
//! nodes, change node and segment types. Edits go through
//! `Command::SetShapeKind`, collapsed into one undo step per drag.

use crate::app::{App, Drag};
use egui::{Modifiers, PointerButton, Response};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{BezPath, PathEl, Point, Rect, Vec2},
    nodes::{self, NodeType, Which},
    Command, ShapeId,
};

/// Keep a move horizontal (`Some(true)`) or vertical (`Some(false)`);
/// `None` leaves it free.
fn constrain_axis(d: &mut Vec2, horizontal: Option<bool>) {
    match horizontal {
        Some(true) => d.y = 0.0,
        Some(false) => d.x = 0.0,
        None => {}
    }
}

pub struct NodeHit {
    pub shape: ShapeId,
    pub index: usize,
}

/// The four corners of a picture on the page, in node order (from its
/// rectangle's first corner), or `None` for other objects.
pub(crate) fn bitmap_corners(s: &tracedraw_core::Shape) -> Option<[Point; 4]> {
    let ShapeKind::Bitmap { rect, .. } = &s.kind else {
        return None;
    };
    let t = s.transform;
    Some([
        t * Point::new(rect.x0, rect.y0),
        t * Point::new(rect.x1, rect.y0),
        t * Point::new(rect.x1, rect.y1),
        t * Point::new(rect.x0, rect.y1),
    ])
}

/// The objects a double click with the Shape tool turns into curves:
/// rectangles, ellipses and polygons. A picture, text, group or clip frame
/// would lose what it is (#60).
pub(crate) fn curves_on_double_click(kind: &ShapeKind) -> bool {
    matches!(
        kind,
        ShapeKind::Rect { .. } | ShapeKind::Ellipse { .. } | ShapeKind::Polygon { .. }
    )
}

impl App {
    /// A node drag (`Drag::NodeMove`) reaching `p`: the grabbed node snaps
    /// and the chosen nodes follow; Ctrl keeps the move horizontal or
    /// vertical. One undo step for the whole drag.
    pub(crate) fn node_move_to(&mut self, p: Point, mods: Modifiers) {
        let Drag::NodeMove {
            start,
            from,
            grab,
            begun,
        } = self.drag.clone()
        else {
            return;
        };
        let mut d = p - from;
        let horizontal = (mods.ctrl || mods.command).then_some(d.x.abs() >= d.y.abs());
        let exclude: Vec<ShapeId> = start.iter().map(|c| c.shape).collect();
        d = self.snapped_node_move(grab, d, horizontal, &exclude);
        self.move_start_nodes(&start, d, grab, begun);
        self.drag = Drag::NodeMove {
            start,
            from,
            grab,
            begun: true,
        };
    }

    /// Node tracking (Options > Display): a press with the Pick tool on a
    /// node of a selected curve chooses that node and starts dragging it.
    pub(crate) fn start_tracked_node_drag(&mut self, p: Point) -> bool {
        if !self.settings.node_tracking {
            return false;
        }
        let Some(hit) = self.node_at(p) else {
            return false;
        };
        self.node_selection = vec![(hit.shape, hit.index)];
        let grab = self
            .selected_node_points()
            .into_iter()
            .find(|(id, i, _)| (*id, *i) == (hit.shape, hit.index))
            .map(|(_, _, q)| q)
            .unwrap_or(p);
        self.drag = Drag::NodeMove {
            start: self.start_curves(),
            from: p,
            grab,
            begun: false,
        };
        true
    }

    /// The move of a grabbed node by `d` (page space): the node snaps to
    /// the grid, the page, guidelines and other objects, and the move
    /// follows it. `horizontal` keeps the move on one axis, before and
    /// after snapping.
    pub(crate) fn snapped_node_move(
        &self,
        grab: Point,
        mut d: Vec2,
        horizontal: Option<bool>,
        exclude: &[ShapeId],
    ) -> Vec2 {
        constrain_axis(&mut d, horizontal);
        let target = self.snap_point_excluding(grab + d, exclude);
        let mut snapped = target - grab;
        constrain_axis(&mut snapped, horizontal);
        snapped
    }

    /// A curve's path, whether it is closed and its transform to the page
    /// (curves inside clip frames too).
    fn path_of(&self, id: ShapeId) -> Option<(BezPath, bool, tracedraw_core::geometry::Affine)> {
        let s = self.doc().find_shape(id)?;
        let parent = self.doc().parent_transform(id)?;
        match &s.kind {
            ShapeKind::Path { path, closed } => Some((path.clone(), *closed, parent * s.transform)),
            _ => None,
        }
    }

    /// The curve whose nodes the Shape tool shows for the selected object
    /// `id`: the object itself, or for a clip frame (a cropped picture)
    /// its frame.
    pub(crate) fn node_target(&self, id: ShapeId) -> ShapeId {
        match self.doc().find_shape(id).map(|s| &s.kind) {
            Some(ShapeKind::ClipFrame { frame, .. })
                if matches!(frame.kind, ShapeKind::Path { .. }) =>
            {
                frame.id
            }
            _ => id,
        }
    }

    /// A selected picture's corner under the pointer: the picture and the
    /// corner (0 to 3, in [`bitmap_corners`] order).
    fn bitmap_corner_at(&self, p: Point) -> Option<(ShapeId, usize)> {
        let tol = self.settings.nodes.pick_px() / self.view.zoom as f64;
        self.selected_shapes().iter().find_map(|s| {
            bitmap_corners(s)?
                .iter()
                .position(|c| (*c - p).hypot() <= tol)
                .map(|i| (s.id, i))
        })
    }

    /// Put the picture `id` in a clip frame whose frame is the picture's
    /// rectangle (an undo step of its own); the frame's id, whose nodes 0
    /// to 3 are the picture's corners.
    pub(crate) fn crop_bitmap_to_frame(&mut self, id: ShapeId) -> Option<ShapeId> {
        let s = self.doc().find_shape(id)?.clone();
        let ShapeKind::Bitmap { rect, .. } = &s.kind else {
            return None;
        };
        let mut path = BezPath::new();
        path.move_to((rect.x0, rect.y0));
        path.line_to((rect.x1, rect.y0));
        path.line_to((rect.x1, rect.y1));
        path.line_to((rect.x0, rect.y1));
        path.close_path();
        let mut frame = tracedraw_core::Shape::new(
            self.engine.new_shape_id(),
            ShapeKind::Path { path, closed: true },
        );
        frame.fill = tracedraw_core::Fill::None;
        frame.stroke = None;
        let mut inner = s.clone();
        inner.id = self.engine.new_shape_id();
        inner.transform = tracedraw_core::geometry::Affine::IDENTITY;
        let frame_id = frame.id;
        let _ = self.engine.run_with_label(
            &Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::ClipFrame {
                    frame: Box::new(frame),
                    contents: vec![inner],
                },
            },
            "Crop Bitmap",
        );
        Some(frame_id)
    }

    /// The node targets of the selection (see [`Self::node_target`]).
    fn node_targets(&self) -> Vec<ShapeId> {
        self.selection
            .iter()
            .map(|id| self.node_target(*id))
            .collect()
    }

    fn set_path(&mut self, id: ShapeId, path: BezPath, closed: bool, label: &'static str) {
        if self.engine.undo_label() == Some(label) {
            let _ = self.engine.undo();
        }
        let _ = self.engine.run_with_label(
            &Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Path { path, closed },
            },
            label,
        );
    }

    fn set_path_new_step(&mut self, id: ShapeId, path: BezPath, closed: bool) {
        let _ = self.engine.run(&Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Path { path, closed },
        });
    }

    /// Node under the pointer among selected curves.
    pub fn node_at(&self, p: Point) -> Option<NodeHit> {
        // The nearest node within the pick distance, measured on the page:
        // a curve with a scale of its own (text converted to curves, for
        // one) takes the same reach as any other.
        let tol = self.settings.nodes.pick_px() / self.view.zoom as f64;
        let mut best: Option<(NodeHit, f64)> = None;
        for id in &self.node_targets() {
            let Some((path, _, t)) = self.path_of(*id) else {
                continue;
            };
            for n in nodes::nodes(&(t * path)) {
                let d = (n.pos - p).hypot();
                if d <= tol && best.as_ref().is_none_or(|b| d < b.1) {
                    let hit = NodeHit {
                        shape: *id,
                        index: n.index,
                    };
                    best = Some((hit, d));
                }
            }
        }
        best.map(|b| b.0)
    }

    /// `p` lies within three pick distances of a node, a control handle
    /// of a chosen node or a rectangle, ellipse or polygon node of the
    /// selection: a press there that took nothing is a near miss.
    pub(crate) fn near_selected_nodes(&self, p: Point) -> bool {
        let reach = 3.0 * self.settings.nodes.pick_px() / self.view.zoom as f64;
        for id in &self.node_targets() {
            let Some((path, _, t)) = self.path_of(*id) else {
                continue;
            };
            for n in nodes::nodes(&(t * path)) {
                let chosen = self.node_selection.contains(&(*id, n.index));
                let mut spots = vec![n.pos];
                if chosen {
                    spots.extend(n.ctrl_in);
                    spots.extend(n.ctrl_out);
                }
                if spots.iter().any(|q| (*q - p).hypot() <= reach) {
                    return true;
                }
            }
        }
        self.selected_shapes()
            .iter()
            .any(|s| crate::kind_nodes::node_at(s, p, reach).is_some())
    }

    /// A rectangle, ellipse or polygon node of the selection under the
    /// pointer, with its object.
    pub fn kind_node_at(
        &self,
        p: Point,
    ) -> Option<(tracedraw_core::Shape, crate::kind_nodes::KindNode)> {
        let tol = self.settings.nodes.pick_px() / self.view.zoom as f64;
        self.selected_shapes()
            .into_iter()
            .find_map(|s| crate::kind_nodes::node_at(&s, p, tol).map(|n| (s, n)))
    }

    /// Control handle under the pointer, for selected nodes only.
    pub(crate) fn ctrl_handle_at(&self, p: Point) -> Option<(ShapeId, usize, Which)> {
        // The nearest handle within the pick distance, on the page.
        let tol = self.settings.nodes.pick_px() / self.view.zoom as f64;
        let mut best: Option<((ShapeId, usize, Which), f64)> = None;
        for (id, index) in &self.node_selection {
            let Some((path, _, t)) = self.path_of(*id) else {
                continue;
            };
            let Some(n) = nodes::nodes(&(t * path))
                .into_iter()
                .find(|n| n.index == *index)
            else {
                continue;
            };
            for (c, which) in [(n.ctrl_in, Which::In), (n.ctrl_out, Which::Out)] {
                let Some(c) = c else { continue };
                let d = (c - p).hypot();
                if d <= tol && best.as_ref().is_none_or(|b| d < b.1) {
                    best = Some(((*id, *index, which), d));
                }
            }
        }
        best.map(|b| b.0)
    }

    pub fn shape_tool_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        if response.double_clicked_by(PointerButton::Primary) {
            if let Some(hit) = self.node_at(p) {
                // Double-click a node deletes it.
                if let Some((path, closed, _)) = self.path_of(hit.shape) {
                    let np = nodes::delete_node(&path, hit.index);
                    self.set_path_new_step(hit.shape, np, closed);
                    self.node_selection.clear();
                }
                return;
            }
            // Double-click a segment of a selected curve adds a node; on a
            // non-curve object, convert it to curves.
            if let Some((id, index, t)) = self.segment_at(p) {
                if let Some((path, closed, _)) = self.path_of(id) {
                    let np = nodes::add_node(&path, index, t);
                    self.set_path_new_step(id, np, closed);
                    self.node_selection = vec![(id, index)];
                }
                return;
            }
            // Only shapes become curves: a picture, text, group or clip
            // frame would lose what it is (#60).
            if let Some(id) = self.hit_test(p) {
                let shape = self
                    .doc()
                    .find_shape(id)
                    .map(|s| curves_on_double_click(&s.kind));
                self.select(vec![id]);
                if shape == Some(true) {
                    self.convert_to_curves();
                }
            }
            return;
        }

        if response.drag_started_by(PointerButton::Primary) {
            // Text: the spacing arrows, then the character nodes.
            if let Some((id, horizontal)) = self.text_arrow_at(p) {
                if let Some(s) = self.doc().find_shape(id).cloned() {
                    self.drag = Drag::TextSpacing {
                        start: Box::new(s),
                        horizontal,
                        word: mods.shift,
                        from: p,
                        begun: false,
                    };
                    return;
                }
            }
            if let Some(key) = self.text_node_at(p) {
                if !self.node_selection.contains(&key) {
                    if mods.ctrl || mods.command || mods.shift {
                        self.click_node(key, mods);
                    } else {
                        self.node_selection = vec![key];
                    }
                }
                if let Some(s) = self.doc().find_shape(key.0).cloned() {
                    self.drag = Drag::TextChars {
                        chars: self.chosen_chars(key.0),
                        start: Box::new(s),
                        from: p,
                        begun: false,
                    };
                    return;
                }
            }
            // A picture's corner: the picture becomes a clip frame whose
            // frame is its rectangle, and the corner moves as a node of it.
            if let Some((id, corner)) = self.bitmap_corner_at(p) {
                if let Some(frame) = self.crop_bitmap_to_frame(id) {
                    self.node_selection = vec![(frame, corner)];
                    let grab = self
                        .selected_node_points()
                        .first()
                        .map(|(_, _, q)| *q)
                        .unwrap_or(p);
                    self.drag = Drag::NodeMove {
                        start: self.start_curves(),
                        from: p,
                        grab,
                        begun: false,
                    };
                    return;
                }
            }
            if let Some((s, node)) = self.kind_node_at(p) {
                let single = crate::kind_nodes::single_corner(
                    self.rect_corner_selected,
                    &s,
                    node,
                    mods.ctrl,
                );
                self.drag = Drag::KindNode {
                    shape: s.id,
                    node,
                    start: Box::new(s),
                    single,
                    begun: false,
                    turn: 0.0,
                };
                return;
            }
            // The node transform handles, while a transform mode is on.
            if let Some(handle) = self.node_transform_handle_at(p) {
                if let Some(bounds) = self.selected_nodes_bounds() {
                    self.drag = Drag::NodeTransform {
                        handle,
                        start: self.start_curves(),
                        bounds,
                        from: p,
                        begun: false,
                    };
                    return;
                }
            }
            if let Some((shape, index, which)) = self.ctrl_handle_at(p) {
                self.drag = Drag::Handle {
                    shape,
                    index,
                    which,
                };
                return;
            }
            if let Some(hit) = self.node_at(p) {
                let key = (hit.shape, hit.index);
                if !self.node_selection.contains(&key) {
                    if mods.ctrl || mods.command || mods.shift {
                        self.click_node(key, mods);
                    } else {
                        self.node_selection = vec![key];
                    }
                }
                let grab = self
                    .selected_node_points()
                    .into_iter()
                    .find(|(id, i, _)| (*id, *i) == key)
                    .map(|(_, _, q)| q)
                    .unwrap_or(p);
                self.drag = Drag::NodeMove {
                    start: self.start_curves(),
                    from: p,
                    grab,
                    begun: false,
                };
                return;
            }
            // A segment of a selected curve bends.
            if let Some((shape, index, t)) = self.segment_at(p) {
                if let Some((path, closed, _)) = self.path_of(shape) {
                    self.segment_spot = None;
                    self.drag = Drag::SegmentBend {
                        shape,
                        path,
                        closed,
                        index,
                        t,
                        from: p,
                        begun: false,
                    };
                    return;
                }
            }
            if let Some(id) = self.hit_test(p) {
                if !self.selection.contains(&id) {
                    self.select(vec![id]);
                }
            }
            self.drag = if self.node_lasso {
                Drag::NodeLasso { points: vec![p] }
            } else {
                Drag::NodeMarquee {
                    start: p,
                    current: p,
                }
            };
        }

        if response.dragged_by(PointerButton::Primary) {
            match self.drag.clone() {
                Drag::TextChars {
                    start,
                    chars,
                    from,
                    begun,
                } => {
                    let mut d = p - from;
                    if mods.ctrl || mods.command {
                        if d.x.abs() >= d.y.abs() {
                            d.y = 0.0;
                        } else {
                            d.x = 0.0;
                        }
                    }
                    self.move_text_chars(&start, &chars, d, begun);
                    self.drag = Drag::TextChars {
                        start,
                        chars,
                        from,
                        begun: true,
                    };
                }
                Drag::TextSpacing {
                    start,
                    horizontal,
                    word,
                    from,
                    begun,
                } => {
                    self.drag_text_spacing(&start, horizontal, word, p - from, begun);
                    self.drag = Drag::TextSpacing {
                        start,
                        horizontal,
                        word,
                        from,
                        begun: true,
                    };
                }
                Drag::NodeMove { .. } => self.node_move_to(p, mods),
                Drag::NodeTransform {
                    handle,
                    start,
                    bounds,
                    from,
                    begun,
                } => {
                    let a = crate::node_edit::handle_transform(
                        self.node_transform,
                        handle,
                        bounds,
                        from,
                        p,
                        mods.shift,
                    );
                    self.transform_start_nodes(&start, a, begun);
                    self.drag = Drag::NodeTransform {
                        handle,
                        start,
                        bounds,
                        from,
                        begun: true,
                    };
                }
                Drag::NodeLasso { mut points } => {
                    if points
                        .last()
                        .is_none_or(|q| (*q - p).hypot() > 0.2 / self.view.zoom as f64)
                    {
                        points.push(p);
                    }
                    self.drag = Drag::NodeLasso { points };
                }
                Drag::Handle {
                    shape,
                    index,
                    which,
                } => {
                    if let Some((path, closed, t)) = self.path_of(shape) {
                        // The handle snaps like a node does.
                        let q = self.snap_point_excluding(p, &[shape]);
                        let lp = t.inverse() * q;
                        let ty = if mods.ctrl {
                            NodeType::Cusp
                        } else {
                            self.node_type_of(shape, index)
                        };
                        let np = nodes::move_handle(&path, index, which, lp, ty);
                        self.set_path(shape, np, closed, "Edit Handle");
                    }
                }
                Drag::NodeMarquee { start, .. } => {
                    self.drag = Drag::NodeMarquee { start, current: p }
                }
                Drag::SegmentBend {
                    shape,
                    path,
                    closed,
                    index,
                    t,
                    from,
                    begun,
                } => {
                    // The grabbed point follows the pointer (in the curve's
                    // own space); one undo step for the whole drag.
                    if let Some((_, _, tf)) = self.path_of(shape) {
                        let det = tf.determinant();
                        if det.is_finite() && det.abs() > 1e-12 {
                            let inv = tf.inverse();
                            let d = (inv * p) - (inv * from);
                            let bent = nodes::bend_segment(&path, index, t, d);
                            let cmd = Command::SetShapeKind {
                                shape,
                                kind: ShapeKind::Path { path: bent, closed },
                            };
                            let _ = if begun {
                                self.engine.amend(&cmd)
                            } else {
                                self.engine.run_with_label(&cmd, "Bend Segment")
                            };
                        }
                    }
                    self.drag = Drag::SegmentBend {
                        shape,
                        path,
                        closed,
                        index,
                        t,
                        from,
                        begun: true,
                    };
                }
                Drag::KindNode {
                    shape,
                    node,
                    start,
                    single,
                    begun,
                    mut turn,
                } => {
                    let q = self.snap_point_excluding(p, &[shape]);
                    if let Some(kind) = crate::kind_nodes::drag(&start, node, q, single, &mut turn)
                    {
                        let cmd = Command::SetShapeKind { shape, kind };
                        // One undo step for the whole drag.
                        let _ = if begun {
                            self.engine.amend(&cmd)
                        } else {
                            self.engine
                                .run_with_label(&cmd, crate::kind_nodes::label(&start))
                        };
                    }
                    self.drag = Drag::KindNode {
                        shape,
                        node,
                        start,
                        single,
                        begun: true,
                        turn,
                    };
                }
                _ => {}
            }
        }

        if response.clicked_by(PointerButton::Primary) {
            if let Some(key) = self.text_node_at(p) {
                self.click_node(key, mods);
                return;
            }
            if let Some((s, node)) = self.kind_node_at(p) {
                // A click on a rectangle corner chooses it: dragging it
                // then changes that corner only.
                self.rect_corner_selected = match node {
                    crate::kind_nodes::KindNode::Corner { corner, .. } => Some((s.id, corner)),
                    _ => None,
                };
                return;
            }
            if let Some(hit) = self.node_at(p) {
                self.segment_spot = None;
                self.click_node((hit.shape, hit.index), mods);
                return;
            }
            // A click on a segment chooses the segment: it is drawn
            // highlighted, Delete removes it, and as its end node is the
            // one chosen To Line and To Curve act on it. The spot clicked
            // is marked; Add Node puts the new node there.
            if let Some((shape, index, t)) = self.segment_at(p) {
                self.node_selection = vec![(shape, index)];
                self.segment_spot = Some((shape, index, t));
                return;
            }
            self.segment_spot = None;
            // A press on a control handle, or a near miss of a node or a
            // handle of the selected curves, keeps the selection: only a
            // click clearly on empty canvas deselects.
            if self.ctrl_handle_at(p).is_some() || self.near_selected_nodes(p) {
                return;
            }
            match self.hit_test(p) {
                Some(id) => {
                    if mods.shift {
                        if !self.selection.contains(&id) {
                            self.selection.push(id);
                        }
                    } else if !self.selection.contains(&id) {
                        self.select(vec![id]);
                    } else {
                        self.node_selection.clear();
                    }
                }
                None => self.select(Vec::new()),
            }
        }
    }

    /// Finish a node marquee: select the nodes inside the rectangle.
    pub fn finish_node_marquee(&mut self, start: Point, current: Point) {
        let r = Rect::from_points(start, current);
        if r.width() < 0.05 && r.height() < 0.05 {
            return;
        }
        let mut sel = Vec::new();
        for id in &self.node_targets() {
            let Some((path, _, t)) = self.path_of(*id) else {
                continue;
            };
            for n in nodes::nodes(&path) {
                if r.contains(t * n.pos) {
                    sel.push((*id, n.index));
                }
            }
        }
        self.node_selection = sel;
        self.segment_spot = None;
        // Character nodes of selected texts too.
        self.choose_chars_in(|q| r.contains(q));
    }

    /// Nearest segment of a selected curve within a few pixels.
    pub(crate) fn segment_at(&self, p: Point) -> Option<(ShapeId, usize, f64)> {
        let tol = (self.settings.nodes.pick_px() + 1.0) / self.view.zoom as f64;
        let mut best: Option<(ShapeId, usize, f64, f64)> = None;
        for id in &self.node_targets() {
            let Some((path, _, t)) = self.path_of(*id) else {
                continue;
            };
            // Measured on the page, as the nodes are.
            if let Some((i, tt, d)) = nodes::nearest_segment(&(t * path), p) {
                if d <= tol && best.map(|b| d < b.3).unwrap_or(true) {
                    best = Some((*id, i, tt, d));
                }
            }
        }
        best.map(|(id, i, t, _)| (id, i, t))
    }

    // ----- property bar actions ---------------------------------------------

    fn for_each_selected_node(
        &mut self,
        label: &'static str,
        f: impl Fn(&BezPath, usize) -> BezPath,
    ) {
        let sel = self.node_selection.clone();
        let mut by_shape: std::collections::BTreeMap<ShapeId, Vec<usize>> = Default::default();
        for (id, i) in sel {
            by_shape.entry(id).or_default().push(i);
        }
        let mut cmds = Vec::new();
        for (id, mut idxs) in by_shape {
            let Some((mut path, closed, _)) = self.path_of(id) else {
                continue;
            };
            // Highest index first so earlier indices stay valid when elements are removed.
            idxs.sort_unstable_by(|a, b| b.cmp(a));
            for i in idxs {
                path = f(&path, i);
            }
            cmds.push(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Path { path, closed },
            });
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch(label, &cmds);
        }
    }

    /// Reduce Nodes: simplify the selected curves (all nodes when none are selected).
    pub fn reduce_selected_nodes(&mut self) {
        let tol = 0.3 / self.view.zoom.max(0.05) as f64;
        let ids: Vec<ShapeId> = if self.node_selection.is_empty() {
            self.selection.clone()
        } else {
            let mut v: Vec<ShapeId> = self.node_selection.iter().map(|(id, _)| *id).collect();
            v.dedup();
            v
        };
        let mut cmds = Vec::new();
        for id in ids {
            let Some((path, closed, _)) = self.path_of(id) else {
                continue;
            };
            let reduced = nodes::reduce_nodes(&path, tol);
            if reduced.elements().len() < path.elements().len() {
                cmds.push(Command::SetShapeKind {
                    shape: id,
                    kind: ShapeKind::Path {
                        path: reduced,
                        closed,
                    },
                });
            }
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Reduce Nodes", &cmds);
        }
        self.node_selection.clear();
    }

    /// Align Nodes: move the selected nodes onto a common horizontal and/or vertical.
    pub fn align_selected_nodes(&mut self, horizontal: bool, vertical: bool) {
        let sel = self.node_selection.clone();
        let mut by_shape: std::collections::BTreeMap<ShapeId, Vec<usize>> = Default::default();
        for (id, i) in sel {
            by_shape.entry(id).or_default().push(i);
        }
        let mut cmds = Vec::new();
        for (id, idxs) in by_shape {
            let Some((path, closed, _)) = self.path_of(id) else {
                continue;
            };
            cmds.push(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Path {
                    path: nodes::align_nodes(&path, &idxs, horizontal, vertical),
                    closed,
                },
            });
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Align Nodes", &cmds);
        }
    }

    /// The segment chosen by a click on it, as (curve, element drawing
    /// it): the spot clicked is kept and its end node is the only node
    /// chosen.
    pub(crate) fn chosen_segment(&self) -> Option<(ShapeId, usize)> {
        let (shape, index, _) = self.segment_spot?;
        (self.node_selection == [(shape, index)]).then_some((shape, index))
    }

    /// Delete the chosen segment: an open curve splits there (or loses an
    /// end), a closed one opens there; a curve left with no segment goes.
    /// One undo step. False when no segment is chosen.
    pub fn delete_chosen_segment(&mut self) -> bool {
        let Some((id, index)) = self.chosen_segment() else {
            return false;
        };
        let Some((path, closed, _)) = self.path_of(id) else {
            return false;
        };
        let left = nodes::delete_segment(&path, index);
        if left == path {
            return false;
        }
        let drawn = left
            .elements()
            .iter()
            .any(|e| !matches!(e, PathEl::MoveTo(_) | PathEl::ClosePath));
        let cmd = if drawn {
            let closed = closed
                && left
                    .elements()
                    .iter()
                    .any(|e| matches!(e, PathEl::ClosePath));
            Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Path { path: left, closed },
            }
        } else {
            Command::DeleteShapes { shapes: vec![id] }
        };
        if let Err(e) = self.engine.run_with_label(&cmd, "Delete Segment") {
            self.status = e.to_string();
        }
        self.node_selection.clear();
        self.segment_spot = None;
        true
    }

    pub fn delete_selected_nodes(&mut self) {
        self.for_each_selected_node("Delete Nodes", nodes::delete_node);
        self.node_selection.clear();
    }

    pub fn add_node_midpoints(&mut self) {
        // A spot clicked on the chosen segment: the node goes there.
        if let Some((shape, index, t)) = self.segment_spot.take() {
            if self.node_selection == [(shape, index)] {
                if let Some((path, closed, _)) = self.path_of(shape) {
                    let np = nodes::add_node(&path, index, t);
                    self.set_path_new_step(shape, np, closed);
                    self.node_selection = vec![(shape, index)];
                }
                return;
            }
        }
        // Add a node in the middle of the segment ending at each selected node.
        self.for_each_selected_node("Add Nodes", |p, i| nodes::add_node(p, i, 0.5));
        self.node_selection.clear();
    }

    /// Make the chosen nodes `ty`: their handles are aligned (and of equal
    /// length) as the type needs, and the type is kept with the curve, so
    /// a cusp node with aligned handles stays cusp. One undo step.
    pub fn set_selected_node_type(&mut self, ty: NodeType) {
        let mut by_shape: std::collections::BTreeMap<ShapeId, Vec<usize>> = Default::default();
        for (id, i) in self.node_selection.clone() {
            by_shape.entry(id).or_default().push(i);
        }
        let mut cmds = Vec::new();
        for (id, idxs) in by_shape {
            let Some((mut path, closed, _)) = self.path_of(id) else {
                continue;
            };
            let mut types = self
                .doc()
                .shape(id)
                .map(|(_, s)| s.node_types.clone())
                .unwrap_or_default();
            for i in idxs {
                path = nodes::set_node_type(&path, i, ty);
                types.insert(i, ty);
            }
            cmds.push(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Path { path, closed },
            });
            cmds.push(Command::SetNodeTypes { shape: id, types });
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Node Type", &cmds);
        }
    }

    /// The type of node `index` of curve `id`: as chosen for it, else as
    /// its handles read.
    pub fn node_type_of(&self, id: ShapeId, index: usize) -> NodeType {
        match self.doc().shape(id) {
            Ok((_, s)) => match &s.kind {
                ShapeKind::Path { path, .. } => nodes::node_type_with(path, index, &s.node_types),
                _ => NodeType::Cusp,
            },
            Err(_) => NodeType::Cusp,
        }
    }

    pub fn selected_segments_to_line(&mut self) {
        self.for_each_selected_node("To Line", nodes::to_line);
    }

    pub fn selected_segments_to_curve(&mut self) {
        self.for_each_selected_node("To Curve", nodes::to_curve);
    }

    pub fn break_selected_nodes(&mut self) {
        self.for_each_selected_node("Break Curve", nodes::break_at);
        self.node_selection.clear();
    }

    pub fn close_selected_curves(&mut self) {
        let ids: Vec<ShapeId> = if self.node_selection.is_empty() {
            self.selection.clone()
        } else {
            self.node_selection.iter().map(|(id, _)| *id).collect()
        };
        let mut cmds = Vec::new();
        for id in ids {
            if let Some((path, _, _)) = self.path_of(id) {
                let np = nodes::close_subpath(&path, 0);
                cmds.push(Command::SetShapeKind {
                    shape: id,
                    kind: ShapeKind::Path {
                        path: np,
                        closed: true,
                    },
                });
            }
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Close Curve", &cmds);
        }
    }

    pub fn reverse_selected_curves(&mut self) {
        let mut cmds = Vec::new();
        for id in self.selection.clone() {
            if let Some((path, closed, _)) = self.path_of(id) {
                cmds.push(Command::SetShapeKind {
                    shape: id,
                    kind: ShapeKind::Path {
                        path: nodes::reverse(&path),
                        closed,
                    },
                });
            }
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Reverse Direction", &cmds);
        }
        self.node_selection.clear();
    }

    pub fn select_all_nodes(&mut self) {
        self.segment_spot = None;
        let mut sel = Vec::new();
        for id in &self.node_targets() {
            if let Some((path, _, _)) = self.path_of(*id) {
                for n in nodes::nodes(&path) {
                    sel.push((*id, n.index));
                }
            }
        }
        self.node_selection = sel;
    }

    /// Node type of the first selected node, for the property bar.
    pub fn current_node_type(&self) -> Option<NodeType> {
        let (id, i) = *self.node_selection.first()?;
        self.path_of(id)?;
        Some(self.node_type_of(id, i))
    }
}

impl App {
    /// Join two selected end nodes (of the same or different curves).
    pub fn join_selected_nodes(&mut self) {
        if self.node_selection.len() != 2 {
            return;
        }
        let (a, b) = (self.node_selection[0], self.node_selection[1]);
        if self.join_nodes(a, b) {
            self.node_selection.clear();
        }
    }

    /// Join end node `a` with end node `b` into one node halfway between
    /// them, with no segment added: the ends of one open subpath close it,
    /// ends of two subpaths (of one curve, or of two curves, which become
    /// one curve keeping `a`'s place and look) make one subpath. One undo
    /// step (Join Nodes). False, and nothing changes, when either is not an
    /// end of an open subpath.
    pub(crate) fn join_nodes(&mut self, a: (ShapeId, usize), b: (ShapeId, usize)) -> bool {
        let (sa, ia) = a;
        let (sb, ib) = b;
        let Some((pa, _, ta)) = self.path_of(sa) else {
            return false;
        };
        let (combined, jb) = if sa == sb {
            (pa, ib)
        } else {
            let Some((pb, _, tb)) = self.path_of(sb) else {
                return false;
            };
            // The second curve drawn into the first one's space, after it.
            let mut both = nodes::normalize(&pa);
            let offset = both.elements().len();
            for el in (ta.inverse() * tb * nodes::normalize(&pb)).elements() {
                both.push(*el);
            }
            (both, ib + offset)
        };
        let Some(joined) = nodes::join_ends(&combined, ia, jb) else {
            return false;
        };
        let closed = every_subpath_closed(&joined);
        let mut cmds = vec![Command::SetShapeKind {
            shape: sa,
            kind: ShapeKind::Path {
                path: joined,
                closed,
            },
        }];
        if sa != sb {
            cmds.push(Command::DeleteShapes { shapes: vec![sb] });
            self.selection.retain(|id| *id != sb);
        }
        if let Err(e) = self.engine.run_batch("Join Nodes", &cmds) {
            self.status = e.to_string();
            return false;
        }
        true
    }

    /// After dragging one end node: dropped on another end node of a
    /// selected curve (within the pick distance), the two are joined, the
    /// curve dropped onto keeping its place and look.
    pub(crate) fn join_dropped_end_node(&mut self) {
        let [(id, index)] = self.node_selection[..] else {
            return;
        };
        let Some((path, _, t)) = self.path_of(id) else {
            return;
        };
        let Some(at) = nodes::nodes(&path)
            .into_iter()
            .find(|n| n.index == index)
            .map(|n| t * n.pos)
        else {
            return;
        };
        let tol = (self.settings.nodes.pick_px() + 1.0) / self.view.zoom as f64;
        let mut near: Vec<((ShapeId, usize), f64)> = Vec::new();
        for other in self.node_targets() {
            let Some((op, _, ot)) = self.path_of(other) else {
                continue;
            };
            for n in nodes::nodes(&op) {
                if (other, n.index) == (id, index) {
                    continue;
                }
                let d = (ot * n.pos - at).hypot();
                if d <= tol {
                    near.push(((other, n.index), d));
                }
            }
        }
        near.sort_by(|a, b| a.1.total_cmp(&b.1));
        // The nearest end node of an open subpath; the curve dropped onto
        // keeps its place and look.
        for (target, _) in near {
            if self.join_nodes(target, (id, index)) {
                self.node_selection.clear();
                return;
            }
        }
    }

    /// Extract the subpath holding the first selected node into its own object.
    pub fn extract_subpath(&mut self) {
        let Some(&(id, index)) = self.node_selection.first() else {
            return;
        };
        let Some((path, closed, _)) = self.path_of(id) else {
            return;
        };
        // Find the subpath boundaries around `index`.
        let els = path.elements();
        let mut start = 0;
        for (i, el) in els.iter().enumerate() {
            if i > index {
                break;
            }
            if matches!(el, tracedraw_core::geometry::PathEl::MoveTo(_)) {
                start = i;
            }
        }
        let mut end = els.len();
        for (i, el) in els.iter().enumerate().skip(start + 1) {
            if matches!(el, tracedraw_core::geometry::PathEl::MoveTo(_)) {
                end = i;
                break;
            }
        }
        if start == 0 && end == els.len() {
            return; // single subpath, nothing to extract
        }
        let mut extracted = tracedraw_core::BezPath::new();
        for el in &els[start..end] {
            extracted.push(*el);
        }
        let mut rest = tracedraw_core::BezPath::new();
        for el in els[..start].iter().chain(els[end..].iter()) {
            rest.push(*el);
        }
        let Ok((layer, src)) = self.doc().shape(id) else {
            return;
        };
        let layer = layer.id;
        let mut new_shape = src.clone();
        new_shape.id = self.engine.new_shape_id();
        new_shape.kind = ShapeKind::Path {
            path: extracted,
            closed,
        };
        let new_id = new_shape.id;
        let cmds = vec![
            Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Path { path: rest, closed },
            },
            Command::AddShape {
                layer,
                shape: new_shape,
            },
        ];
        let _ = self.engine.run_batch("Extract Subpath", &cmds);
        self.node_selection.clear();
        self.selection = vec![new_id];
    }

    /// Object > Align with Pixel Grid: move the selection so its bounds
    /// sit on whole 96 dpi pixels.
    pub fn align_to_pixel_grid(&mut self) {
        let ids: Vec<ShapeId> = self.selection.clone();
        self.align_shapes_to_pixel_grid(&ids);
    }

    /// Snap the bottom-left corner of each object's bounds to the 96 dpi grid.
    pub fn align_shapes_to_pixel_grid(&mut self, ids: &[ShapeId]) {
        let px = 25.4 / 96.0;
        for id in ids {
            let Some(s) = self.doc().find_shape(*id).cloned() else {
                continue;
            };
            let b = s.bounds();
            let dx = (b.x0 / px).round() * px - b.x0;
            let dy = (b.y0 / px).round() * px - b.y0;
            if dx.abs() > 1e-9 || dy.abs() > 1e-9 {
                self.run(Command::TransformShapes {
                    shapes: vec![s.id],
                    transform: tracedraw_core::Affine::translate((dx, dy)),
                });
            }
        }
    }

    /// Object > Object Hinting: objects flagged for hinting stay on the pixel
    /// grid (they are re-aligned after every move).
    pub fn object_hinted(&self, id: ShapeId) -> bool {
        self.doc()
            .find_shape(id)
            .map(|s| s.data.iter().any(|(k, v)| k == "hinting" && v == "1"))
            .unwrap_or(false)
    }

    pub fn toggle_object_hinting(&mut self) {
        let ids = self.selection.clone();
        let all_on = ids.iter().all(|id| self.object_hinted(*id));
        for id in &ids {
            let Some(s) = self.doc().find_shape(*id).cloned() else {
                continue;
            };
            let mut data = s.data;
            data.retain(|(k, _)| k != "hinting");
            if !all_on {
                data.push(("hinting".into(), "1".into()));
            }
            self.run(Command::SetObjectData { shape: *id, data });
        }
        if !all_on {
            self.align_shapes_to_pixel_grid(&ids);
        }
    }

    /// Re-align hinted objects among `ids` (after a transform).
    pub fn apply_hinting(&mut self, ids: &[ShapeId]) {
        let hinted: Vec<ShapeId> = ids
            .iter()
            .copied()
            .filter(|id| self.object_hinted(*id))
            .collect();
        if !hinted.is_empty() {
            self.align_shapes_to_pixel_grid(&hinted);
        }
    }
}

// ----- Eraser ----------------------------------------------------------------------

impl App {
    /// Eraser: subtract the nib's band along `points` (page space) from
    /// the selected objects, or from the object under the first point.
    /// Objects become curves; what vanishes entirely is deleted.
    pub fn erase_along(&mut self, points: &[Point]) {
        use tracedraw_core::document::Shape;
        use tracedraw_core::geometry::Shape as _;
        let Some(first) = points.first().copied() else {
            return;
        };
        let band =
            tracedraw_core::shaping::stroke_band(points, self.eraser_width, self.eraser_square);
        if band.elements().is_empty() {
            return;
        }
        let targets: Vec<Shape> = if self.selection.is_empty() {
            self.hit_test(first)
                .and_then(|id| self.doc().find_shape(id).cloned())
                .into_iter()
                .collect()
        } else {
            self.selected_shapes()
        };
        let mut cmds = Vec::new();
        let mut deleted = Vec::new();
        for s in targets {
            if matches!(
                s.kind,
                ShapeKind::Bitmap { .. } | ShapeKind::Group { .. } | ShapeKind::ClipFrame { .. }
            ) {
                continue;
            }
            let page_path = s.page_path();
            // Quick reject: the band must touch the object's bounds.
            if page_path
                .bounding_box()
                .intersect(band.bounding_box())
                .area()
                <= 0.0
            {
                continue;
            }
            let remaining = tracedraw_core::shaping::overlay(
                &page_path,
                &band,
                tracedraw_core::shaping::Op::Trim,
            );
            if remaining.elements().is_empty() {
                deleted.push(s.id);
                continue;
            }
            let local = s.transform.inverse() * remaining;
            cmds.push(Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Path {
                    path: local,
                    closed: true,
                },
            });
        }
        if !deleted.is_empty() {
            cmds.push(Command::DeleteShapes {
                shapes: deleted.clone(),
            });
            self.selection.retain(|id| !deleted.contains(id));
        }
        if !cmds.is_empty() {
            let _ = self.engine.run_batch("Eraser", &cmds);
        }
    }
}

#[cfg(test)]
mod constrain_tests {
    use super::constrain_axis;
    use crate::app::App;
    use tracedraw_core::{document::Guide, geometry::Point, geometry::Vec2, Command};

    /// A headless app at 1 px per mm with a guideline at x = 50 and the
    /// snapping radius at 10 px (10 mm).
    fn app_with_guide() -> App {
        let mut app = App::headless();
        app.view.zoom = 1.0;
        let page = app.page;
        app.run(Command::AddGuide {
            page,
            guide: Guide::vertical(50.0),
        });
        app
    }

    #[test]
    fn a_grabbed_node_snaps_to_a_guideline_and_the_move_follows_it() {
        let app = app_with_guide();
        // The node at x = 45 moved by 4.5 mm lands at 49.5, which snaps to 50.
        let d = app.snapped_node_move(Point::new(45.0, 20.0), Vec2::new(4.5, 0.0), None, &[]);
        assert!((d.x - 5.0).abs() < 1e-9, "{d:?}");
        assert!(d.y.abs() < 1e-9, "{d:?}");
    }

    #[test]
    fn a_constrained_node_move_stays_on_its_axis_after_snapping() {
        let app = app_with_guide();
        // Horizontal: the guideline pulls x, y stays put.
        let d = app.snapped_node_move(Point::new(45.0, 20.0), Vec2::new(4.5, 3.0), Some(true), &[]);
        assert!((d.x - 5.0).abs() < 1e-9 && d.y == 0.0, "{d:?}");
    }

    #[test]
    fn a_constrained_move_keeps_one_axis() {
        let mut d = Vec2::new(3.0, 4.0);
        constrain_axis(&mut d, Some(true));
        assert_eq!((d.x, d.y), (3.0, 0.0));

        let mut d = Vec2::new(3.0, 4.0);
        constrain_axis(&mut d, Some(false));
        assert_eq!((d.x, d.y), (0.0, 4.0));

        let mut d = Vec2::new(3.0, 4.0);
        constrain_axis(&mut d, None);
        assert_eq!((d.x, d.y), (3.0, 4.0));
    }
}

#[cfg(test)]
mod eraser_tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    #[test]
    fn a_band_through_a_rectangle_splits_it_and_a_dot_outside_does_nothing() {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 40.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .unwrap();
        app.select(vec![id]);
        app.eraser_width = 4.0;
        // Vertical band down the middle: two pieces remain.
        app.erase_along(&[Point::new(20.0, -5.0), Point::new(20.0, 25.0)]);
        let s = app.doc().find_shape(id).unwrap();
        let ShapeKind::Path { path, .. } = &s.kind else {
            panic!("{:?}", s.kind);
        };
        let subpaths = path
            .elements()
            .iter()
            .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
            .count();
        assert_eq!(subpaths, 2);
        let b = s.bounds();
        assert!(
            (b.width() - 40.0).abs() < 0.1 && (b.height() - 20.0).abs() < 0.1,
            "{b:?}"
        );
        // A dot far away leaves it alone.
        app.erase_along(&[Point::new(100.0, 100.0)]);
        assert!(app.doc().find_shape(id).is_some());
        // Erasing everything deletes the object.
        app.eraser_width = 100.0;
        app.erase_along(&[Point::new(0.0, 10.0), Point::new(40.0, 10.0)]);
        assert!(app.doc().find_shape(id).is_none());
    }
}

#[cfg(test)]
mod bitmap_node_tests {
    use super::*;
    use tracedraw_core::geometry::{Rect, Vec2};

    fn picture(app: &mut App) -> ShapeId {
        let png = tiny_skia::Pixmap::new(2, 2)
            .and_then(|p| p.encode_png().ok())
            .unwrap_or_default();
        app.new_shape(ShapeKind::Bitmap {
            rect: Rect::new(10.0, 10.0, 50.0, 40.0),
            width_px: 2,
            height_px: 2,
            png,
            fx: None,
        })
        .expect("picture")
    }

    #[test]
    fn a_pictures_corner_crops_it_and_double_click_keeps_it_a_picture() {
        let mut app = App::headless();
        let id = picture(&mut app);
        app.select(vec![id]);
        let corners = bitmap_corners(app.doc().find_shape(id).expect("shape")).expect("corners");
        assert_eq!(corners[0], Point::new(10.0, 10.0));
        // A corner of the selected picture is a node for the Shape tool.
        app.view.zoom = 1.0;
        assert_eq!(app.bitmap_corner_at(Point::new(10.5, 10.5)), Some((id, 0)));
        // Cropping: the picture goes into a clip frame of its rectangle,
        // whose nodes the Shape tool now edits.
        let frame = app.crop_bitmap_to_frame(id).expect("frame");
        assert_eq!(app.node_target(id), frame);
        app.node_selection = vec![(frame, 2)];
        let start = app.start_curves();
        assert_eq!(start.len(), 1);
        app.move_start_nodes(
            &start,
            Vec2::new(-20.0, -10.0),
            Point::new(50.0, 40.0),
            false,
        );
        let s = app.doc().find_shape(id).expect("clip frame");
        let ShapeKind::ClipFrame { frame: f, contents } = &s.kind else {
            panic!("not a clip frame: {:?}", s.kind);
        };
        assert!(matches!(contents[0].kind, ShapeKind::Bitmap { .. }));
        let b = tracedraw_core::geometry::Shape::bounding_box(&f.page_path());
        assert!(
            (b.x1 - 50.0).abs() < 1e-9 && (b.y1 - 40.0).abs() < 1e-9,
            "{b:?}"
        );
        let moved = nodes::nodes(match &f.kind {
            ShapeKind::Path { path, .. } => path,
            _ => panic!("frame"),
        })
        .into_iter()
        .any(|n| (n.pos - Point::new(30.0, 30.0)).hypot() < 1e-9);
        assert!(moved);
        // A double click with the Shape tool turns shapes into curves, not
        // pictures (#60).
        assert!(!curves_on_double_click(&contents[0].kind));
        assert!(curves_on_double_click(&ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 1.0, 1.0),
            radius: 0.0,
            corners: None,
        }));
    }
}

/// Whether every subpath of `p` ends with a close.
fn every_subpath_closed(p: &BezPath) -> bool {
    let mut open = false;
    let mut any = false;
    for el in p.elements() {
        match el {
            tracedraw_core::geometry::PathEl::MoveTo(_) => {
                if open {
                    return false;
                }
                open = true;
                any = true;
            }
            tracedraw_core::geometry::PathEl::ClosePath => open = false,
            _ => {}
        }
    }
    any && !open
}
