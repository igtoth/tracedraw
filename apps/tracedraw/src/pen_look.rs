//! The Pen and Bezier tools while a curve is being drawn: what the canvas
//! shows (the segments placed in the outline the curve will get, the
//! Pen's preview segment to the pointer, the nodes and the last
//! node's handles), and the Pen's automatic adding and deleting of nodes
//! on the curve being drawn. Pure functions, so tests can check them.

use crate::app::{App, CurveInProgress};
use crate::tools::Tool;
use egui::Color32;
use tracedraw_core::geometry::{BezPath, Point};

/// How a node of the curve being drawn is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeLook {
    /// A node placed before: a small hollow square.
    Placed,
    /// The last node, the one the next segment leaves: a filled square in
    /// the selection blue.
    Current,
}

/// What the canvas draws for a Pen or Bezier curve being drawn.
#[derive(Debug, Clone)]
pub struct CurveLook {
    /// The segments placed so far, drawn as a hairline in `placed_color`.
    pub placed: BezPath,
    /// The colour of the outline new objects get (black without one).
    pub placed_color: Color32,
    /// Pen preview: the next segment, from the last node to the pointer
    /// (curved by the last node's outgoing handle), or the closing segment
    /// while the first node is dragged; a solid line in `preview_color`
    /// on a white line twice as wide (#670).
    pub preview: Option<BezPath>,
    pub preview_color: Color32,
    /// Every node with its mark.
    pub nodes: Vec<(Point, NodeLook)>,
    /// Handles shown, as (node, handle end): the last node's, and the
    /// first node's closing handle while the closing segment is dragged;
    /// dashed lines ending in arrow tips in `preview_color`.
    pub handles: Vec<(Point, Point)>,
}

/// The look of `c` with the pointer (as the next node would take it) at
/// `pointer`: `preview` is the Pen's preview mode, `outline` the colour
/// of the outline new objects get.
pub fn curve_look(
    c: &CurveInProgress,
    pointer: Option<Point>,
    preview: bool,
    outline: Option<Color32>,
) -> CurveLook {
    let blue = crate::theme::Tokens::SELECTION;
    let last = c.nodes.len().checked_sub(1);
    let mut nodes: Vec<(Point, NodeLook)> = c
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            let look = if Some(i) == last {
                NodeLook::Current
            } else {
                NodeLook::Placed
            };
            (n.0, look)
        })
        .collect();
    let mut handles = Vec::new();
    if let Some(i) = last {
        let (p, out) = c.nodes[i];
        if let Some(o) = out {
            handles.push((p, o));
        }
        // The incoming handle of a node reached by a curve.
        if i > 0 {
            if let Some(h) = c.in_handle(i) {
                handles.push((p, h));
            }
        }
    }
    let mut preview_path = None;
    if c.closing.is_some() {
        // The closing segment being shaped from the first node.
        if let (Some((first, _)), Some((end, end_out))) =
            (c.nodes.first().copied(), c.nodes.last().copied())
        {
            let into = c.closing_handle();
            let mut seg = BezPath::new();
            seg.move_to(end);
            if end_out.is_some() || into.is_some() {
                seg.curve_to(end_out.unwrap_or(end), into.unwrap_or(first), first);
            } else {
                seg.line_to(first);
            }
            preview_path = Some(seg);
            if let Some(h) = into {
                handles.push((first, h));
            }
            // The first node is where the drag is.
            if let Some(n) = nodes.first_mut() {
                n.1 = NodeLook::Current;
            }
        }
    } else if preview && !c.dragging_handle {
        if let (Some(p), Some((end, end_out))) = (pointer, c.nodes.last().copied()) {
            if (p - end).hypot() > 1e-9 {
                let mut seg = BezPath::new();
                seg.move_to(end);
                match end_out {
                    Some(o) => seg.curve_to(o, p, p),
                    None => seg.line_to(p),
                }
                preview_path = Some(seg);
            }
        }
    }
    CurveLook {
        placed: c.path(None),
        placed_color: outline.unwrap_or(Color32::BLACK),
        preview: preview_path,
        preview_color: blue,
        nodes,
        handles,
    }
}

