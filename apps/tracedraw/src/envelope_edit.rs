//! Editing envelopes. The Envelope tool gives the object it clicks an
//! envelope and drags its nodes; the Shape tool drags them too. A node
//! moves the way the chosen mode says (straight line, single arc, double
//! arc or unconstrained); in the Unconstrained mode the selected node's
//! handles show and move on their own. Double-clicking the frame adds a
//! node, double-clicking a node that is not a corner removes it. Presets,
//! the mapping, Keep Lines, Add New, Clear and Copy act on the selection.

use crate::app::App;
use crate::i18n::tr;
use egui::{PointerButton, Response};
use tracedraw_core::{
    document::Shape,
    envelope::{Frame, Mapping, Mode},
    geometry::{Affine, Point, Rect, Shape as _, Vec2},
    live::{evaluate, Effect},
    Command, ShapeId,
};

/// A part of an envelope's frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Node(usize),
    /// Handle `end` (0 at the segment's start, 1 at its end) of segment
    /// `seg`.
    Handle {
        seg: usize,
        end: usize,
    },
}

/// A part being dragged: the effects as they were when the drag began,
/// so every step starts again from them.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvelopeDrag {
    pub shape: ShapeId,
    pub part: Part,
    pub start: Point,
    pub original: Vec<Effect>,
    /// A step of this drag is on the undo list.
    pub stepped: bool,
}

/// The envelope presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Arch,
    Bulge,
    Squeeze,
    Flag,
    Perspective,
    Circle,
}

impl Preset {
    pub const ALL: [Preset; 6] = [
        Preset::Arch,
        Preset::Bulge,
        Preset::Squeeze,
        Preset::Flag,
        Preset::Perspective,
        Preset::Circle,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Preset::Arch => "docker.preset_arch",
            Preset::Bulge => "docker.preset_bulge",
            Preset::Squeeze => "docker.preset_squeeze",
            Preset::Flag => "docker.preset_flag",
            Preset::Perspective => "docker.preset_perspective",
            Preset::Circle => "docker.preset_circle",
        }
    }

    /// The preset's frame around `b`.
    pub fn frame(self, b: Rect) -> Frame {
        let mut f = Frame::around(b);
        let (w, h) = (b.width(), b.height());
        // Nodes: 0 BL, 1 BM, 2 BR, 3 RM, 4 TR, 5 TM, 6 TL, 7 LM.
        let moves: Vec<(usize, Vec2)> = match self {
            Preset::Arch => vec![(5, Vec2::new(0.0, h * 0.3))],
            Preset::Bulge => vec![
                (1, Vec2::new(0.0, -h * 0.2)),
                (5, Vec2::new(0.0, h * 0.2)),
                (3, Vec2::new(w * 0.2, 0.0)),
                (7, Vec2::new(-w * 0.2, 0.0)),
            ],
            Preset::Squeeze => vec![(1, Vec2::new(0.0, h * 0.2)), (5, Vec2::new(0.0, -h * 0.2))],
            Preset::Flag => vec![
                (1, Vec2::new(0.0, -h * 0.2)),
                (5, Vec2::new(0.0, -h * 0.2)),
                (0, Vec2::new(0.0, h * 0.1)),
                (6, Vec2::new(0.0, h * 0.1)),
            ],
            Preset::Perspective => {
                vec![(4, Vec2::new(-w * 0.2, 0.0)), (6, Vec2::new(w * 0.2, 0.0))]
            }
            Preset::Circle => {
                // (sqrt 2 - 1) / 2 out: the middles sit on the circle.
                let k = 0.4142 * 0.5;
                vec![
                    (1, Vec2::new(0.0, -h * k)),
                    (5, Vec2::new(0.0, h * k)),
                    (3, Vec2::new(w * k, 0.0)),
                    (7, Vec2::new(-w * k, 0.0)),
                ]
            }
        };
        let mode = if self == Preset::Perspective {
            Mode::StraightLine
        } else {
            Mode::SingleArc
        };
        for (i, d) in moves {
            f.move_node(i, d, mode);
        }
        for s in 0..4 {
            f.reshape_side(s, mode);
        }
        f
    }
}

/// The last envelope in a list of effects: its place and its frame.
pub fn last_envelope(effects: &[Effect]) -> Option<(usize, Frame)> {
    effects
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, e)| e.envelope_frame().map(|f| (i, f)))
}

