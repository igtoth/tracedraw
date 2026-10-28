//! The Variable Outline tool: width nodes along an object's outline.
//! A drag across the outline of the selected object adds a node there (or
//! takes the node under the pointer) and sets how far the outline reaches
//! to the pointer; with Shift (or Alt) only the side the pointer is on
//! changes. Ctrl+click adds a node to the picked ones (or takes it out),
//! Shift+click picks the nodes from the picked one to the clicked one; a
//! drag then changes every picked node. The property bar edits the picked
//! nodes' reach and removes nodes.

use crate::app::App;
use crate::ui::tips::RichTip;
use egui::{Color32, Modifiers, Painter, PointerButton, Response, Stroke as EStroke};
use tracedraw_core::geometry::{Affine, BezPath, Point};
use tracedraw_core::{Command, ShapeId, Stroke, WidthNode};

/// Screen distance (pixels) within which a press takes a node or the
/// outline.
const GRAB_PX: f64 = 7.0;

/// The tool's state: the node picked on the selected object and the drag
/// under way.
#[derive(Default, Clone, Debug)]
pub struct VariableOutlineState {
    /// The node picked last (the property bar shows its reach).
    pub node: Option<usize>,
    /// Other nodes picked with it (Ctrl+click, Shift+click).
    pub more: Vec<usize>,
    drag: Option<NodeDrag>,
}

#[derive(Clone, Debug)]
struct NodeDrag {
    shape: ShapeId,
    index: usize,
    /// Shift or Alt held at the start: only the side the pointer is on
    /// changes.
    one_side: bool,
    /// The first change of the drag has been recorded (later ones amend
    /// it, so the drag is one undo step).
    recorded: bool,
    /// What the drag does: slide the node along the path or change its
    /// reach; `None` until the first move says (along the path or
    /// across it) for a press on a node's centre (#479).
    slide: Option<bool>,
    /// Where the press was (page space).
    from: Point,
}

/// Sorted nodes, with the index the node at `keep` ends up at.
pub(crate) fn sorted(mut nodes: Vec<WidthNode>, keep: usize) -> (Vec<WidthNode>, usize) {
    let marked: Vec<(usize, WidthNode)> = nodes.drain(..).enumerate().collect();
    let mut marked = marked;
    marked.sort_by(|a, b| a.1.at.total_cmp(&b.1.at));
    let at = marked.iter().position(|(i, _)| *i == keep).unwrap_or(0);
    (marked.into_iter().map(|(_, n)| n).collect(), at)
}

impl App {
    /// The object the tool works on: the first selected one with a path.
    pub fn variable_outline_target(&self) -> Option<ShapeId> {
        self.selection.first().copied().filter(|id| {
            self.doc()
                .find_shape(*id)
                .map(|s| !s.page_path().elements().is_empty())
                .unwrap_or(false)
        })
    }

    /// The target's outline in page space, its stroke and the factor its
    /// widths are drawn at.
    pub fn variable_outline_geometry(&self, id: ShapeId) -> Option<(BezPath, Stroke, f64)> {
        let doc = self.doc();
        let shape = doc.find_shape(id)?;
        let parent = doc.parent_transform(id).unwrap_or(Affine::IDENTITY);
        let path = parent * shape.page_path();
        let stroke = shape.stroke.clone().unwrap_or_default();
        let scale = if stroke.scale_with_object {
            let c = (parent * shape.transform).as_coeffs();
            ((c[0] * c[3] - c[1] * c[2]).abs()).sqrt().max(1e-6)
        } else {
            1.0
        };
        Some((path, stroke, scale))
    }

    /// Replace the target's width nodes; the first change of a drag is a
    /// new undo step, later ones join it.
    fn set_width_nodes(&mut self, id: ShapeId, nodes: Vec<WidthNode>, amend: bool) {
        let Some((_, mut stroke, _)) = self.variable_outline_geometry(id) else {
            return;
        };
        stroke.widths = nodes;
        let cmd = Command::SetStroke {
            shapes: vec![id],
            stroke: Some(stroke),
        };
        if amend {
            let _ = self.engine.amend(&cmd);
        } else {
            self.run(cmd);
        }
    }

