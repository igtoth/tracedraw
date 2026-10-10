//! More of the Shape tool, as the target design has it: Ctrl+click
//! adds or removes a node, Shift+click selects the run of nodes up to the
//! one clicked, Tab and Shift+Tab step through the nodes, the selected
//! nodes can be stretched, scaled, rotated and skewed with handles around
//! them, Reflect Nodes mirrors a move onto the nodes across the selection,
//! Elastic mode moves farther nodes less, a freehand marquee selects nodes,
//! segments between selected nodes can be copied, cut and duplicated,
//! Extend Curve to Close joins two end nodes, and Curve smoothness removes
//! nodes.

use crate::app::{App, Handle};
use egui::Modifiers;
use std::collections::BTreeMap;
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, BezPath, Point, Rect, Shape as _, Vec2},
    nodes, Command, Shape, ShapeId,
};

/// The Shape tool's node transform buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NodeTransformMode {
    #[default]
    None,
    /// Corner handles scale, side handles stretch.
    StretchScale,
    /// Corner handles rotate, side handles skew.
    RotateSkew,
}

/// A run of nodes Shift+click selected: from `anchor` to `end`, with or
/// against the path's direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeRun {
    pub shape: ShapeId,
    pub anchor: usize,
    pub end: usize,
    pub forward: bool,
}

/// A curve as it was when a node drag began.
#[derive(Debug, Clone)]
pub struct StartCurve {
    pub shape: ShapeId,
    pub path: BezPath,
    pub closed: bool,
    pub transform: Affine,
    pub indices: Vec<usize>,
}