/// What a Pen click on the curve being drawn does with automatic adding
/// and deleting on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PenAuto {
    /// Delete this node.
    Delete(usize),
    /// Add a node on the segment ending at this node, at this parameter.
    Add(usize, f64),
}

impl CurveInProgress {
    /// The node nearest `p` within `reach`.
    pub fn node_near(&self, p: Point, reach: f64) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, (n.0 - p).hypot()))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// The segment nearest `p` within `reach`: the node it ends at and the
    /// parameter along it.
    pub fn segment_near(&self, p: Point, reach: f64) -> Option<(usize, f64)> {
        if self.bspline || self.nodes.len() < 2 {
            return None;
        }
        let (index, t, d) = tracedraw_core::nodes::nearest_segment(&self.path(None), p)?;
        (index >= 1 && index < self.nodes.len() && d <= reach).then_some((index, t))
    }

    /// Split the segment ending at node `k` at parameter `t`: the new node
    /// lies on the curve, which keeps its shape (the nodes on either side
    /// keep their other handles). Returns the new node's index.
    pub fn insert_node(&mut self, k: usize, t: f64) -> Option<usize> {
        if k == 0 || k >= self.nodes.len() {
            return None;
        }
        let t = t.clamp(0.01, 0.99);
        let (a, a_out) = self.nodes[k - 1];
        let b = self.nodes[k].0;
        let b_in = self.in_handle(k);
        let shift = |map: &mut std::collections::BTreeMap<usize, Option<Point>>| {
            let moved: Vec<(usize, Option<Point>)> = map.split_off(&k).into_iter().collect();
            for (i, h) in moved {
                map.insert(i + 1, h);
            }
        };
        if !self.smooth || (a_out.is_none() && b_in.is_none()) {
            shift(&mut self.in_handles);
            self.nodes.insert(k, (a.lerp(b, t), None));
            return Some(k);
        }
        // De Casteljau split of the cubic.
        let (c1, c2) = (a_out.unwrap_or(a), b_in.unwrap_or(b));
        let p01 = a.lerp(c1, t);
        let p12 = c1.lerp(c2, t);
        let p23 = c2.lerp(b, t);
        let p012 = p01.lerp(p12, t);
        let p123 = p12.lerp(p23, t);
        let mid = p012.lerp(p123, t);
        // The node before keeps its incoming handle as it is.
        let a_in = self.in_handle(k - 1);
        self.in_handles.insert(k - 1, a_in);
        self.nodes[k - 1].1 = Some(p01);
        shift(&mut self.in_handles);
        self.nodes.insert(k, (mid, Some(p123)));
        self.in_handles.insert(k, Some(p012));
        self.in_handles.insert(k + 1, Some(p23));
        Some(k)
    }

    /// Delete node `i`: its neighbours are joined by a segment that keeps
    /// their handles.
    pub fn delete_node(&mut self, i: usize) -> bool {
        if i >= self.nodes.len() {
            return false;
        }
        self.nodes.remove(i);
        let old = std::mem::take(&mut self.in_handles);
        for (j, h) in old {
            if j < i {
                self.in_handles.insert(j, h);
            } else if j > i {
                self.in_handles.insert(j - 1, h);
            }
        }
        true
    }
}

impl App {
    /// How near (screen pixels) the pointer must be to the first or last
    /// node of the curve being drawn for a click to close the curve or to
    /// act on the last node. With the Pen's automatic adding and deleting
    /// on it reaches at least as far as a node is picked for deleting, so
    /// a click by the first node closes the curve and never deletes it.
    pub fn curve_end_reach_px(&self) -> f64 {
        let join = self.settings.curves.join_px();
        if self.tool == Tool::Pen && self.settings.curves.pen_auto_add_delete {
            join.max(self.settings.nodes.pick_px())
        } else {
            join
        }
    }

