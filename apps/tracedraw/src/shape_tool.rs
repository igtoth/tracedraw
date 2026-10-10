//! The Shape tool: select nodes, drag nodes and handles, add and delete
//! nodes, change node and segment types. Edits go through
//! `Command::SetShapeKind`, collapsed into one undo step per drag.

use crate::app::{App, Drag};
use egui::{Modifiers, PointerButton, Response};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{BezPath, Point, Rect},
    nodes::{self, NodeType, Which},
    Command, ShapeId,
};

const NODE_PX: f64 = 5.0;

pub struct NodeHit {
    pub shape: ShapeId,
    pub index: usize,
}

impl App {
    fn path_of(&self, id: ShapeId) -> Option<(BezPath, bool, tracedraw_core::geometry::Affine)> {
        let (_, s) = self.doc().shape(id).ok()?;
        match &s.kind {
            ShapeKind::Path { path, closed } => Some((path.clone(), *closed, s.transform)),
            _ => None,
        }
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
        let tol = NODE_PX / self.view.zoom as f64;
        for id in &self.selection {
            let Some((path, _, t)) = self.path_of(*id) else {
                continue;
            };
            let lp = t.inverse() * p;
            for n in nodes::nodes(&path) {
                if (n.pos - lp).hypot() <= tol {
                    return Some(NodeHit {
                        shape: *id,
                        index: n.index,
                    });
                }
            }
        }
        None
    }

    /// A rectangle, ellipse or polygon node of the selection under the
    /// pointer, with its object.
    pub fn kind_node_at(
        &self,
        p: Point,
    ) -> Option<(tracedraw_core::Shape, crate::kind_nodes::KindNode)> {
        let tol = NODE_PX / self.view.zoom as f64;
        self.selected_shapes()
            .into_iter()
            .find_map(|s| crate::kind_nodes::node_at(&s, p, tol).map(|n| (s, n)))
    }

    /// Control handle under the pointer, for selected nodes only.
    fn ctrl_handle_at(&self, p: Point) -> Option<(ShapeId, usize, Which)> {
        let tol = NODE_PX / self.view.zoom as f64;
        for (id, index) in &self.node_selection {
            let Some((path, _, t)) = self.path_of(*id) else {
                continue;
            };
            let lp = t.inverse() * p;
            if let Some(n) = nodes::nodes(&path).into_iter().find(|n| n.index == *index) {
                if let Some(c) = n.ctrl_in {
                    if (c - lp).hypot() <= tol {
                        return Some((*id, *index, Which::In));
                    }
                }
                if let Some(c) = n.ctrl_out {
                    if (c - lp).hypot() <= tol {
                        return Some((*id, *index, Which::Out));
                    }
                }
            }
        }
        None
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
            if let Some(id) = self.hit_test(p) {
                self.select(vec![id]);
                self.convert_to_curves();
            }
            return;
        }

        if response.drag_started_by(PointerButton::Primary) {
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
                Drag::NodeMove {
                    start,
                    from,
                    grab,
                    begun,
                } => {
                    let mut d = p - from;
                    // Ctrl keeps the move horizontal or vertical.
                    if mods.ctrl || mods.command {
                        if d.x.abs() >= d.y.abs() {
                            d.y = 0.0;
                        } else {
                            d.x = 0.0;
                        }
                    }
                    self.move_start_nodes(&start, d, grab, begun);
                    self.drag = Drag::NodeMove {
                        start,
                        from,
                        grab,
                        begun: true,
                    };
                }
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
                        let lp = t.inverse() * p;
                        let ty = if mods.ctrl {
                            NodeType::Cusp
                        } else {
                            nodes::node_type(&path, index)
                        };
                        let np = nodes::move_handle(&path, index, which, lp, ty);
                        self.set_path(shape, np, closed, "Edit Handle");
                    }
                }
                Drag::NodeMarquee { start, .. } => {
                    self.drag = Drag::NodeMarquee { start, current: p }
                }
                Drag::KindNode {
                    shape,
                    node,
                    start,
                    single,
                    begun,
                    mut turn,
                } => {
                    if let Some(kind) = crate::kind_nodes::drag(&start, node, p, single, &mut turn)
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
                self.click_node((hit.shape, hit.index), mods);
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
        for id in &self.selection {
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
    }

    /// Nearest segment of a selected curve within a few pixels.
    fn segment_at(&self, p: Point) -> Option<(ShapeId, usize, f64)> {
        let tol = 6.0 / self.view.zoom as f64;
        let mut best: Option<(ShapeId, usize, f64, f64)> = None;
        for id in &self.selection {
            let Some((path, _, t)) = self.path_of(*id) else {
                continue;
            };
            let lp = t.inverse() * p;
            if let Some((i, tt, d)) = nodes::nearest_segment(&path, lp) {
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

    pub fn delete_selected_nodes(&mut self) {
        self.for_each_selected_node("Delete Nodes", nodes::delete_node);
        self.node_selection.clear();
    }

    pub fn add_node_midpoints(&mut self) {
        // Add a node in the middle of the segment ending at each selected node.
        self.for_each_selected_node("Add Nodes", |p, i| nodes::add_node(p, i, 0.5));
        self.node_selection.clear();
    }

    pub fn set_selected_node_type(&mut self, ty: NodeType) {
        self.for_each_selected_node("Node Type", move |p, i| nodes::set_node_type(p, i, ty));
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
        let mut sel = Vec::new();
        for id in &self.selection {
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
        let (path, _, _) = self.path_of(id)?;
        Some(nodes::node_type(&path, i))
    }
}

impl App {
    /// Join two selected end nodes (of the same or different curves).
    pub fn join_selected_nodes(&mut self) {
        if self.node_selection.len() != 2 {
            return;
        }
        let (a, ia) = self.node_selection[0];
        let (b, ib) = self.node_selection[1];
        if a == b {
            // Same curve: close it.
            if let Some((path, _, _)) = self.path_of(a) {
                let np = nodes::close_subpath(&path, ia.min(ib));
                self.run(Command::SetShapeKind {
                    shape: a,
                    kind: ShapeKind::Path {
                        path: np,
                        closed: true,
                    },
                });
            }
            self.node_selection.clear();
            return;
        }
        // Different curves: combine them with a connecting line.
        self.selection = vec![a, b];
        self.join_two_curves();
        self.node_selection.clear();
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