/// Is `p` inside the polygon `poly` (even-odd)?
pub fn in_polygon(poly: &[Point], p: Point) -> bool {
    let mut inside = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y);
            if p.x < x {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// The affine a node transform handle makes: `handle` dragged from
/// `from` to `to` over the box `b` (page space). Shift (`center`) works
/// about the box's centre.
pub fn handle_transform(
    mode: NodeTransformMode,
    handle: Handle,
    b: Rect,
    from: Point,
    to: Point,
    center: bool,
) -> Affine {
    let c = b.center();
    match mode {
        NodeTransformMode::None => Affine::IDENTITY,
        NodeTransformMode::StretchScale => {
            // The fixed point: the opposite corner or side, or the centre.
            let (ax, ay) = match handle {
                Handle::NW => (b.x1, b.y0),
                Handle::N => (c.x, b.y0),
                Handle::NE => (b.x0, b.y0),
                Handle::W => (b.x1, c.y),
                Handle::E => (b.x0, c.y),
                Handle::SW => (b.x1, b.y1),
                Handle::S => (c.x, b.y1),
                Handle::SE => (b.x0, b.y1),
            };
            let anchor = if center { c } else { Point::new(ax, ay) };
            let ratio = |a: f64, f: f64, t: f64| {
                let d = f - a;
                if d.abs() < 1e-9 {
                    1.0
                } else {
                    (t - a) / d
                }
            };
            let (mut sx, mut sy) = (ratio(anchor.x, from.x, to.x), ratio(anchor.y, from.y, to.y));
            match handle {
                Handle::N | Handle::S => sx = 1.0,
                Handle::E | Handle::W => sy = 1.0,
                _ => {
                    // Corners scale in proportion.
                    let k = if sx.abs() > sy.abs() { sx } else { sy };
                    sx = k;
                    sy = k;
                }
            }
            Affine::translate(anchor.to_vec2())
                * Affine::scale_non_uniform(sx, sy)
                * Affine::translate(-anchor.to_vec2())
        }
        NodeTransformMode::RotateSkew => {
            if handle.is_corner() {
                let a0 = (from - c).atan2();
                let a1 = (to - c).atan2();
                Affine::translate(c.to_vec2())
                    * Affine::rotate(a1 - a0)
                    * Affine::translate(-c.to_vec2())
            } else {
                // Side handles skew along their side, about the centre.
                let h = (b.height() / 2.0).max(1e-9);
                let w = (b.width() / 2.0).max(1e-9);
                let skew = match handle {
                    Handle::N => Affine::new([1.0, 0.0, (to.x - from.x) / h, 1.0, 0.0, 0.0]),
                    Handle::S => Affine::new([1.0, 0.0, -(to.x - from.x) / h, 1.0, 0.0, 0.0]),
                    Handle::E => Affine::new([1.0, (to.y - from.y) / w, 0.0, 1.0, 0.0, 0.0]),
                    _ => Affine::new([1.0, -(to.y - from.y) / w, 0.0, 1.0, 0.0, 0.0]),
                };
                Affine::translate(c.to_vec2()) * skew * Affine::translate(-c.to_vec2())
            }
        }
    }
}

impl App {
    fn curve_of(&self, id: ShapeId) -> Option<(BezPath, bool, Affine)> {
        let s = self.doc().find_shape(id)?;
        match &s.kind {
            ShapeKind::Path { path, closed } => Some((path.clone(), *closed, s.transform)),
            _ => None,
        }
    }

    /// The selected nodes, by curve.
    pub fn nodes_by_shape(&self) -> BTreeMap<ShapeId, Vec<usize>> {
        let mut out: BTreeMap<ShapeId, Vec<usize>> = BTreeMap::new();
        for (id, i) in &self.node_selection {
            out.entry(*id).or_default().push(*i);
        }
        out
    }

    /// Where the selected nodes are on the page.
    pub fn selected_node_points(&self) -> Vec<(ShapeId, usize, Point)> {
        let mut out = Vec::new();
        for (id, idxs) in self.nodes_by_shape() {
            let Some((path, _, t)) = self.curve_of(id) else {
                continue;
            };
            for n in nodes::nodes(&path) {
                if idxs.contains(&n.index) {
                    out.push((id, n.index, t * n.pos));
                }
            }
        }
        out
    }

    /// The box around the selected nodes, when there are two or more.
    pub fn selected_nodes_bounds(&self) -> Option<Rect> {
        let pts = self.selected_node_points();
        if pts.len() < 2 {
            return None;
        }
        let mut r = Rect::from_points(pts[0].2, pts[0].2);
        for (_, _, p) in &pts[1..] {
            r = r.union_pt(*p);
        }
        Some(r)
    }

    /// The node transform handles (page space) while a transform mode is on.
    pub fn node_transform_handles(&self) -> Vec<(Handle, Point)> {
        if self.node_transform == NodeTransformMode::None {
            return Vec::new();
        }
        let Some(b) = self.selected_nodes_bounds() else {
            return Vec::new();
        };
        let c = b.center();
        vec![
            (Handle::NW, Point::new(b.x0, b.y1)),
            (Handle::N, Point::new(c.x, b.y1)),
            (Handle::NE, Point::new(b.x1, b.y1)),
            (Handle::W, Point::new(b.x0, c.y)),
            (Handle::E, Point::new(b.x1, c.y)),
            (Handle::SW, Point::new(b.x0, b.y0)),
            (Handle::S, Point::new(c.x, b.y0)),
            (Handle::SE, Point::new(b.x1, b.y0)),
        ]
    }

    /// The node transform handle under the pointer.
    pub fn node_transform_handle_at(&self, p: Point) -> Option<Handle> {
        let tol = 5.0 / self.view.zoom.max(1e-6) as f64;
        self.node_transform_handles()
            .into_iter()
            .find(|(_, q)| (*q - p).hypot() <= tol)
            .map(|(h, _)| h)
    }

    /// The selected curves with their selected nodes, as they are now.
    pub fn start_curves(&self) -> Vec<StartCurve> {
        self.nodes_by_shape()
            .into_iter()
            .filter_map(|(id, indices)| {
                let (path, closed, transform) = self.curve_of(id)?;
                Some(StartCurve {
                    shape: id,
                    path,
                    closed,
                    transform,
                    indices,
                })
            })
            .collect()
    }

    /// Apply the page-space affine `a` to the selected nodes of `start`;
    /// the first step of a drag is recorded, later ones join it.
    pub fn transform_start_nodes(&mut self, start: &[StartCurve], a: Affine, begun: bool) {
        let cmds: Vec<Command> = start
            .iter()
            .map(|c| {
                let local = c.transform.inverse() * a * c.transform;
                Command::SetShapeKind {
                    shape: c.shape,
                    kind: ShapeKind::Path {
                        path: nodes::transform_nodes(&c.path, &c.indices, local),
                        closed: c.closed,
                    },
                }
            })
            .collect();
        if cmds.is_empty() {
            return;
        }
        if begun {
            for cmd in &cmds {
                let _ = self.engine.amend(cmd);
            }
        } else {
            let _ = self.engine.run_batch("Transform Nodes", &cmds);
        }
    }

    /// Move the selected nodes from where `start` had them by the page
    /// delta `d`, with Reflect Nodes and Elastic mode; `grab` is the node
    /// the drag began on.
    pub fn move_start_nodes(&mut self, start: &[StartCurve], d: Vec2, grab: Point, begun: bool) {
        // Every selected node's page position, for the mirror axis and the
        // elastic weights.
        let mut all: Vec<Point> = Vec::new();
        for c in start {
            for n in nodes::nodes(&c.path) {
                if c.indices.contains(&n.index) {
                    all.push(c.transform * n.pos);
                }
            }
        }
        if all.is_empty() {
            return;
        }
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for p in &all {
            x0 = x0.min(p.x);
            x1 = x1.max(p.x);
            y0 = y0.min(p.y);
            y1 = y1.max(p.y);
        }
        let axis = Point::new((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let far = all
            .iter()
            .map(|p| (*p - grab).hypot())
            .fold(0.0, f64::max)
            .max(1e-9);
        let (rh, rv) = self.reflect_nodes;
        let side = |v: f64, a: f64| {
            if v < a - 1e-9 {
                -1
            } else if v > a + 1e-9 {
                1
            } else {
                0
            }
        };
        let mut cmds = Vec::new();
        for c in start {
            let inv = c.transform.inverse();
            let mut path = c.path.clone();
            for n in nodes::nodes(&c.path) {
                if !c.indices.contains(&n.index) {
                    continue;
                }
                let p = c.transform * n.pos;
                let mut dd = d;
                // Reflect: nodes across the axis from the grabbed one move
                // the mirrored way.
                if rh && side(p.x, axis.x) != 0 && side(p.x, axis.x) != side(grab.x, axis.x) {
                    dd.x = -dd.x;
                }
                if rv && side(p.y, axis.y) != 0 && side(p.y, axis.y) != side(grab.y, axis.y) {
                    dd.y = -dd.y;
                }
                if self.elastic_mode {
                    // Nodes farther from the grabbed one move less.
                    let w = 1.0 - 0.5 * (p - grab).hypot() / far;
                    dd *= w;
                }
                let ld = (inv * (Point::ZERO + dd)) - (inv * Point::ZERO);
                path = nodes::move_node(&path, n.index, ld);
            }
            cmds.push(Command::SetShapeKind {
                shape: c.shape,
                kind: ShapeKind::Path {
                    path,
                    closed: c.closed,
                },
            });
        }
        if begun {
            for cmd in &cmds {
                let _ = self.engine.amend(cmd);
            }
        } else {
            let _ = self.engine.run_batch("Move Nodes", &cmds);
        }
    }

    /// A click on a node: alone, Ctrl adds or removes it, Shift selects
    /// the run of nodes from the last one selected to it (the shorter way
    /// round a closed subpath); Shift+clicking the end of that run again
    /// selects the other way round instead.
    pub fn click_node(&mut self, key: (ShapeId, usize), mods: Modifiers) {
        if mods.command || mods.ctrl {
            self.node_run = None;
            if let Some(i) = self.node_selection.iter().position(|k| *k == key) {
                self.node_selection.remove(i);
            } else {
                self.node_selection.push(key);
            }
            return;
        }
        if !mods.shift {
            self.node_run = None;
            self.node_selection = vec![key];
            return;
        }
        let Some((path, _, _)) = self.curve_of(key.0) else {
            if !self.node_selection.contains(&key) {
                self.node_selection.push(key);
            }
            return;
        };
        // The end of the last run clicked again: the other way round.
        if let Some(run) = self.node_run {
            let same = run.shape == key.0
                && run.end == key.1
                && self.node_selection.last() == Some(&key)
                && self.node_selection.contains(&(run.shape, run.anchor));
            if same {
                if let Some((other, dir)) =
                    nodes::node_run(&path, run.anchor, run.end, Some(!run.forward))
                {
                    let old = nodes::node_run(&path, run.anchor, run.end, Some(run.forward))
                        .map(|r| r.0)
                        .unwrap_or_default();
                    self.node_selection
                        .retain(|(id, i)| *id != key.0 || !old.contains(i));
                    self.add_run(key.0, &other);
                    self.node_run = Some(NodeRun {
                        forward: dir,
                        ..run
                    });
                }
                return;
            }
        }
        if let Some(&(id, last)) = self.node_selection.last() {
            if id == key.0 && last != key.1 {
                if let Some((run, dir)) = nodes::node_run(&path, last, key.1, None) {
                    self.add_run(id, &run);
                    self.node_run = Some(NodeRun {
                        shape: id,
                        anchor: last,
                        end: key.1,
                        forward: dir,
                    });
                    return;
                }
            }
        }
        // The first node of a run, or one on another subpath.
        self.node_run = None;
        if !self.node_selection.contains(&key) {
            self.node_selection.push(key);
        }
    }

    /// Add a run of nodes to the node selection, its last node last.
    fn add_run(&mut self, id: ShapeId, run: &[usize]) {
        for i in run {
            self.node_selection.retain(|k| *k != (id, *i));
            self.node_selection.push((id, *i));
        }
    }

    /// Tab and Shift+Tab with the Shape tool: the next or previous node of
    /// the curve; true when it did something.
    pub fn cycle_node(&mut self, forward: bool) -> bool {
        let Some(&(id, cur)) = self.node_selection.last() else {
            return false;
        };
        let Some((path, _, _)) = self.curve_of(id) else {
            return false;
        };
        let list: Vec<usize> = nodes::nodes(&path).into_iter().map(|n| n.index).collect();
        let Some(k) = list.iter().position(|i| *i == cur) else {
            return false;
        };
        let n = list.len();
        let next = if forward {
            (k + 1) % n
        } else {
            (k + n - 1) % n
        };
        self.node_selection = vec![(id, list[next])];
        true
    }

    /// Select the nodes of the selected curves inside a freehand marquee.
    pub fn finish_node_lasso(&mut self, poly: &[Point]) {
        let mut sel = Vec::new();
        for id in self.selection.clone() {
            let Some((path, _, t)) = self.curve_of(id) else {
                continue;
            };
            for n in nodes::nodes(&path) {
                if in_polygon(poly, t * n.pos) {
                    sel.push((id, n.index));
                }
            }
        }
        self.node_selection = sel;
        self.choose_chars_in(|q| in_polygon(poly, q));
    }

    /// The segments between selected nodes as new curves, styled like
    /// their curve (page space, ready to paste or place).
    fn selected_segment_shapes(&self) -> Vec<Shape> {
        let mut out = Vec::new();
        for (id, idxs) in self.nodes_by_shape() {
            let Some(s) = self.doc().find_shape(id).cloned() else {
                continue;
            };
            let ShapeKind::Path { path, .. } = &s.kind else {
                continue;
            };
            let segs = nodes::selected_segments(path, &idxs);
            if segs.elements().is_empty() {
                continue;
            }
            let mut piece = s.clone();
            piece.kind = ShapeKind::Path {
                path: segs,
                closed: false,
            };
            piece.effects.clear();
            out.push(piece);
        }
        out
    }

    /// Ctrl+C with segments chosen in the Shape tool; true when it copied.
    pub fn copy_segments(&mut self) -> bool {
        let shapes = self.selected_segment_shapes();
        if shapes.is_empty() {
            return false;
        }
        self.clipboard = Some(crate::app::Clipboard { shapes });
        true
    }

    /// Ctrl+X: copy the segments, then take them out of their curves.
    pub fn cut_segments(&mut self) -> bool {
        if !self.copy_segments() {
            return false;
        }
        let mut cmds = Vec::new();
        for (id, idxs) in self.nodes_by_shape() {
            let Some((path, _, _)) = self.curve_of(id) else {
                continue;
            };
            let left = nodes::without_segments(&path, &idxs);
            if left.elements().is_empty() {
                cmds.push(Command::DeleteShapes { shapes: vec![id] });
            } else {
                cmds.push(Command::SetShapeKind {
                    shape: id,
                    kind: ShapeKind::Path {
                        path: left,
                        closed: false,
                    },
                });
            }
        }
        if let Err(e) = self.engine.run_batch("Cut Segments", &cmds) {
            self.status = e.to_string();
        }
        self.node_selection.clear();
        true
    }

    /// Ctrl+D: the segments as new curves at the duplicate offset.
    pub fn duplicate_segments(&mut self) -> bool {
        let shapes = self.selected_segment_shapes();
        if shapes.is_empty() {
            return false;
        }
        let Some(layer) = self.active_layer() else {
            return false;
        };
        let mut ids = Vec::new();
        let mut cmds = Vec::new();
        for mut s in shapes {
            let id = self.engine.new_shape_id();
            s.id = id;
            s.transform = Affine::translate(self.duplicate_offset) * s.transform;
            ids.push(id);
            cmds.push(Command::AddShape { layer, shape: s });
        }
        if let Err(e) = self.engine.run_batch("Duplicate Segments", &cmds) {
            self.status = e.to_string();
        }
        self.select(ids);
        true
    }

    /// Extend Curve to Close: two selected end nodes of one curve are
    /// joined by a straight segment (closing a subpath, or making one of
    /// two).
    pub fn extend_curve_to_close(&mut self) {
        let by = self.nodes_by_shape();
        let Some((id, idxs)) = by.into_iter().find(|(_, v)| v.len() == 2) else {
            return;
        };
        let Some((path, closed, _)) = self.curve_of(id) else {
            return;
        };
        if let Some(np) = nodes::connect_ends(&path, idxs[0], idxs[1]) {
            let all_closed = np
                .elements()
                .iter()
                .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::MoveTo(_)))
                .count()
                == np
                    .elements()
                    .iter()
                    .filter(|e| matches!(e, tracedraw_core::geometry::PathEl::ClosePath))
                    .count();
            self.run(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Path {
                    path: np,
                    closed: closed || all_closed,
                },
            });
            self.node_selection.clear();
        }
    }

    /// Curve smoothness (0 to 100): remove nodes of the curves as they
    /// were when the slider was taken; the selected nodes only when some
    /// are selected. The first change is recorded, later ones join it.
    pub fn smooth_curves(&mut self, start: &[StartCurve], amount: f64, begun: bool) {
        let mut cmds = Vec::new();
        for c in start {
            let b = c.path.bounding_box();
            let size = b.width().max(b.height()).max(1e-6);
            let tol = (amount.clamp(0.0, 100.0) / 100.0).powi(2) * size * 0.1;
            let path = if c.indices.is_empty() {
                nodes::reduce_nodes(&c.path, tol)
            } else {
                nodes::reduce_chosen_nodes(&c.path, &c.indices, tol)
            };
            cmds.push(Command::SetShapeKind {
                shape: c.shape,
                kind: ShapeKind::Path {
                    path,
                    closed: c.closed,
                },
            });
        }
        if cmds.is_empty() {
            return;
        }
        if begun {
            for cmd in &cmds {
                let _ = self.engine.amend(cmd);
            }
        } else {
            let _ = self.engine.run_batch("Curve Smoothness", &cmds);
        }
    }

    /// The curves the smoothness slider works on: the curves with
    /// selected nodes, else every selected curve (all its nodes).
    pub fn smoothing_curves(&self) -> Vec<StartCurve> {
        let chosen = self.start_curves();
        if !chosen.is_empty() {
            return chosen;
        }
        self.selection
            .iter()
            .filter_map(|id| {
                let (path, closed, transform) = self.curve_of(*id)?;
                Some(StartCurve {
                    shape: *id,
                    path,
                    closed,
                    transform,
                    indices: Vec::new(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app_with_square() -> (App, ShapeId) {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Path {
                path: Rect::new(0.0, 0.0, 10.0, 10.0).to_path(0.01),
                closed: true,
            })
            .expect("a square");
        app.select(vec![id]);
        app.tool = crate::tools::Tool::Shape;
        (app, id)
    }

    #[test]
    fn ctrl_toggles_and_shift_selects_a_run() {
        let (mut app, id) = app_with_square();
        app.click_node((id, 0), Modifiers::NONE);
        app.click_node((id, 2), Modifiers::SHIFT);
        assert_eq!(app.node_selection, vec![(id, 0), (id, 1), (id, 2)]);
        // Shift+click the end again: the other way round, and back.
        app.click_node((id, 2), Modifiers::SHIFT);
        assert_eq!(app.node_selection, vec![(id, 0), (id, 3), (id, 2)]);
        app.click_node((id, 2), Modifiers::SHIFT);
        assert_eq!(app.node_selection, vec![(id, 0), (id, 1), (id, 2)]);
        app.click_node((id, 1), Modifiers::CTRL);
        assert_eq!(app.node_selection, vec![(id, 0), (id, 2)]);
        app.click_node((id, 3), Modifiers::CTRL);
        assert_eq!(app.node_selection.len(), 3);
        app.click_node((id, 1), Modifiers::NONE);
        assert_eq!(app.node_selection, vec![(id, 1)]);
        // Tab walks on, wrapping round.
        assert!(app.cycle_node(true));
        assert_eq!(app.node_selection, vec![(id, 2)]);
        app.node_selection = vec![(id, 0)];
        assert!(app.cycle_node(false));
        assert_eq!(app.node_selection, vec![(id, 3)]);
    }

    #[test]
    fn handles_scale_rotate_and_skew_the_chosen_nodes() {
        let (mut app, id) = app_with_square();
        app.node_selection = vec![(id, 1), (id, 2)];
        app.node_transform = NodeTransformMode::StretchScale;
        let b = app.selected_nodes_bounds().unwrap();
        assert_eq!(b, Rect::new(10.0, 0.0, 10.0, 10.0));
        assert_eq!(app.node_transform_handles().len(), 8);
        // The top side dragged up 10 mm stretches the right side to 20.
        let a = handle_transform(
            app.node_transform,
            Handle::N,
            b,
            Point::new(10.0, 10.0),
            Point::new(10.0, 20.0),
            false,
        );
        let start = app.start_curves();
        app.transform_start_nodes(&start, a, false);
        let pts: Vec<Point> = app
            .selected_node_points()
            .into_iter()
            .map(|(_, _, p)| p)
            .collect();
        assert!(pts.contains(&Point::new(10.0, 20.0)), "{pts:?}");
        assert!(pts.contains(&Point::new(10.0, 0.0)), "{pts:?}");
        // A quarter turn about the centre.
        let r = handle_transform(
            NodeTransformMode::RotateSkew,
            Handle::NE,
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
            false,
        );
        let q = r * Point::new(10.0, 5.0);
        assert!((q - Point::new(5.0, 10.0)).hypot() < 1e-9, "{q:?}");
        // Skewing by the top handle leaves the middle line where it is.
        let k = handle_transform(
            NodeTransformMode::RotateSkew,
            Handle::N,
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Point::new(5.0, 10.0),
            Point::new(8.0, 10.0),
            false,
        );
        assert!((k * Point::new(0.0, 5.0) - Point::new(0.0, 5.0)).hypot() < 1e-9);
        assert!((k * Point::new(0.0, 10.0) - Point::new(3.0, 10.0)).hypot() < 1e-9);
    }

    #[test]
    fn reflect_and_elastic_moves() {
        let (mut app, id) = app_with_square();
        // Bottom corners: the right one dragged right by 2 mm.
        app.node_selection = vec![(id, 0), (id, 1)];
        app.reflect_nodes = (true, false);
        let start = app.start_curves();
        app.move_start_nodes(&start, Vec2::new(2.0, 1.0), Point::new(10.0, 0.0), false);
        let pts: Vec<Point> = app
            .selected_node_points()
            .into_iter()
            .map(|(_, _, p)| p)
            .collect();
        assert!(pts.contains(&Point::new(12.0, 1.0)), "{pts:?}");
        assert!(pts.contains(&Point::new(-2.0, 1.0)), "{pts:?}");
        // Elastic: the far node moves half as much.
        app.undo();
        app.reflect_nodes = (false, false);
        app.elastic_mode = true;
        let start = app.start_curves();
        app.move_start_nodes(&start, Vec2::new(0.0, 4.0), Point::new(10.0, 0.0), false);
        let pts: Vec<Point> = app
            .selected_node_points()
            .into_iter()
            .map(|(_, _, p)| p)
            .collect();
        assert!(pts.contains(&Point::new(10.0, 4.0)), "{pts:?}");
        assert!(pts.contains(&Point::new(0.0, 2.0)), "{pts:?}");
    }

    #[test]
    fn segments_copy_cut_and_duplicate() {
        let (mut app, id) = app_with_square();
        app.node_selection = vec![(id, 1), (id, 2)];
        assert!(app.copy_segments());
        assert_eq!(app.clipboard.as_ref().map(|c| c.shapes.len()), Some(1));
        assert!(app.duplicate_segments());
        assert_eq!(app.selection.len(), 1);
        assert_ne!(app.selection[0], id);
        app.select(vec![id]);
        app.node_selection = vec![(id, 1), (id, 2)];
        assert!(app.cut_segments());
        let s = app.doc().find_shape(id).cloned().unwrap();
        let ShapeKind::Path { path, closed } = &s.kind else {
            panic!();
        };
        assert!(!closed);
        assert_eq!(nodes::nodes(path).len(), 4);
        // Nothing chosen: nothing to copy.
        app.node_selection.clear();
        assert!(!app.copy_segments());
    }

    #[test]
    fn a_freehand_marquee_picks_the_nodes_inside() {
        let (mut app, id) = app_with_square();
        let lasso = [
            Point::new(-1.0, -1.0),
            Point::new(11.0, -1.0),
            Point::new(11.0, 1.0),
            Point::new(-1.0, 1.0),
        ];
        app.finish_node_lasso(&lasso);
        assert_eq!(app.node_selection, vec![(id, 0), (id, 1)]);
        assert!(in_polygon(&lasso, Point::new(5.0, 0.0)));
        assert!(!in_polygon(&lasso, Point::new(5.0, 5.0)));
        assert!(!in_polygon(&lasso[..2], Point::new(5.0, 0.0)));
    }

    #[test]
    fn extend_to_close_and_smoothness() {
        let mut app = App::headless();
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((5.0, 0.1));
        p.line_to((10.0, 0.0));
        p.line_to((10.0, 10.0));
        let id = app
            .new_shape(ShapeKind::Path {
                path: p,
                closed: false,
            })
            .unwrap();
        app.select(vec![id]);
        app.tool = crate::tools::Tool::Shape;
        // Smoothness 100 removes the nearly straight node.
        let start = app.smoothing_curves();
        app.smooth_curves(&start, 100.0, false);
        let s = app.doc().find_shape(id).cloned().unwrap();
        assert_eq!(nodes::nodes(&s.local_path()).len(), 3);
        // The two ends joined: closed.
        app.node_selection = vec![(id, 0), (id, 2)];
        app.extend_curve_to_close();
        let s = app.doc().find_shape(id).cloned().unwrap();
        assert!(matches!(s.kind, ShapeKind::Path { closed: true, .. }));
    }
}
