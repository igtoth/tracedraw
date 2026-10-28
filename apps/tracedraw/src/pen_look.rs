//! The Pen and Bezier tools while a curve is being drawn: what the canvas
//! shows (the segments placed in the outline the curve will get, the
//! Pen's dashed preview segment to the pointer, the nodes and the last
//! node's handles). Pure functions, so tests can check them.

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
    /// while the first node is dragged; dashed in `preview_color`.
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
        assert!((by_first - pt(0.0, 0.0)).hypot() * 4.0 <= reach);
        // Without auto add and delete the join reach alone counts.
        app.settings.curves.pen_auto_add_delete = false;
        assert_eq!(app.curve_end_reach_px(), app.settings.curves.join_px());
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
