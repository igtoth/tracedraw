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
                    if mods.shift {
                        self.node_selection.push(key);
                    } else {
                        self.node_selection = vec![key];
                    }
                }
                self.drag = Drag::Node { last: p };
                return;
            }
            match self.hit_test(p) {
                Some(id) if !self.selection.contains(&id) => {
                    self.select(vec![id]);
                    self.drag = Drag::NodeMarquee {
                        start: p,
                        current: p,
                    };
                }
                _ => {
                    self.drag = Drag::NodeMarquee {
                        start: p,
                        current: p,
                    }
                }
            }
        }

        if response.dragged_by(PointerButton::Primary) {
            match self.drag.clone() {
                Drag::Node { last } => {
                    let d = p - last;
                    let sel = self.node_selection.clone();
                    let mut by_shape: std::collections::BTreeMap<ShapeId, Vec<usize>> =
                        Default::default();
                    for (id, i) in sel {
                        by_shape.entry(id).or_default().push(i);
                    }
                    for (id, idxs) in by_shape {
                        let Some((mut path, closed, t)) = self.path_of(id) else {
                            continue;
                        };
                        let ld = (t.inverse() * (Point::ZERO + d)) - (t.inverse() * Point::ZERO);
                        for i in idxs {
                            path = nodes::move_node(&path, i, ld);
                        }
                        self.set_path(id, path, closed, "Move Nodes");
                    }
                    self.drag = Drag::Node { last: p };
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
                _ => {}
            }
        }

        if response.clicked_by(PointerButton::Primary) {
            if let Some(hit) = self.node_at(p) {
                let key = (hit.shape, hit.index);
                if mods.shift {
                    if let Some(i) = self.node_selection.iter().position(|k| *k == key) {
                        self.node_selection.remove(i);
                    } else {
                        self.node_selection.push(key);
                    }
                } else {
                    self.node_selection = vec![key];
                }
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
        self.join_curves();
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
        let px = 25.4 / 96.0;
        for s in self.selected_shapes() {
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
}