/// The object's own bounding box where the effect at `at` takes it: after
/// the effects before it.
pub fn box_at(shape: &Shape, at: usize) -> Rect {
    let mut s = shape.clone();
    s.effects.truncate(at);
    evaluate(&s).main.local_path().bounding_box()
}

/// The effect at `at` with its frame replaced (an envelope keeps its Keep
/// Lines and mapping).
fn with_frame(effects: &[Effect], at: usize, frame: &Frame) -> Vec<Effect> {
    let mut out = effects.to_vec();
    if let Some(e) = out.get_mut(at) {
        if let Effect::Envelope {
            keep_lines,
            mapping,
            ..
        } = e
        {
            *e = Effect::from_envelope(frame, *keep_lines, *mapping);
        }
    }
    out
}

/// The linear part of `m` applied to `d`.
fn local_delta(m: Affine, d: Vec2) -> Vec2 {
    let inv = m.inverse();
    (inv * (Point::ZERO + d)) - (inv * Point::ZERO)
}

impl App {
    /// The last envelope of object `id`.
    pub fn envelope_of(&self, id: ShapeId) -> Option<(usize, Frame)> {
        let s = self.doc().find_shape(id)?;
        last_envelope(&s.effects)
    }

    /// The selected objects with an envelope, as on the page.
    fn enveloped_selection(&self) -> Vec<(Shape, Frame)> {
        self.selected_shapes()
            .into_iter()
            .filter_map(|s| last_envelope(&s.effects).map(|(_, f)| (s, f)))
            .collect()
    }

    /// The handles shown for object `id`: those of its selected node, in
    /// the Unconstrained mode. (segment, end, handle, its node), in local
    /// space.
    pub fn envelope_handles(
        &self,
        id: ShapeId,
        frame: &Frame,
    ) -> Vec<(usize, usize, Point, Point)> {
        if self.envelope_mode != Mode::Unconstrained {
            return Vec::new();
        }
        let Some((sid, i)) = self.selected_effect_node else {
            return Vec::new();
        };
        let n = frame.len();
        if sid != id || i >= n {
            return Vec::new();
        }
        let prev = (i + n - 1) % n;
        let node = frame.nodes[i];
        let mut out = Vec::new();
        if let Some(h) = frame.handles.get(i) {
            out.push((i, 0, h[0], node));
        }
        if let Some(h) = frame.handles.get(prev) {
            out.push((prev, 1, h[1], node));
        }
        out
    }

    /// The envelope part under `p` (page space): handles first, then nodes.
    pub fn envelope_part_at(&self, p: Point) -> Option<(ShapeId, Part)> {
        let tol = 5.0 / self.view.zoom as f64;
        let shapes = self.enveloped_selection();
        for (s, f) in &shapes {
            for (seg, end, h, _) in self.envelope_handles(s.id, f) {
                if (s.transform * h - p).hypot() <= tol {
                    return Some((s.id, Part::Handle { seg, end }));
                }
            }
        }
        for (s, f) in &shapes {
            for (i, n) in f.nodes.iter().enumerate() {
                if (s.transform * *n - p).hypot() <= tol {
                    return Some((s.id, Part::Node(i)));
                }
            }
        }
        None
    }

    /// The frame segment under `p`: (object, segment, parameter).
    pub fn envelope_segment_at(&self, p: Point) -> Option<(ShapeId, usize, f64)> {
        let tol = 4.0 / self.view.zoom as f64;
        for (s, f) in self.enveloped_selection() {
            // Measure on the page: the frame moved there.
            let page = f.transformed(s.transform);
            if let Some((seg, t, d)) = page.nearest_segment(p) {
                if d <= tol {
                    return Some((s.id, seg, t));
                }
            }
        }
        None
    }

    /// Put `frame` in place of the envelope at `at` of object `id`, one
    /// undo step.
    fn set_envelope_frame(&mut self, id: ShapeId, at: usize, frame: &Frame, label: &'static str) {
        let Some(s) = self.doc().find_shape(id) else {
            return;
        };
        let effects = with_frame(&s.effects, at, frame);
        if let Err(e) = self
            .engine
            .run_with_label(&Command::SetEffects { shape: id, effects }, label)
        {
            self.status = e.to_string();
        }
    }

    /// Double-click on the frame: a node there; on a node that is not a
    /// corner: the node goes. True when it did either.
    pub fn envelope_double_click(&mut self, p: Point) -> bool {
        if let Some((id, Part::Node(i))) = self.envelope_part_at(p) {
            return self.remove_envelope_node(id, i);
        }
        if let Some((id, seg, t)) = self.envelope_segment_at(p) {
            return self.add_envelope_node(id, seg, t);
        }
        false
    }