    pub fn variable_outline_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        let clicked = response.clicked_by(PointerButton::Primary);
        let started = response.drag_started_by(PointerButton::Primary);
        let ctrl = mods.ctrl || mods.command;
        if clicked && (mods.shift || ctrl) && self.variable_outline_pick(p, mods.shift) {
            // Picked (Shift: a run of nodes, Ctrl: one more or one less).
        } else if started {
            // Shift or Alt: one side; Ctrl: the node joins the picked ones.
            self.variable_outline_press_with(p, mods.shift || mods.alt, ctrl);
        } else if clicked {
            // A plain click picks the node alone.
            self.variable_outline_press(p, mods.alt);
            self.vo.more.clear();
        }
        if response.dragged_by(PointerButton::Primary) {
            // Shift or Alt held (or let go) during the drag: one side, or
            // both again, from then on (#492).
            if let Some(d) = &mut self.vo.drag {
                d.one_side = mods.shift || mods.alt;
            }
            self.variable_outline_drag(p);
        }
        if response.drag_stopped() || response.clicked_by(PointerButton::Primary) {
            self.vo.drag = None;
        }
    }

    /// A press: on a node, it is picked; on the outline, a node is added
    /// there with the reach the outline has; elsewhere, the object under
    /// the pointer is selected.
    pub fn variable_outline_press(&mut self, p: Point, one_side: bool) {
        self.variable_outline_press_with(p, one_side, false);
    }

    /// The width node under `p` on the target, if any.
    fn width_node_at(&self, p: Point) -> Option<usize> {
        let grab = GRAB_PX / self.view.zoom.max(1e-6) as f64;
        let id = self.variable_outline_target()?;
        let (path, stroke, _) = self.variable_outline_geometry(id)?;
        stroke.widths.iter().enumerate().find_map(|(i, n)| {
            tracedraw_core::shaping::point_on_path(&path, n.at)
                .filter(|(q, _)| (*q - p).hypot() <= grab)
                .map(|_| i)
        })
    }

    /// Every picked node, the last picked first.
    pub fn picked_width_nodes(&self) -> Vec<usize> {
        let mut out: Vec<usize> = self.vo.node.into_iter().collect();
        for i in &self.vo.more {
            if !out.contains(i) {
                out.push(*i);
            }
        }
        out
    }

    /// A click with Shift or Ctrl on a node: Shift picks the nodes from
    /// the one picked to this one, Ctrl adds this one to the picked ones
    /// or takes it out. False when no node is under `p`.
    pub fn variable_outline_pick(&mut self, p: Point, range: bool) -> bool {
        let Some(i) = self.width_node_at(p) else {
            return false;
        };
        self.vo.drag = None;
        match (range, self.vo.node) {
            (true, Some(from)) => {
                let (a, b) = (from.min(i), from.max(i));
                self.vo.more = (a..=b).filter(|k| *k != i).collect();
                self.vo.node = Some(i);
            }
            (false, Some(_)) if self.picked_width_nodes().contains(&i) => {
                let rest: Vec<usize> = self
                    .picked_width_nodes()
                    .into_iter()
                    .filter(|k| *k != i)
                    .collect();
                self.vo.node = rest.first().copied();
                self.vo.more = rest.into_iter().skip(1).collect();
            }
            _ => {
                let all = self.picked_width_nodes();
                self.vo.more = all;
                self.vo.node = Some(i);
            }
        }
        true
    }

    /// A press (see [`Self::variable_outline_press`]); with `extend` the
    /// node pressed joins the picked ones, else a node that is not picked
    /// becomes the only one (a picked one keeps the others, so a drag
    /// changes them all).
    pub fn variable_outline_press_with(&mut self, p: Point, one_side: bool, extend: bool) {
        let grab = GRAB_PX / self.view.zoom.max(1e-6) as f64;
        if let Some(id) = self.variable_outline_target() {
            if let Some((path, stroke, scale)) = self.variable_outline_geometry(id) {
                // A node under the pointer.
                let hit = stroke.widths.iter().enumerate().find_map(|(i, n)| {
                    tracedraw_core::shaping::point_on_path(&path, n.at)
                        .filter(|(q, _)| (*q - p).hypot() <= grab)
                        .map(|_| i)
                });
                if let Some(i) = hit {
                    let picked = self.picked_width_nodes();
                    if extend || picked.contains(&i) {
                        self.vo.more = picked.into_iter().filter(|k| *k != i).collect();
                    } else {
                        self.vo.more.clear();
                    }
                    self.vo.node = Some(i);
                    self.vo.drag = Some(NodeDrag {
                        shape: id,
                        index: i,
                        one_side,
                        recorded: false,
                        slide: None,
                        from: p,
                    });
                    return;
                }
                // A node's reach handle (the dots at its ends): the drag
                // changes its reach (#479).
                let handle = stroke.widths.iter().enumerate().find_map(|(i, n)| {
                    let (q, nn) = tracedraw_core::shaping::point_on_path(&path, n.at)?;
                    [n.left * scale, -n.right * scale]
                        .into_iter()
                        .map(|d| Point::new(q.x + nn[0] * d, q.y + nn[1] * d))
                        .any(|e| (e - p).hypot() <= grab)
                        .then_some(i)
                });
                if let Some(i) = handle {
                    let picked = self.picked_width_nodes();
                    self.vo.more = if extend || picked.contains(&i) {
                        picked.into_iter().filter(|k| *k != i).collect()
                    } else {
                        Vec::new()
                    };
                    self.vo.node = Some(i);
                    self.vo.drag = Some(NodeDrag {
                        shape: id,
                        index: i,
                        one_side,
                        recorded: false,
                        slide: Some(false),
                        from: p,
                    });
                    return;
                }
                // The outline under the pointer.
                if let Some((t, d, _)) = tracedraw_core::shaping::locate_on_path(&path, p) {
                    let (l, r) = tracedraw_core::style::width_at(stroke.width, &stroke.widths, t);
                    if d <= (l.max(r) * scale) + grab {
                        let mut nodes = stroke.widths.clone();
                        nodes.push(WidthNode {
                            at: t,
                            left: l,
                            right: r,
                        });
                        let last = nodes.len() - 1;
                        let (nodes, index) = sorted(nodes, last);
                        self.set_width_nodes(id, nodes, false);
                        // The nodes after the new one move up one place.
                        let shifted: Vec<usize> = self
                            .picked_width_nodes()
                            .into_iter()
                            .map(|k| if k >= index { k + 1 } else { k })
                            .collect();
                        self.vo.more = if extend { shifted } else { Vec::new() };
                        self.vo.node = Some(index);
                        self.vo.drag = Some(NodeDrag {
                            shape: id,
                            index,
                            one_side,
                            recorded: true,
                            slide: Some(false),
                            from: p,
                        });
                        return;
                    }
                }
            }
        }
        self.vo.node = None;
        self.vo.more.clear();
        self.vo.drag = None;
        match self.hit_test(p) {
            Some(id) => self.select(vec![id]),
            None => self.selection.clear(),
        }
    }

    /// A drag: the node pressed reaches to the pointer (one side or both),
    /// and the other picked nodes take the same reach on that side.
    pub fn variable_outline_drag(&mut self, p: Point) {
        let Some(drag) = self.vo.drag.clone() else {
            return;
        };
        let Some((path, stroke, scale)) = self.variable_outline_geometry(drag.shape) else {
            return;
        };
        let Some(node) = stroke.widths.get(drag.index).copied() else {
            return;
        };
        let Some((q, n)) = tracedraw_core::shaping::point_on_path(&path, node.at) else {
            return;
        };
        // A press on the node's centre: the first move along the path
        // slides the node, across it changes the reach (#479).
        let slide = match drag.slide {
            Some(s) => s,
            None => {
                let d = p - drag.from;
                if d.hypot() < 1.0 / self.view.zoom.max(1e-6) as f64 {
                    return;
                }
                let across = (d.x * n[0] + d.y * n[1]).abs();
                let along = (d.x * n[1] - d.y * n[0]).abs();
                let s = along > across;
                if let Some(dr) = self.vo.drag.as_mut() {
                    dr.slide = Some(s);
                }
                s
            }
        };
        if slide {
            let Some((t, _, _)) = tracedraw_core::shaping::locate_on_path(&path, p) else {
                return;
            };
            // Between its neighbours, so the nodes keep their order.
            let mut nodes = stroke.widths.clone();
            let lo = drag
                .index
                .checked_sub(1)
                .and_then(|k| nodes.get(k))
                .map_or(0.0, |m| m.at + 1e-4);
            let hi = nodes.get(drag.index + 1).map_or(1.0, |m| m.at - 1e-4);
            if let Some(m) = nodes.get_mut(drag.index) {
                m.at = t.clamp(lo.min(hi), hi.max(lo));
            }
            self.set_width_nodes(drag.shape, nodes, drag.recorded);
            if let Some(d) = self.vo.drag.as_mut() {
                d.recorded = true;
            }
            return;
        }
        // Distance across the outline, measured along the normal.
        let across = (p.x - q.x) * n[0] + (p.y - q.y) * n[1];
        let reach = across.abs() / scale;
        let mut nodes = stroke.widths.clone();
        let mut targets = self.picked_width_nodes();
        if !targets.contains(&drag.index) {
            targets.push(drag.index);
        }
        for i in targets {
            if let Some(m) = nodes.get_mut(i) {
                if drag.one_side {
                    if across >= 0.0 {
                        m.left = reach;
                    } else {
                        m.right = reach;
                    }
                } else {
                    m.left = reach;
                    m.right = reach;
                }
            }
        }
        self.set_width_nodes(drag.shape, nodes, drag.recorded);
        if let Some(d) = self.vo.drag.as_mut() {
            d.recorded = true;
        }
    }

    /// The picked node's left and right reach (mm).
    pub fn variable_outline_node(&self) -> Option<WidthNode> {
        let id = self.variable_outline_target()?;
        let (_, stroke, _) = self.variable_outline_geometry(id)?;
        stroke.widths.get(self.vo.node?).copied()
    }

    /// Set the picked nodes' reach (property bar).
    pub fn set_variable_outline_node(&mut self, left: f64, right: f64) {
        let Some(id) = self.variable_outline_target() else {
            return;
        };
        let Some((_, stroke, _)) = self.variable_outline_geometry(id) else {
            return;
        };
        let mut nodes = stroke.widths.clone();
        let mut changed = false;
        for i in self.picked_width_nodes() {
            if let Some(n) = nodes.get_mut(i) {
                n.left = left.max(0.0);
                n.right = right.max(0.0);
                changed = true;
            }
        }
        if changed {
            self.set_width_nodes(id, nodes, false);
        }
    }

    /// Remove the picked nodes (Delete or the property bar), one step.
    pub fn delete_variable_outline_node(&mut self) -> bool {
        let Some(id) = self.variable_outline_target() else {
            return false;
        };
        let Some((_, stroke, _)) = self.variable_outline_geometry(id) else {
            return false;
        };
        let picked = self.picked_width_nodes();
        let nodes: Vec<WidthNode> = stroke
            .widths
            .iter()
            .enumerate()
            .filter(|(i, _)| !picked.contains(i))
            .map(|(_, n)| *n)
            .collect();
        if nodes.len() == stroke.widths.len() {
            return false;
        }
        self.set_width_nodes(id, nodes, false);
        self.vo.node = None;
        self.vo.more.clear();
        true
    }

    /// Remove every width node of the selected objects: a plain outline
    /// again.
    pub fn clear_variable_outline(&mut self) {
        let shapes: Vec<_> = self
            .selected_shapes()
            .into_iter()
            .filter(|s| s.stroke.as_ref().is_some_and(|st| !st.widths.is_empty()))
            .collect();
        let cmds: Vec<Command> = shapes
            .iter()
            .filter_map(|s| {
                let mut st = s.stroke.clone()?;
                st.widths.clear();
                Some(Command::SetStroke {
                    shapes: vec![s.id],
                    stroke: Some(st),
                })
            })
            .collect();
        for cmd in cmds {
            self.run(cmd);
        }
        self.vo.node = None;
        self.vo.more.clear();
    }
}