    /// Before a press changes the curve being drawn: keep how it was, so
    /// Ctrl+Z takes the change back. A new curve starts a fresh list.
    pub(crate) fn curve_step(&mut self) {
        if self.curve.is_none() {
            self.curve_steps.undo.clear();
        }
        self.curve_steps.undo.push(self.curve.clone());
        self.curve_steps.redo.clear();
    }

    /// Ctrl+Z while a curve is drawn: back one step (the last node, handle
    /// pull or click on a node); before the first node the curve is gone.
    /// False when no curve is being drawn.
    pub fn curve_undo_step(&mut self) -> bool {
        if self.curve.is_none() {
            return false;
        }
        let Some(before) = self.curve_steps.undo.pop() else {
            return false;
        };
        let now = std::mem::replace(&mut self.curve, before);
        self.curve_steps.redo.push(now);
        self.status = crate::i18n::trf("status.undo_n", &[("l", "Curve Node")]);
        true
    }

    /// Ctrl+Shift+Z after [`Self::curve_undo_step`]: the step again. False
    /// when nothing of the curve was undone.
    pub fn curve_redo_step(&mut self) -> bool {
        let Some(after) = self.curve_steps.redo.pop() else {
            return false;
        };
        let now = std::mem::replace(&mut self.curve, after);
        self.curve_steps.undo.push(now);
        self.status = crate::i18n::trf("status.redo_n", &[("l", "Curve Node")]);
        true
    }