    /// Add a node to segment `seg` of object `id`'s envelope at `t`; it
    /// becomes the selected node.
    pub fn add_envelope_node(&mut self, id: ShapeId, seg: usize, t: f64) -> bool {
        let Some((at, mut f)) = self.envelope_of(id) else {
            return false;
        };
        let Some(i) = f.add_node(seg, t) else {
            return false;
        };
        self.set_envelope_frame(id, at, &f, "Add Envelope Node");
        self.selected_effect_node = Some((id, i));
        true
    }

    /// Remove node `i` (not a corner) of object `id`'s envelope.
    pub fn remove_envelope_node(&mut self, id: ShapeId, i: usize) -> bool {
        let Some((at, mut f)) = self.envelope_of(id) else {
            return false;
        };
        if !f.remove_node(i) {
            self.status = tr("status.envelope_corner_stays");
            return false;
        }
        self.set_envelope_frame(id, at, &f, "Delete Envelope Node");
        self.selected_effect_node = None;
        true
    }

    /// The selected envelope node, if it is one of a selected object.
    pub fn selected_envelope_node(&self) -> Option<(ShapeId, usize)> {
        let (id, i) = self.selected_effect_node?;
        let (_, f) = self.envelope_of(id)?;
        (self.selection.contains(&id) && i < f.len()).then_some((id, i))
    }

    /// The property bar's Add Node: a node halfway along the segment after
    /// the selected node.
    pub fn add_node_after_selected(&mut self) {
        if let Some((id, i)) = self.selected_envelope_node() {
            self.add_envelope_node(id, i, 0.5);
        }
    }

    /// The property bar's Delete Node: the selected node goes.
    pub fn delete_selected_envelope_node(&mut self) {
        if let Some((id, i)) = self.selected_envelope_node() {
            self.remove_envelope_node(id, i);
        }
    }

    pub(crate) fn begin_envelope_drag(&mut self, id: ShapeId, part: Part, p: Point) {
        let Some(s) = self.doc().find_shape(id) else {
            return;
        };
        self.envelope_drag = Some(EnvelopeDrag {
            shape: id,
            part,
            start: p,
            original: s.effects.clone(),
            stepped: false,
        });
        if let Part::Node(i) = part {
            self.selected_effect_node = Some((id, i));
        }
    }

    /// A drag step: the part moved from where the drag began to `p`, the
    /// whole drag one undo step.
    pub(crate) fn envelope_drag_to(&mut self, p: Point) {
        let Some(drag) = self.envelope_drag.clone() else {
            return;
        };
        let Some(page) = self.doc().shape_on_page(drag.shape) else {
            return;
        };
        let Some((at, mut f)) = last_envelope(&drag.original) else {
            return;
        };
        let d = local_delta(page.transform, p - drag.start);
        match drag.part {
            Part::Node(i) => f.move_node(i, d, self.envelope_mode),
            Part::Handle { seg, end } => f.move_handle(seg, end, d),
        }
        let effects = with_frame(&drag.original, at, &f);
        if drag.stepped && self.engine.undo_label() == Some("Edit Envelope") {
            let _ = self.engine.undo();
        }
        if self
            .engine
            .run_with_label(
                &Command::SetEffects {
                    shape: drag.shape,
                    effects,
                },
                "Edit Envelope",
            )
            .is_ok()
        {
            if let Some(d) = self.envelope_drag.as_mut() {
                d.stepped = true;
            }
        }
    }

    /// Clicks, double-clicks and drags on the selected objects' envelopes,
    /// for the Envelope and the Shape tools. True when the input was
    /// theirs.
    pub fn envelope_input(&mut self, response: &Response, p: Point) -> bool {
        if response.double_clicked() && self.envelope_double_click(p) {
            return true;
        }
        if response.drag_started_by(PointerButton::Primary) {
            if let Some((id, part)) = self.envelope_part_at(p) {
                self.begin_envelope_drag(id, part, p);
                return true;
            }
        }
        if self.envelope_drag.is_some() {
            if response.dragged_by(PointerButton::Primary) {
                self.envelope_drag_to(p);
            }
            if response.drag_stopped() {
                self.envelope_drag = None;
            }
            return true;
        }
        if response.clicked_by(PointerButton::Primary) {
            match self.envelope_part_at(p) {
                Some((id, Part::Node(i))) => {
                    self.selected_effect_node = Some((id, i));
                    return true;
                }
                Some((_, Part::Handle { .. })) => return true,
                None => {}
            }
        }
        false
    }