/// The target's width nodes: a dot on the path and a line across the
/// reach on each side; the picked node filled.
pub fn draw_width_nodes(app: &App, painter: &Painter) {
    if app.tool != crate::tools::Tool::VariableOutline {
        return;
    }
    let Some(id) = app.variable_outline_target() else {
        return;
    };
    let Some((path, stroke, scale)) = app.variable_outline_geometry(id) else {
        return;
    };
    let blue = crate::theme::Tokens::SELECTION;
    // The path itself, thin, so the line the nodes sit on shows.
    for (pts, closed) in crate::canvas::flatten(&path, &app.view) {
        let line = EStroke::new(1.0, blue.gamma_multiply(0.6));
        if closed {
            painter.add(egui::epaint::PathShape::closed_line(pts, line));
        } else {
            painter.add(egui::epaint::PathShape::line(pts, line));
        }
    }
    for (i, node) in stroke.widths.iter().enumerate() {
        let Some((q, n)) = tracedraw_core::shaping::point_on_path(&path, node.at) else {
            continue;
        };
        let at = |d: f64| {
            app.view
                .to_screen(Point::new(q.x + n[0] * d, q.y + n[1] * d))
        };
        let c = app.view.to_screen(q);
        let a = at(node.left * scale);
        let b = at(-node.right * scale);
        painter.line_segment([a, b], EStroke::new(1.0, blue));
        for e in [a, b] {
            painter.circle_filled(e, 2.5, blue);
        }
        let picked = app.vo.node == Some(i) || app.vo.more.contains(&i);
        painter.circle_filled(c, 4.0, if picked { blue } else { Color32::WHITE });
        painter.circle_stroke(c, 4.0, EStroke::new(1.2, blue));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Shape as _;
    use tracedraw_core::{Shape, ShapeKind};

    fn line_app() -> (App, ShapeId) {
        let mut app = App::headless();
        let layer = app.active_layer().expect("a layer");
        let id = app.engine.new_shape_id();
        let mut path = BezPath::new();
        path.move_to(Point::new(10.0, 50.0));
        path.line_to(Point::new(110.0, 50.0));
        let mut s = Shape::new(
            id,
            ShapeKind::Path {
                path,
                closed: false,
            },
        );
        s.stroke = Some(Stroke::new(tracedraw_core::Color::BLACK, 2.0));
        app.run(Command::AddShape { layer, shape: s });
        app.select(vec![id]);
        app.tool = crate::tools::Tool::VariableOutline;
        (app, id)
    }

    fn widths(app: &App, id: ShapeId) -> Vec<WidthNode> {
        app.doc()
            .find_shape(id)
            .and_then(|s| s.stroke.clone())
            .map(|s| s.widths)
            .unwrap_or_default()
    }

    #[test]
    fn a_drag_across_the_outline_adds_a_node_and_sets_its_reach_in_one_step() {
        let (mut app, id) = line_app();
        let undo_before = app.engine.undo_label().map(str::to_string);
        // Press on the outline halfway, drag 5 mm up.
        app.variable_outline_press(Point::new(60.0, 50.5), false);
        assert_eq!(widths(&app, id).len(), 1);
        app.variable_outline_drag(Point::new(60.0, 53.0));
        app.variable_outline_drag(Point::new(60.0, 55.0));
        let w = widths(&app, id);
        assert!((w[0].at - 0.5).abs() < 0.01, "{w:?}");
        assert!((w[0].left - 5.0).abs() < 1e-6 && (w[0].right - 5.0).abs() < 1e-6);
        // The outline is 10 mm thick there now.
        let shape = app.doc().find_shape(id).cloned();
        let band = shape
            .and_then(|s| {
                let st = s.stroke.clone()?;
                tracedraw_core::shaping::outline_band(&s.page_path(), &st, 1.0)
            })
            .map(|b| b.bounding_box())
            .unwrap_or_default();
        assert!((band.height() - 10.0).abs() < 1e-3, "{band:?}");
        // One undo takes the whole drag back.
        app.undo();
        assert!(widths(&app, id).is_empty());
        assert_eq!(app.engine.undo_label().map(str::to_string), undo_before);
    }

    #[test]
    fn alt_changes_one_side_and_the_bar_edits_and_removes_nodes() {
        let (mut app, id) = line_app();
        app.variable_outline_press(Point::new(35.0, 50.0), true);
        app.variable_outline_drag(Point::new(35.0, 46.0));
        let w = widths(&app, id);
        assert!((w[0].right - 4.0).abs() < 1e-6 && (w[0].left - 1.0).abs() < 1e-6);
        // A press on the node picks it again rather than adding one.
        app.vo.node = None;
        app.variable_outline_press(Point::new(35.0, 50.0), false);
        assert_eq!(widths(&app, id).len(), 1);
        assert_eq!(app.vo.node, Some(0));
        app.set_variable_outline_node(3.0, 0.5);
        assert_eq!(
            app.variable_outline_node().map(|n| (n.left, n.right)),
            Some((3.0, 0.5))
        );
        // A second node further along keeps the nodes in path order.
        app.vo.drag = None;
        app.variable_outline_press(Point::new(20.0, 50.0), false);
        let w = widths(&app, id);
        assert_eq!(w.len(), 2);
        assert!(w[0].at < w[1].at);
        assert_eq!(app.vo.node, Some(0));
        assert!(app.delete_variable_outline_node());
        assert_eq!(widths(&app, id).len(), 1);
        app.clear_variable_outline();
        assert!(widths(&app, id).is_empty());
        // Away from the outline: the object under the pointer, or nothing.
        app.variable_outline_press(Point::new(60.0, 150.0), false);
        assert!(app.selection.is_empty());
    }

    /// Shift+click picks a run of nodes, Ctrl+click one more or one less;
    /// a drag changes every picked node, both sides, or one side when it
    /// starts with Shift (#374).
    #[test]
    fn shift_and_ctrl_pick_nodes_and_shift_drags_one_side() {
        let (mut app, id) = line_app();
        for x in [20.0, 60.0, 100.0] {
            app.variable_outline_press(Point::new(x, 50.0), false);
            app.vo.drag = None;
        }
        assert_eq!(widths(&app, id).len(), 3);
        // The first picked, then Shift+click on the last: all three.
        assert!(app.variable_outline_pick(Point::new(20.0, 50.0), false));
        app.vo.more.clear();
        app.vo.node = Some(0);
        assert!(app.variable_outline_pick(Point::new(100.0, 50.0), true));
        let mut all = app.picked_width_nodes();
        all.sort();
        assert_eq!(all, vec![0, 1, 2]);
        // A drag on the middle one: all three reach 3 mm both ways.
        app.variable_outline_press_with(Point::new(60.0, 50.0), false, false);
        app.variable_outline_drag(Point::new(60.0, 53.0));
        app.vo.drag = None;
        let w = widths(&app, id);
        assert!(
            w.iter()
                .all(|n| (n.left - 3.0).abs() < 1e-6 && (n.right - 3.0).abs() < 1e-6),
            "{w:?}"
        );
        // Ctrl+click takes the middle one out.
        assert!(app.variable_outline_pick(Point::new(60.0, 50.0), false));
        let mut left = app.picked_width_nodes();
        left.sort();
        assert_eq!(left, vec![0, 2]);
        // A drag that starts with Shift: one side only, on the picked ones.
        app.variable_outline_press_with(Point::new(20.0, 50.0), true, false);
        app.variable_outline_drag(Point::new(20.0, 45.0));
        app.vo.drag = None;
        let w = widths(&app, id);
        assert!(
            (w[0].right - 5.0).abs() < 1e-6 && (w[0].left - 3.0).abs() < 1e-6,
            "{w:?}"
        );
        assert!(
            (w[2].right - 5.0).abs() < 1e-6 && (w[2].left - 3.0).abs() < 1e-6,
            "{w:?}"
        );
        assert!((w[1].right - 3.0).abs() < 1e-6, "{w:?}");
        // Delete removes the picked ones in one step.
        assert!(app.delete_variable_outline_node());
        assert_eq!(widths(&app, id).len(), 1);
        app.undo();
        assert_eq!(widths(&app, id).len(), 3);
    }

    /// A node pressed on its centre slides along the path when the drag
    /// goes along it (kept between its neighbours) and changes its reach
    /// when it goes across; a press on a reach handle changes the reach.
    /// All of it one undo step per drag (#479).
    #[test]
    fn width_nodes_slide_along_the_path_and_their_handles_change_the_reach() {
        let (mut app, id) = line_app();
        app.view.zoom = 4.0;
        // A node at 60, 4 mm each way.
        app.variable_outline_press(Point::new(60.0, 50.0), false);
        app.variable_outline_drag(Point::new(60.0, 54.0));
        app.vo.drag = None;
        let w = widths(&app, id);
        assert_eq!(w.len(), 1);
        assert!((w[0].at - 0.5).abs() < 1e-9 && (w[0].left - 4.0).abs() < 1e-6);
        // Along the path: it slides to 80, its reach as it was.
        let steps = app.engine.history_labels().0.len();
        app.variable_outline_press(Point::new(60.0, 50.0), false);
        app.variable_outline_drag(Point::new(65.0, 50.2));
        app.variable_outline_drag(Point::new(80.0, 50.5));
        app.vo.drag = None;
        let w = widths(&app, id);
        assert!((w[0].at - 0.7).abs() < 1e-6, "{w:?}");
        assert!((w[0].left - 4.0).abs() < 1e-6 && (w[0].right - 4.0).abs() < 1e-6);
        assert_eq!(app.engine.history_labels().0.len(), steps + 1, "one step");
        // Its handle (4 mm out): the reach follows the pointer.
        app.variable_outline_press(Point::new(80.0, 54.0), false);
        app.variable_outline_drag(Point::new(80.0, 57.0));
        app.vo.drag = None;
        let w = widths(&app, id);
        assert!((w[0].at - 0.7).abs() < 1e-6);
        assert!(
            (w[0].left - 7.0).abs() < 1e-6 && (w[0].right - 7.0).abs() < 1e-6,
            "{w:?}"
        );
        // A second node at 30 cannot slide past the first.
        app.variable_outline_press(Point::new(30.0, 50.0), false);
        app.vo.drag = None;
        app.variable_outline_press(Point::new(30.0, 50.0), false);
        app.variable_outline_drag(Point::new(40.0, 50.0));
        app.variable_outline_drag(Point::new(100.0, 50.0));
        app.vo.drag = None;
        let w = widths(&app, id);
        assert!(w[0].at < w[1].at && w[1].at > 0.69, "{w:?}");
    }
}

/// The Variable Outline tool's property bar: the picked node's width on
/// each side, Delete Node and Clear.
pub fn variable_outline_bar(app: &mut App, ui: &mut egui::Ui) {
    use crate::i18n::tr;
    use crate::ui::field::NumField;
    let node = app.variable_outline_node();
    let has_nodes = app
        .selected_shapes()
        .iter()
        .any(|s| s.stroke.as_ref().is_some_and(|st| !st.widths.is_empty()));
    let field = |ui: &mut egui::Ui, key: &str, mm: f64| -> Option<f64> {
        ui.label(tr(key));
        let mut v = app.units.world_from_mm(mm);
        let r = ui.add_enabled(
            node.is_some(),
            NumField::new(&mut v)
                .range(0.0..=app.units.world_from_mm(500.0))
                .fixed_decimals(app.settings.precision.min(10) as usize)
                .unit_mm(app.units.world_to_mm(1.0))
                .suffix(format!(" {}", app.units.short()))
                .width(76.0),
        );
        r.changed().then(|| app.units.world_to_mm(v))
    };
    let (l, r) = node.map(|n| (n.left, n.right)).unwrap_or((0.0, 0.0));
    let new_l = field(ui, "vo.left", l);
    let new_r = field(ui, "vo.right", r);
    if new_l.is_some() || new_r.is_some() {
        app.set_variable_outline_node(new_l.unwrap_or(l), new_r.unwrap_or(r));
    }
    crate::ui::propbar::sep(ui);
    if ui
        .add_enabled(node.is_some(), egui::Button::new(tr("vo.delete_node")))
        .clicked()
    {
        app.delete_variable_outline_node();
    }
    if ui
        .add_enabled(has_nodes, egui::Button::new(tr("vo.clear")))
        .rich_tip(tr("vo.clear_hint"))
        .clicked()
    {
        app.clear_variable_outline();
    }
}