    /// The Pen with automatic adding and deleting on, a curve being drawn
    /// and the pointer at `p`: the node a click deletes (not the last
    /// one, which a click makes a cusp, nor the first once a click there
    /// closes the curve) or the segment it adds a node on.
    pub fn pen_auto_at(&self, p: Point) -> Option<PenAuto> {
        if self.tool != Tool::Pen || !self.settings.curves.pen_auto_add_delete {
            return None;
        }
        let c = self.curve.as_ref()?;
        if !c.smooth || c.bspline || c.dragging_handle || c.closing.is_some() {
            return None;
        }
        let zoom = f64::from(self.view.zoom.max(1e-6));
        let join = self.curve_end_reach_px() / zoom;
        let last = c.nodes.len().checked_sub(1)?;
        // The first and last nodes keep their own meaning.
        if (c.nodes[last].0 - p).hypot() <= join {
            return None;
        }
        if c.closes_at_first() && (c.nodes[0].0 - p).hypot() <= join {
            return None;
        }
        let reach = self.settings.nodes.pick_px() / zoom;
        if let Some(i) = c.node_near(p, reach) {
            return (i != last).then_some(PenAuto::Delete(i));
        }
        c.segment_near(p, reach).map(|(k, t)| PenAuto::Add(k, t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::{ParamCurve, PathSeg};

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// Three nodes: a pulled one, a cusp, and a last one pulled to the
    /// right.
    fn sample() -> CurveInProgress {
        let mut c = CurveInProgress {
            nodes: vec![
                (pt(0.0, 0.0), Some(pt(10.0, 10.0))),
                (pt(30.0, 0.0), None),
                (pt(60.0, 0.0), Some(pt(70.0, 0.0))),
            ],
            smooth: true,
            ..Default::default()
        };
        c.in_handles.insert(1, Some(pt(20.0, 10.0)));
        c
    }

    fn close(a: Point, b: Point) -> bool {
        (a - b).hypot() < 1e-6
    }

    #[test]
    fn the_preview_runs_from_the_last_node_curved_by_its_handle() {
        let c = sample();
        let look = curve_look(&c, Some(pt(90.0, 20.0)), true, None);
        let preview = look.preview.expect("a preview segment");
        let segs: Vec<PathSeg> = preview.segments().collect();
        assert_eq!(segs.len(), 1);
        match segs[0] {
            PathSeg::Cubic(k) => {
                assert!(close(k.p0, pt(60.0, 0.0)), "{k:?}");
                assert!(close(k.p1, pt(70.0, 0.0)), "{k:?}");
                assert!(close(k.p3, pt(90.0, 20.0)), "{k:?}");
            }
            other => panic!("a curve: {other:?}"),
        }
        // A last node without a handle: a straight preview.
        let mut s = c.clone();
        s.nodes[2].1 = None;
        let look = curve_look(&s, Some(pt(90.0, 20.0)), true, None);
        let segs: Vec<PathSeg> = look.preview.expect("preview").segments().collect();
        assert!(matches!(segs[0], PathSeg::Line(_)), "{segs:?}");
        // Preview mode off, or a handle being pulled: none.
        assert!(curve_look(&c, Some(pt(90.0, 20.0)), false, None)
            .preview
            .is_none());
        let mut d = c.clone();
        d.dragging_handle = true;
        assert!(curve_look(&d, Some(pt(90.0, 20.0)), true, None)
            .preview
            .is_none());
        // The placed segments never reach the pointer.
        let end = look.placed.segments().last().expect("segments").eval(1.0);
        assert!(close(end, pt(60.0, 0.0)), "{end:?}");
    }

    #[test]
    fn placed_segments_take_the_outline_and_only_the_last_node_is_filled() {
        let c = sample();
        let red = Color32::from_rgb(200, 0, 0);
        let look = curve_look(&c, None, true, Some(red));
        assert_eq!(look.placed_color, red);
        assert_eq!(look.preview_color, crate::theme::Tokens::SELECTION);
        assert_eq!(look.placed.segments().count(), 2);
        // No outline for new objects: a black hairline.
        assert_eq!(
            curve_look(&c, None, true, None).placed_color,
            Color32::BLACK
        );
        let marks: Vec<NodeLook> = look.nodes.iter().map(|n| n.1).collect();
        assert_eq!(
            marks,
            vec![NodeLook::Placed, NodeLook::Placed, NodeLook::Current]
        );
        // The last node's handles: its outgoing one and the mirror.
        assert_eq!(look.handles.len(), 2);
        assert!(close(look.handles[0].1, pt(70.0, 0.0)));
        assert!(close(look.handles[1].1, pt(50.0, 0.0)));
    }

    #[test]
    fn the_closing_drag_previews_the_closing_segment() {
        let mut c = sample();
        c.closing = Some(pt(-10.0, -10.0));
        let look = curve_look(&c, Some(pt(-10.0, -10.0)), true, None);
        let segs: Vec<PathSeg> = look.preview.expect("closing").segments().collect();
        match segs[0] {
            PathSeg::Cubic(k) => {
                assert!(close(k.p0, pt(60.0, 0.0)));
                assert!(close(k.p1, pt(70.0, 0.0)));
                assert!(close(k.p2, pt(-10.0, -10.0)), "{k:?}");
                assert!(close(k.p3, pt(0.0, 0.0)));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(look.nodes[0].1, NodeLook::Current);
        assert!(look.handles.iter().any(|h| close(h.1, pt(-10.0, -10.0))));
    }

    #[test]
    fn a_node_added_on_a_segment_keeps_the_curve_and_can_be_deleted() {
        let c = sample();
        let before = c.path(None);
        let on = before.segments().next().expect("segment").eval(0.4);
        let (k, t) = c.segment_near(on, 0.01).expect("on the first segment");
        assert_eq!(k, 1);
        let mut d = c.clone();
        assert_eq!(d.insert_node(k, t), Some(1));
        assert_eq!(d.nodes.len(), 4);
        assert!(close(d.nodes[1].0, on), "{:?}", d.nodes[1]);
        // The shape is unchanged: points along it lie on the old curve.
        let after = d.path(None);
        for s in after.segments() {
            for i in 0..=10 {
                let q = s.eval(i as f64 / 10.0);
                let (_, _, dist) =
                    tracedraw_core::nodes::nearest_segment(&before, q).expect("near");
                assert!(dist < 1e-6, "{q:?} is {dist} off");
            }
        }
        // The cusp, now at index 2, still arrives along its handle's line.
        let h = d.in_handles.get(&2).copied().flatten().expect("a handle");
        let (u, v) = (h - pt(30.0, 0.0), pt(20.0, 10.0) - pt(30.0, 0.0));
        assert!((u.x * v.y - u.y * v.x).abs() < 1e-9, "{h:?}");
        // A straight segment splits straight.
        let mut s = CurveInProgress {
            nodes: vec![(pt(0.0, 0.0), None), (pt(10.0, 0.0), None)],
            smooth: true,
            ..Default::default()
        };
        assert_eq!(s.insert_node(1, 0.5), Some(1));
        assert!(close(s.nodes[1].0, pt(5.0, 0.0)));
        assert!(matches!(
            s.path(None).segments().next(),
            Some(PathSeg::Line(_))
        ));
        // Deleting the new node: the neighbours keep their handles.
        assert!(d.delete_node(1));
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.in_handles.get(&1).copied(), Some(Some(h)));
        assert!(!d.delete_node(9));
        assert_eq!(d.insert_node(0, 0.5), None);
    }

    #[test]
    fn the_first_node_is_kept_for_closing_with_auto_add_delete() {
        let mut app = App::headless();
        app.set_tool(Tool::Pen);
        app.view.zoom = 4.0;
        app.settings.curves.pen_auto_add_delete = true;
        app.curve = Some(sample());
        // Pick reach wider than the join reach: the gap between the two
        // used to delete the first node instead of closing the curve.
        app.settings.curves.join_px = 3.0;
        app.settings.nodes.pick_distance = 10.0;
        let reach = app.curve_end_reach_px();
        assert!(reach >= app.settings.nodes.pick_px());
        // 2.0 mm off at zoom 4 is 8 px: past the join reach, within pick.
        let by_first = pt(2.0, 0.0);
        assert_eq!(app.pen_auto_at(by_first), None);
        assert!((by_first - pt(0.0, 0.0)).hypot() * 4.0 <= reach);
        // Without auto add and delete the join reach alone counts.
        app.settings.curves.pen_auto_add_delete = false;
        assert_eq!(app.curve_end_reach_px(), app.settings.curves.join_px());
    }

    #[test]
    fn the_pen_deletes_and_adds_on_the_curve_being_drawn() {
        let mut app = App::headless();
        app.set_tool(Tool::Pen);
        app.view.zoom = 4.0;
        app.curve = Some(sample());
        // The middle node: delete; the last: a cusp (not here); the first
        // with three nodes: closes (not here).
        assert_eq!(app.pen_auto_at(pt(30.0, 0.0)), Some(PenAuto::Delete(1)));
        assert_eq!(app.pen_auto_at(pt(60.0, 0.0)), None);
        assert_eq!(app.pen_auto_at(pt(0.0, 0.0)), None);
        let on = app
            .curve
            .as_ref()
            .map(|c| c.path(None))
            .and_then(|p| p.segments().nth(1))
            .map(|s| s.eval(0.5))
            .expect("second segment");
        assert!(matches!(app.pen_auto_at(on), Some(PenAuto::Add(2, _))));
        // Far from the curve: nothing.
        assert_eq!(app.pen_auto_at(pt(30.0, 40.0)), None);
        // Off, or the Bezier tool: nothing.
        app.settings.curves.pen_auto_add_delete = false;
        assert_eq!(app.pen_auto_at(pt(30.0, 0.0)), None);
        app.settings.curves.pen_auto_add_delete = true;
        app.set_tool(Tool::Bezier);
        app.curve = Some(sample());
        assert_eq!(app.pen_auto_at(pt(30.0, 0.0)), None);
    }

    #[test]
    fn both_options_are_on_by_default() {
        let c = crate::prefs::CurvePrefs::default();
        assert!(c.pen_preview && c.pen_auto_add_delete);
        // Older preferences without them turn them on.
        let old: crate::prefs::CurvePrefs =
            serde_json::from_str(r#"{"smoothing": 30, "join_px": 4.0}"#).expect("prefs");
        assert!(old.pen_preview && old.pen_auto_add_delete);
        assert_eq!(old.smoothing, 30);
    }
}