    /// The Envelope tool: the frame's parts as with the Shape tool; a click
    /// on an object selects it and gives it an envelope if it has none;
    /// a click on nothing clears the selection.
    pub fn envelope_tool_input(&mut self, response: &Response, p: Point) {
        if self.envelope_input(response, p) {
            return;
        }
        if response.clicked_by(PointerButton::Primary) {
            match self.hit_test(p) {
                Some(id) => {
                    self.select(vec![id]);
                    self.selected_effect_node = None;
                    if self.envelope_of(id).is_none() {
                        self.add_new_envelope();
                    }
                }
                None => {
                    self.select(Vec::new());
                    self.selected_effect_node = None;
                }
            }
        }
    }

    /// Add New Envelope: a fresh frame around each selected object as it
    /// now looks (over an envelope it already has), one undo step.
    pub fn add_new_envelope(&mut self) {
        let mut cmds = Vec::new();
        for id in self.selection.clone() {
            let Some(s) = self.doc().find_shape(id) else {
                continue;
            };
            let b = box_at(s, s.effects.len());
            if b.width() < 1e-9 && b.height() < 1e-9 {
                continue;
            }
            let mut effects = s.effects.clone();
            let mut e = Effect::envelope_around(b, self.envelope_keep_lines);
            if let Effect::Envelope { mapping, .. } = &mut e {
                *mapping = self.envelope_mapping;
            }
            effects.push(e);
            cmds.push(Command::SetEffects { shape: id, effects });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Add Envelope", &cmds) {
            self.status = e.to_string();
        }
        self.selected_effect_node = None;
    }

    /// Clear Envelope: each selected object loses its last envelope, one
    /// undo step.
    pub fn clear_envelope(&mut self) {
        let mut cmds = Vec::new();
        for id in self.selection.clone() {
            let Some(s) = self.doc().find_shape(id) else {
                continue;
            };
            let Some((at, _)) = last_envelope(&s.effects) else {
                continue;
            };
            let mut effects = s.effects.clone();
            effects.remove(at);
            cmds.push(Command::SetEffects { shape: id, effects });
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch("Clear Envelope", &cmds) {
                self.status = e.to_string();
            }
        }
        self.selected_effect_node = None;
    }

    /// Change the last envelope of each selected object with `f`, one undo
    /// step named `label`.
    fn edit_envelopes(&mut self, label: &'static str, f: impl Fn(&mut Effect)) {
        let mut cmds = Vec::new();
        for id in self.selection.clone() {
            let Some(s) = self.doc().find_shape(id) else {
                continue;
            };
            let Some((at, _)) = last_envelope(&s.effects) else {
                continue;
            };
            let mut effects = s.effects.clone();
            if let Some(e) = effects.get_mut(at) {
                f(e);
            }
            if effects != s.effects {
                cmds.push(Command::SetEffects { shape: id, effects });
            }
        }
        if !cmds.is_empty() {
            if let Err(e) = self.engine.run_batch(label, &cmds) {
                self.status = e.to_string();
            }
        }
    }

    /// The mapping new envelopes take and the selected ones change to.
    pub fn set_envelope_mapping(&mut self, m: Mapping) {
        self.envelope_mapping = m;
        self.edit_envelopes("Envelope Mapping", |e| {
            if let Effect::Envelope { mapping, .. } = e {
                *mapping = m;
            }
        });
    }

    /// Keep Lines for new envelopes and the selected ones.
    pub fn set_envelope_keep_lines(&mut self, on: bool) {
        self.envelope_keep_lines = on;
        self.edit_envelopes("Keep Lines", |e| {
            if let Effect::Envelope { keep_lines, .. } = e {
                *keep_lines = on;
            }
        });
    }

    /// The mapping and Keep Lines the bar shows: the first selected
    /// envelope's, or what new envelopes take.
    pub fn envelope_settings_shown(&self) -> (Mapping, bool) {
        for id in &self.selection {
            if let Some(s) = self.doc().find_shape(*id) {
                if let Some((at, _)) = last_envelope(&s.effects) {
                    if let Some(Effect::Envelope {
                        mapping,
                        keep_lines,
                        ..
                    }) = s.effects.get(at)
                    {
                        return (*mapping, *keep_lines);
                    }
                }
            }
        }
        (self.envelope_mapping, self.envelope_keep_lines)
    }

    /// A preset in place of each selected object's last envelope (or as a
    /// new one), around the object where the envelope takes it.
    pub fn apply_envelope_preset(&mut self, preset: Preset) {
        self.apply_envelope_frame(|b| preset.frame(b));
    }

    /// The frame `make` gives each selected object's box in place of its
    /// last envelope (or as a new one), one undo step.
    pub fn apply_envelope_frame(&mut self, make: impl Fn(Rect) -> Frame) {
        let mut cmds = Vec::new();
        for id in self.selection.clone() {
            let Some(s) = self.doc().find_shape(id) else {
                continue;
            };
            let mut effects = s.effects.clone();
            let at = match last_envelope(&effects) {
                Some((at, _)) => at,
                None => {
                    effects.push(Effect::envelope_around(
                        Rect::ZERO,
                        self.envelope_keep_lines,
                    ));
                    effects.len() - 1
                }
            };
            let b = box_at(s, at);
            if b.width() < 1e-9 || b.height() < 1e-9 {
                continue;
            }
            let (keep, mapping) = match effects.get(at) {
                Some(Effect::Envelope {
                    keep_lines,
                    mapping,
                    ..
                }) if last_envelope(&s.effects).is_some() => (*keep_lines, *mapping),
                _ => (self.envelope_keep_lines, self.envelope_mapping),
            };
            if let Some(e) = effects.get_mut(at) {
                *e = Effect::from_envelope(&make(b), keep, mapping);
            }
            cmds.push(Command::SetEffects { shape: id, effects });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Envelope Preset", &cmds) {
            self.status = e.to_string();
        }
        self.selected_effect_node = None;
    }

    /// Copy Envelope Properties: the next click picks the object whose
    /// envelope the selection takes.
    pub fn start_copy_envelope(&mut self) {
        if self.selection.is_empty() {
            return;
        }
        self.pending_copy_envelope = true;
        self.status = tr("status.click_source_object");
    }

    /// The selected objects take `source`'s last envelope, its frame
    /// stretched from `source`'s box to each object's, one undo step.
    pub fn copy_envelope_from(&mut self, source: ShapeId) {
        let Some(src) = self.doc().find_shape(source).cloned() else {
            return;
        };
        let Some((src_at, frame)) = last_envelope(&src.effects) else {
            self.status = tr("status.no_envelope_to_copy");
            return;
        };
        let (keep, mapping) = match src.effects.get(src_at) {
            Some(Effect::Envelope {
                keep_lines,
                mapping,
                ..
            }) => (*keep_lines, *mapping),
            _ => (false, Mapping::Putty),
        };
        let from = box_at(&src, src_at);
        if from.width() < 1e-9 || from.height() < 1e-9 {
            return;
        }
        let mut cmds = Vec::new();
        for id in self.selection.clone() {
            if id == source {
                continue;
            }
            let Some(s) = self.doc().find_shape(id) else {
                continue;
            };
            let mut effects = s.effects.clone();
            let at = match last_envelope(&effects) {
                Some((at, _)) => at,
                None => {
                    effects.push(Effect::envelope_around(Rect::ZERO, keep));
                    effects.len() - 1
                }
            };
            let to = box_at(s, at);
            let m = Affine::translate(to.origin().to_vec2())
                * Affine::scale_non_uniform(to.width() / from.width(), to.height() / from.height())
                * Affine::translate(-from.origin().to_vec2());
            if let Some(e) = effects.get_mut(at) {
                *e = Effect::from_envelope(&frame.transformed(m), keep, mapping);
            }
            cmds.push(Command::SetEffects { shape: id, effects });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Copy Envelope", &cmds) {
            self.status = e.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::ParamCurve;
    use tracedraw_core::ShapeKind;

    fn rect(app: &mut App, x0: f64, y0: f64, x1: f64, y1: f64) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(x0, y0, x1, y1),
            radius: 0.0,
            corners: None,
        })
        .expect("shape")
    }

    fn bounds(app: &App, id: ShapeId) -> Rect {
        let s = app.doc().shape_on_page(id).expect("shape");
        tracedraw_core::live::evaluate(&s).main.bounds()
    }

    /// A drag moves a node the chosen way from where the drag began, the
    /// whole drag one undo step; a second drag is a second step.
    #[test]
    fn dragging_a_node_is_one_step_in_the_chosen_mode() {
        let mut app = App::headless();
        let id = rect(&mut app, 0.0, 0.0, 40.0, 20.0);
        app.select(vec![id]);
        app.add_new_envelope();
        assert!(app.envelope_of(id).is_some());
        let depth = app.engine.history_labels().0.len();
        app.envelope_mode = Mode::StraightLine;
        // The top's middle node, (20, 20), up by 10 in three steps.
        app.begin_envelope_drag(id, Part::Node(5), Point::new(20.0, 20.0));
        for y in [22.0, 26.0, 30.0] {
            app.envelope_drag_to(Point::new(20.0, y));
        }
        app.envelope_drag = None;
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let (_, f) = app.envelope_of(id).expect("envelope");
        assert!((f.nodes[5].y - 30.0).abs() < 1e-9);
        // Straight from the corner to the raised middle.
        let mid = f.segment(4).eval(0.5);
        assert!((mid.y - 25.0).abs() < 1e-9, "{mid:?}");
        assert!((bounds(&app, id).y1 - 30.0).abs() < 1e-6);
        // Another drag is another step.
        app.begin_envelope_drag(id, Part::Node(1), Point::new(20.0, 0.0));
        app.envelope_drag_to(Point::new(20.0, -5.0));
        app.envelope_drag = None;
        assert_eq!(app.engine.history_labels().0.len(), depth + 2);
        app.undo();
        assert!((app.envelope_of(id).expect("env").1.nodes[1].y).abs() < 1e-9);
    }

    /// In the Unconstrained mode the selected node's handles show and move
    /// on their own; nodes come and go with double-clicks.
    #[test]
    fn handles_and_nodes_in_the_unconstrained_mode() {
        let mut app = App::headless();
        let id = rect(&mut app, 0.0, 0.0, 40.0, 20.0);
        app.select(vec![id]);
        app.add_new_envelope();
        app.envelope_mode = Mode::Unconstrained;
        assert!(app
            .envelope_handles(id, &app.envelope_of(id).expect("e").1)
            .is_empty());
        app.selected_effect_node = Some((id, 5));
        let (_, f) = app.envelope_of(id).expect("e");
        let handles = app.envelope_handles(id, &f);
        assert_eq!(handles.len(), 2);
        let (seg, end, h, _) = handles[0];
        assert_eq!((seg, end), (5, 0));
        app.begin_envelope_drag(id, Part::Handle { seg, end }, h);
        app.envelope_drag_to(h + Vec2::new(0.0, 8.0));
        app.envelope_drag = None;
        let (_, f2) = app.envelope_of(id).expect("e");
        assert!((f2.handles[5][0].y - (h.y + 8.0)).abs() < 1e-9);
        assert_eq!(f2.nodes, f.nodes);
        // No handles in the other modes.
        app.envelope_mode = Mode::SingleArc;
        assert!(app.envelope_handles(id, &f2).is_empty());
        // A double-click on the bottom adds a node; on it, removes it; a
        // corner stays.
        app.view.zoom = 1.0;
        assert!(app.envelope_double_click(Point::new(10.0, 0.0)));
        assert_eq!(app.envelope_of(id).expect("e").1.len(), 9);
        assert!(app.envelope_double_click(Point::new(10.0, 0.0)));
        assert_eq!(app.envelope_of(id).expect("e").1.len(), 8);
        assert!(!app.envelope_double_click(Point::new(0.0, 0.0)));
        assert_eq!(app.envelope_of(id).expect("e").1.len(), 8);
    }

    /// An enveloped object is picked and boxed where it is drawn: a click
    /// on the arch above the rectangle selects it, and the selection's
    /// bounds reach the top of the arch.
    #[test]
    fn enveloped_objects_are_hit_and_boxed_as_drawn() {
        let mut app = App::headless();
        let id = rect(&mut app, 0.0, 0.0, 40.0, 20.0);
        app.select(vec![id]);
        app.apply_envelope_preset(Preset::Arch);
        app.select(Vec::new());
        app.view.zoom = 1.0;
        // Above the rectangle, inside the arch (rectangles are filled).
        let fill = tracedraw_core::Fill::Solid(tracedraw_core::Color::BLACK);
        app.run(Command::SetFill {
            shapes: vec![id],
            fill,
        });
        assert_eq!(app.hit_test(Point::new(20.0, 24.0)), Some(id));
        assert_eq!(app.hit_test(Point::new(20.0, 40.0)), None);
        app.select(vec![id]);
        let b = app.selection_bounds().expect("bounds");
        assert!((b.y1 - 26.0).abs() < 0.1, "{b:?}");
        // A marquee around the drawn object takes it; one around the
        // rectangle alone does not.
        use crate::select_region::{takes, Region};
        let s = app.doc().shape_on_page(id).expect("shape");
        let all = |_: &tracedraw_core::Shape| true;
        assert!(takes(
            Region::Rect(Rect::new(-1.0, -1.0, 41.0, 27.0)),
            &s,
            false,
            &all
        ));
        assert!(!takes(
            Region::Rect(Rect::new(-1.0, -1.0, 41.0, 21.0)),
            &s,
            false,
            &all
        ));
    }

    /// Presets, mapping, Keep Lines, Clear and Copy work on the selection.
    #[test]
    fn presets_settings_clear_and_copy() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 40.0, 20.0);
        let b = rect(&mut app, 100.0, 0.0, 120.0, 40.0);
        app.select(vec![a]);
        app.apply_envelope_preset(Preset::Arch);
        assert!(
            (bounds(&app, a).y1 - 26.0).abs() < 0.1,
            "{:?}",
            bounds(&app, a)
        );
        app.set_envelope_mapping(Mapping::Vertical);
        app.set_envelope_keep_lines(true);
        assert_eq!(app.envelope_settings_shown(), (Mapping::Vertical, true));
        // Copy onto b: the arch stretched to b's box.
        app.select(vec![b]);
        app.copy_envelope_from(a);
        let (_, f) = app.envelope_of(b).expect("copied");
        assert!(
            (f.nodes[5] - Point::new(110.0, 52.0)).hypot() < 1e-6,
            "{:?}",
            f.nodes[5]
        );
        assert_eq!(app.envelope_settings_shown(), (Mapping::Vertical, true));
        // Add New goes over the first; Clear takes the last away.
        app.add_new_envelope();
        let n = |app: &App| {
            app.doc()
                .find_shape(b)
                .map(|s| {
                    s.effects
                        .iter()
                        .filter(|e| e.envelope_frame().is_some())
                        .count()
                })
                .unwrap_or(0)
        };
        assert_eq!(n(&app), 2);
        app.clear_envelope();
        assert_eq!(n(&app), 1);
        app.clear_envelope();
        assert_eq!(n(&app), 0);
        // Every preset makes a frame around the box, corners included.
        for p in Preset::ALL {
            let f = p.frame(Rect::new(0.0, 0.0, 10.0, 10.0));
            assert_eq!(f.len(), 8, "{p:?}");
        }
    }

    fn page_shape(app: &App, id: ShapeId) -> tracedraw_core::Shape {
        app.doc().shape_on_page(id).expect("shape")
    }

    /// Convert to Curves makes the envelope part of the curve (#647): it
    /// used to do nothing on an enveloped curve, and a rectangle lost its
    /// envelope's shape. One undo step brings the envelope back.
    #[test]
    fn convert_to_curves_bakes_the_envelope() {
        let mut app = App::headless();
        let id = rect(&mut app, 0.0, 0.0, 40.0, 20.0);
        app.select(vec![id]);
        app.convert_to_curves();
        assert!(!app.can_convert_to_curves());
        app.apply_envelope_preset(Preset::Arch);
        assert!(app.can_convert_to_curves());
        app.convert_to_curves();
        let s = page_shape(&app, id);
        assert!(matches!(s.kind, ShapeKind::Path { .. }), "{:?}", s.kind);
        assert!(s.effects.is_empty() && app.envelope_of(id).is_none());
        assert!((s.bounds().y1 - 26.0).abs() < 0.1, "{:?}", s.bounds());
        assert_eq!(app.engine.undo_label(), Some("Convert To Curves"));
        assert!(!app.can_convert_to_curves());
        app.undo();
        assert!(app.envelope_of(id).is_some());
        // A rectangle with an envelope becomes the warped curve at once.
        let r = rect(&mut app, 100.0, 0.0, 140.0, 20.0);
        app.select(vec![r]);
        app.apply_envelope_preset(Preset::Arch);
        app.convert_to_curves();
        let s = page_shape(&app, r);
        assert!(matches!(s.kind, ShapeKind::Path { .. }), "{:?}", s.kind);
        assert!(s.effects.is_empty());
        assert!((s.bounds().y1 - 26.0).abs() < 0.1, "{:?}", s.bounds());
        assert!((s.bounds().x0 - 100.0).abs() < 0.1, "{:?}", s.bounds());
    }

    /// Break Apart of an enveloped curve breaks the warped curve: the
    /// pieces are drawn where the whole was, with no envelope left (they
    /// used to lose the warp).
    #[test]
    fn break_apart_bakes_an_enveloped_curve() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 15.0, 20.0);
        let b = rect(&mut app, 25.0, 0.0, 40.0, 20.0);
        app.select(vec![a, b]);
        app.combine();
        assert_eq!(app.selection.len(), 1);
        app.apply_envelope_preset(Preset::Arch);
        let drawn = app.selection_bounds().expect("bounds");
        assert!(drawn.y1 > 25.0, "{drawn:?}");
        app.break_apart();
        assert_eq!(app.selection.len(), 2, "{:?}", app.selection);
        let mut all: Option<Rect> = None;
        for id in app.selection.clone() {
            let s = page_shape(&app, id);
            assert!(s.effects.is_empty(), "{:?}", s.effects);
            assert!(matches!(s.kind, ShapeKind::Path { .. }));
            all = Some(all.map_or(s.bounds(), |u| u.union(s.bounds())));
        }
        let all = all.expect("pieces");
        for (got, want) in [
            (all.x0, drawn.x0),
            (all.y0, drawn.y0),
            (all.x1, drawn.x1),
            (all.y1, drawn.y1),
        ] {
            assert!((got - want).abs() < 0.1, "{all:?} {drawn:?}");
        }
        assert_eq!(app.engine.undo_label(), Some("Break Apart"));
    }

    /// An envelope on a group bakes into each of its objects: the group
    /// stays, with the same objects, now curves.
    #[test]
    fn an_enveloped_group_converts_object_by_object() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 15.0, 20.0);
        let b = rect(&mut app, 25.0, 0.0, 40.0, 20.0);
        app.select(vec![a, b]);
        app.group_selection();
        let g = app.selection[0];
        app.apply_envelope_preset(Preset::Arch);
        assert!(app.envelope_of(g).is_some());
        assert!(app.can_convert_to_curves());
        app.convert_to_curves();
        let s = page_shape(&app, g);
        assert!(s.effects.is_empty() && app.envelope_of(g).is_none());
        let ShapeKind::Group { children } = &s.kind else {
            panic!("still a group: {:?}", s.kind);
        };
        let mut ids: Vec<ShapeId> = children.iter().map(|c| c.id).collect();
        ids.sort_by_key(|id| id.0);
        let mut want = vec![a, b];
        want.sort_by_key(|id| id.0);
        assert_eq!(ids, want);
        assert!(children
            .iter()
            .all(|c| matches!(c.kind, ShapeKind::Path { .. })));
        assert!(s.bounds().y1 > 25.0, "{:?}", s.bounds());
    }

    /// Break Apart has nothing to break in a group: it keeps its envelope
    /// and its objects (Convert to Curves bakes it).
    #[test]
    fn break_apart_leaves_an_enveloped_group() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 15.0, 20.0);
        let b = rect(&mut app, 25.0, 0.0, 40.0, 20.0);
        app.select(vec![a, b]);
        app.group_selection();
        let g = app.selection[0];
        app.apply_envelope_preset(Preset::Arch);
        let before = page_shape(&app, g);
        app.break_apart();
        assert_eq!(page_shape(&app, g).kind, before.kind);
        assert!(app.envelope_of(g).is_some());
    }

    /// A brush press on an enveloped group leaves it as it is: brushes
    /// never touch groups, so nothing is converted or baked.
    #[test]
    fn a_brush_press_leaves_an_enveloped_group() {
        let mut app = App::headless();
        let a = rect(&mut app, 0.0, 0.0, 15.0, 20.0);
        let b = rect(&mut app, 25.0, 0.0, 40.0, 20.0);
        app.select(vec![a, b]);
        app.group_selection();
        let g = app.selection[0];
        app.apply_envelope_preset(Preset::Arch);
        let before = page_shape(&app, g);
        let depth = app.engine.history_labels().0.len();
        app.brush_begin(Point::new(10.0, 10.0));
        let after = page_shape(&app, g);
        assert_eq!(after.kind, before.kind);
        assert_eq!(after.effects, before.effects);
        assert_eq!(app.engine.history_labels().0.len(), depth);
    }
}
