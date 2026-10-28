//! The Distort tool: Push and Pull, Zipper and Twister as a live effect
//! (`Effect::Distort`, formulas in `tracedraw_core::distort`).
//!
//! A drag on an object distorts it about the point the drag started on:
//! Push and Pull by how far the pointer went right (pushed) or left
//! (pulled), a full half width of the object being 100; Zipper with teeth
//! a quarter of the drag's length high; Twister by the angle the pointer
//! turned about the start, whole turns included. A selected distortion
//! shows its handles: the centre (a diamond, dragged to move it), a line
//! to the end handle (a square whose distance from the centre is the
//! Push and Pull amount or the Zipper amplitude; a circle whose angle
//! about the centre is the Twister angle, whole turns counted while it
//! is dragged round) and, for the Zipper, a slider on the line for the
//! frequency (#1122). Each drag is one undo step and replaces the
//! object's distortion.
//!
//! The property bar shows the selected object's distortion (or the
//! tool's settings) and edits the selected distortions live.

use crate::app::App;
use serde::{Deserialize, Serialize};
use tracedraw_core::{
    distort::{Distort, MAX_FREQUENCY},
    geometry::{Point, Shape as _, Vec2},
    live::Effect,
    Command, ShapeId, ShapeKind,
};

/// The kind of distortion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistortMode {
    #[default]
    PushPull,
    Zipper,
    Twister,
}

impl DistortMode {
    pub const ALL: [DistortMode; 3] = [
        DistortMode::PushPull,
        DistortMode::Zipper,
        DistortMode::Twister,
    ];

    pub fn key(self) -> &'static str {
        match self {
            DistortMode::PushPull => "toolbar.push_pull",
            DistortMode::Zipper => "toolbar.zipper",
            DistortMode::Twister => "toolbar.twister",
        }
    }

    pub fn of(d: &Distort) -> DistortMode {
        match d {
            Distort::PushPull { .. } => DistortMode::PushPull,
            Distort::Zipper { .. } => DistortMode::Zipper,
            Distort::Twister { .. } => DistortMode::Twister,
        }
    }
}

/// The Distort tool's values, one set per kind.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DistortSettings {
    pub mode: DistortMode,
    /// Push and Pull, percent of the nodes' distance (-200 to 200).
    pub amount: f64,
    /// Zipper: the teeth's height (mm), how many on each segment, and
    /// its options.
    pub amplitude: f64,
    pub frequency: u32,
    pub random: bool,
    pub smooth: bool,
    pub local: bool,
    /// Twister: whole turns, degrees more (0 to 359) and the direction.
    pub turns: u32,
    pub degrees: f64,
    pub clockwise: bool,
}

impl Default for DistortSettings {
    fn default() -> Self {
        DistortSettings {
            mode: DistortMode::PushPull,
            amount: 20.0,
            amplitude: 2.0,
            frequency: 6,
            random: false,
            smooth: false,
            local: false,
            turns: 0,
            degrees: 90.0,
            clockwise: true,
        }
    }
}

/// The highest number of whole turns the bar takes.
pub const MAX_TURNS: u32 = 9;

impl DistortSettings {
    /// The same values in their ranges (a preferences file edited by
    /// hand).
    pub fn clamped(self) -> Self {
        let d = DistortSettings::default();
        let fin = |v: f64, lo: f64, hi: f64, or: f64| {
            if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                or
            }
        };
        DistortSettings {
            amount: fin(self.amount, -200.0, 200.0, d.amount),
            amplitude: fin(self.amplitude, 0.0, 100.0, d.amplitude),
            frequency: self.frequency.min(MAX_FREQUENCY),
            turns: self.turns.min(MAX_TURNS),
            degrees: fin(self.degrees, 0.0, 359.0, d.degrees),
            ..self
        }
    }

    /// The Twister angle, degrees (counterclockwise positive).
    pub fn angle(&self) -> f64 {
        let a = self.turns as f64 * 360.0 + self.degrees;
        if self.clockwise {
            -a
        } else {
            a
        }
    }

    /// The distortion of kind `mode` with these values about `center`.
    pub fn effect(&self, mode: DistortMode, center: Option<Point>) -> Distort {
        match mode {
            DistortMode::PushPull => Distort::PushPull {
                amount: self.amount,
                center,
            },
            DistortMode::Zipper => Distort::Zipper {
                amplitude: self.amplitude,
                frequency: self.frequency,
                center,
                random: self.random,
                smooth: self.smooth,
                local: self.local,
            },
            DistortMode::Twister => Distort::Twister {
                angle_deg: self.angle(),
                center,
            },
        }
    }

    /// Take the values of `d` (its kind becomes the mode).
    pub fn take(&mut self, d: &Distort) {
        self.mode = DistortMode::of(d);
        match *d {
            Distort::PushPull { amount, .. } => self.amount = amount,
            Distort::Zipper {
                amplitude,
                frequency,
                random,
                smooth,
                local,
                ..
            } => {
                self.amplitude = amplitude;
                self.frequency = frequency;
                self.random = random;
                self.smooth = smooth;
                self.local = local;
            }
            Distort::Twister { angle_deg, .. } => {
                let a = if angle_deg.is_finite() {
                    angle_deg
                } else {
                    0.0
                };
                self.clockwise = a < 0.0;
                let a = a.abs().min(MAX_TURNS as f64 * 360.0 + 359.0);
                self.turns = (a / 360.0).floor() as u32;
                self.degrees = (a - self.turns as f64 * 360.0).round().min(359.0);
            }
        }
        *self = self.clamped();
    }
}

/// What a drag of the Distort tool holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grab {
    /// The object: a new distortion about where the drag started.
    Object,
    /// The centre marker: the centre moves with the pointer.
    Center,
    /// The end handle: the amount or amplitude from its distance to the
    /// centre, the Twister angle from its direction.
    End,
    /// The Zipper's frequency slider.
    Frequency,
}

/// A drag of the Distort tool under way.
#[derive(Debug, Clone, PartialEq)]
pub struct DistortDrag {
    pub id: ShapeId,
    /// Where the drag started (page space) and the same point in the
    /// object's own space (the centre).
    pub start: Point,
    pub center: Point,
    /// Half the object's larger side (page mm): Push and Pull's 100.
    pub reach: f64,
    pub grab: Grab,
    /// Twister: the turn so far (radians) and the pointer's last angle
    /// about the start.
    pub turned: f64,
    pub last_angle: Option<f64>,
}

/// The handles of a selected distortion, page space.
#[derive(Debug, Clone, PartialEq)]
pub struct DistortHandles {
    pub id: ShapeId,
    pub mode: DistortMode,
    pub center: Point,
    /// The end of the handle line: a square at the amount (Push and
    /// Pull, half the object's larger side being 100, to the left when
    /// negative) or four times the amplitude (Zipper, as the drag that
    /// made it) to the right of the centre; a circle on the Twister's
    /// angle, half the larger side away.
    pub end: Point,
    /// Zipper: the frequency slider, `frequency / MAX_FREQUENCY` of the
    /// way from the centre to the end.
    pub slider: Option<Point>,
}

/// How near (pixels) a press must be to a handle to take it.
const HANDLE_PICK_PX: f64 = 6.0;

/// The distance of the Zipper's end handle for an amplitude.
const AMPLITUDE_REACH: f64 = 4.0;

/// Objects the tool distorts: not groups or bitmaps.
pub fn distortable(kind: &ShapeKind) -> bool {
    !matches!(kind, ShapeKind::Bitmap { .. } | ShapeKind::Group { .. })
}

/// The distortion among `effects`, if any.
pub fn distortion_of(effects: &[Effect]) -> Option<Distort> {
    effects.iter().find_map(|e| match e {
        Effect::Distort(d) => Some(*d),
        _ => None,
    })
}

/// `effects` with the distortion replaced in place by `d` (added at the
/// end when there is none; removed with `None`).
pub fn with_distortion(effects: &[Effect], d: Option<Distort>) -> Vec<Effect> {
    let mut out = Vec::with_capacity(effects.len() + 1);
    let mut placed = false;
    for e in effects {
        if matches!(e, Effect::Distort(_)) {
            if !placed {
                if let Some(d) = d {
                    out.push(Effect::Distort(d));
                }
                placed = true;
            }
        } else {
            out.push(e.clone());
        }
    }
    if !placed {
        if let Some(d) = d {
            out.push(Effect::Distort(d));
        }
    }
    out
}

impl App {
    /// The distortion the bar shows: the first selected object's.
    pub fn selected_distortion(&self) -> Option<Distort> {
        self.selected_shapes()
            .iter()
            .find_map(|s| distortion_of(&s.effects))
    }

    /// The selected objects that are distorted.
    fn distorted_selection(&self) -> Vec<(ShapeId, Distort, Vec<Effect>)> {
        self.selected_shapes()
            .into_iter()
            .filter_map(|s| distortion_of(&s.effects).map(|d| (s.id, d, s.effects.clone())))
            .collect()
    }

    /// Change every selected distortion with `f`, one undo step labelled
    /// `label` (a run of the same change on the same objects with
    /// nothing in between is one step).
    pub fn change_distortions(
        &mut self,
        label: &'static str,
        f: impl Fn(&Distort) -> Option<Distort>,
    ) {
        let targets = self.distorted_selection();
        self.set_distortions(label, targets, f);
    }

    /// [`App::change_distortions`] on `targets` (object, its distortion,
    /// its effects).
    fn set_distortions(
        &mut self,
        label: &'static str,
        targets: Vec<(ShapeId, Distort, Vec<Effect>)>,
        f: impl Fn(&Distort) -> Option<Distort>,
    ) {
        let mut cmds = Vec::new();
        let mut ids = Vec::new();
        for (id, d, effects) in targets {
            let next = f(&d);
            if next == Some(d) && distortion_of(&effects) == Some(d) {
                continue;
            }
            ids.push(id);
            cmds.push(Command::SetEffects {
                shape: id,
                effects: with_distortion(&effects, next),
            });
        }
        if cmds.is_empty() {
            return;
        }
        let key = (self.engine.revision(), ids.clone(), label.to_string());
        if self.engine.undo_label() == Some(label) && self.effect_edit_step.as_ref() == Some(&key) {
            let _ = self.engine.undo();
        }
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
            return;
        }
        self.effect_edit_step = Some((self.engine.revision(), ids, label.to_string()));
    }

    /// The bar's values went to the tool; the selected distortions of the
    /// mode shown take them (keeping their centres), and selected objects
    /// without one take it about their middles, so an edit on the bar
    /// shows on the selection at once (#1030). Typing on, the edits are
    /// one undo step.
    pub fn apply_distort_settings(&mut self) {
        self.distort = self.distort.clamped();
        let s = self.distort;
        let targets: Vec<(ShapeId, Distort, Vec<Effect>)> = self
            .selected_shapes()
            .into_iter()
            .filter_map(|sh| match distortion_of(&sh.effects) {
                Some(d) => Some((sh.id, d, sh.effects.clone())),
                None if distortable(&sh.kind) => {
                    Some((sh.id, s.effect(s.mode, None), sh.effects.clone()))
                }
                None => None,
            })
            .collect();
        self.set_distortions("Distortion", targets, |d| {
            Some(s.effect(s.mode, d.center()))
        });
    }

    /// A mode button: the tool's mode, and the selected distortions turn
    /// into that kind with the tool's values for it.
    pub fn set_distort_mode(&mut self, mode: DistortMode) {
        self.distort.mode = mode;
        self.distort = self.distort.clamped();
        let s = self.distort;
        self.change_distortions("Distortion", |d| Some(s.effect(s.mode, d.center())));
    }

    /// Apply the bar's distortion to the selected objects that have none
    /// (about their middles).
    pub fn add_distortion(&mut self) {
        let s = self.distort.clamped();
        let mut cmds = Vec::new();
        for sh in self.selected_shapes() {
            if distortion_of(&sh.effects).is_some() || !distortable(&sh.kind) {
                continue;
            }
            cmds.push(Command::SetEffects {
                shape: sh.id,
                effects: with_distortion(&sh.effects, Some(s.effect(s.mode, None))),
            });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Distortion", &cmds) {
            self.status = e.to_string();
        }
    }

    /// Center Distortion: the selected distortions about their objects'
    /// middles again.
    pub fn center_distortion(&mut self) {
        self.change_distortions("Center Distortion", |d| Some(d.with_center(None)));
    }

    /// Clear Distortion.
    pub fn clear_distortion(&mut self) {
        self.change_distortions("Clear Distortion", |_| None);
    }

    /// Convert to Curves: the selected objects' distortions become their
    /// outlines.
    pub fn bake_distortion(&mut self) {
        let mut cmds = Vec::new();
        for (id, d, effects) in self.distorted_selection() {
            let Some(s) = self.doc().find_shape(id) else {
                continue;
            };
            let path = tracedraw_core::distort::distort(&s.local_path(), d);
            let closed = path
                .elements()
                .iter()
                .any(|e| matches!(e, tracedraw_core::geometry::PathEl::ClosePath));
            cmds.push(Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Path { path, closed },
            });
            cmds.push(Command::SetEffects {
                shape: id,
                effects: with_distortion(&effects, None),
            });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Convert To Curves", &cmds) {
            self.status = e.to_string();
        }
    }

    /// The handles of the selected distortions, page space.
    pub fn distortion_handles(&self) -> Vec<DistortHandles> {
        self.selection
            .iter()
            .filter_map(|id| {
                let s = self.doc().shape_on_page(*id)?;
                let d = distortion_of(&s.effects)?;
                let local = d
                    .center()
                    .unwrap_or_else(|| s.local_path().bounding_box().center());
                let center = s.transform * local;
                if !(center.x.is_finite() && center.y.is_finite()) {
                    return None;
                }
                let b = s.bounds();
                let reach = (b.width().max(b.height()) / 2.0).max(1.0);
                let (end, slider) = match d {
                    Distort::PushPull { amount, .. } => {
                        let a = if amount.is_finite() { amount } else { 0.0 };
                        (center + Vec2::new(a / 100.0 * reach, 0.0), None)
                    }
                    Distort::Zipper {
                        amplitude,
                        frequency,
                        ..
                    } => {
                        let a = if amplitude.is_finite() {
                            amplitude
                        } else {
                            0.0
                        };
                        let len = a * AMPLITUDE_REACH;
                        let t = frequency.min(MAX_FREQUENCY) as f64 / MAX_FREQUENCY as f64;
                        (
                            center + Vec2::new(len, 0.0),
                            Some(center + Vec2::new(len * t, 0.0)),
                        )
                    }
                    Distort::Twister { angle_deg, .. } => {
                        let a = if angle_deg.is_finite() {
                            angle_deg.to_radians()
                        } else {
                            0.0
                        };
                        (center + Vec2::new(a.cos(), a.sin()) * reach, None)
                    }
                };
                Some(DistortHandles {
                    id: *id,
                    mode: DistortMode::of(&d),
                    center,
                    end,
                    slider,
                })
            })
            .collect()
    }

    /// Start a drag at `at` (page space): on a handle, take it; on an
    /// object, distort it about `at`. False when there is nothing there.
    pub fn begin_distort(&mut self, at: Point) -> bool {
        let tol = HANDLE_PICK_PX / self.view.zoom.max(1e-6) as f64;
        let near = |p: Point| (p - at).hypot() <= tol;
        let handles = self.distortion_handles();
        // The centre first, then the slider (it can sit on the line's
        // end), then the end.
        let grabbed = handles
            .iter()
            .find(|h| near(h.center))
            .map(|h| (h, Grab::Center))
            .or_else(|| {
                handles
                    .iter()
                    .find(|h| h.slider.is_some_and(near))
                    .map(|h| (h, Grab::Frequency))
            })
            .or_else(|| handles.iter().find(|h| near(h.end)).map(|h| (h, Grab::End)));
        if let Some((h, grab)) = grabbed {
            let Some(s) = self.doc().shape_on_page(h.id) else {
                return false;
            };
            let b = s.bounds();
            let reach = (b.width().max(b.height()) / 2.0).max(1.0);
            // The Twister's handle goes on turning from its angle.
            let (turned, last_angle) = match distortion_of(&s.effects) {
                Some(Distort::Twister { angle_deg, .. }) if angle_deg.is_finite() => {
                    (angle_deg.to_radians(), Some((h.end - h.center).atan2()))
                }
                _ => (0.0, None),
            };
            if let Some(d) = distortion_of(&s.effects) {
                self.distort.take(&d);
            }
            self.distort_drag = Some(DistortDrag {
                id: h.id,
                start: h.center,
                center: h.center,
                reach,
                grab,
                turned,
                last_angle,
            });
            return true;
        }
        let Some(id) = self.hit_test_inside(at) else {
            return false;
        };
        let Some(s) = self.doc().shape_on_page(id) else {
            return false;
        };
        if matches!(s.kind, ShapeKind::Bitmap { .. } | ShapeKind::Group { .. }) {
            return false;
        }
        if s.transform.determinant().abs() < 1e-12 {
            return false;
        }
        self.select(vec![id]);
        let b = s.bounds();
        let reach = (b.width().max(b.height()) / 2.0).max(1.0);
        self.distort_drag = Some(DistortDrag {
            id,
            start: at,
            center: s.transform.inverse() * at,
            reach,
            grab: Grab::Object,
            turned: 0.0,
            last_angle: None,
        });
        true
    }

    /// The pointer's turn about `dd.start` added to `dd.turned`, the
    /// shortest way round from its last angle (so whole turns count).
    fn turn_toward(&self, dd: &mut DistortDrag, p: Point) {
        let d = p - dd.start;
        let tol = 2.0 / self.view.zoom.max(1e-6) as f64;
        if d.hypot() <= tol {
            return;
        }
        let a = d.atan2();
        if let Some(last) = dd.last_angle {
            let mut da = a - last;
            while da > std::f64::consts::PI {
                da -= std::f64::consts::TAU;
            }
            while da < -std::f64::consts::PI {
                da += std::f64::consts::TAU;
            }
            dd.turned += da;
        }
        dd.last_angle = Some(a);
    }

    /// The Twister angle (degrees) of a turn, within the bar's range.
    fn turn_degrees(turned: f64) -> f64 {
        let most = MAX_TURNS as f64 * 360.0 + 359.0;
        turned.to_degrees().clamp(-most, most).round()
    }

    /// The pointer moved to `p` during a drag.
    pub fn drag_distort(&mut self, p: Point) {
        let Some(mut dd) = self.distort_drag.clone() else {
            return;
        };
        let Some(s) = self.doc().shape_on_page(dd.id) else {
            self.distort_drag = None;
            return;
        };
        if s.transform.determinant().abs() < 1e-12 {
            return;
        }
        if self.selection != vec![dd.id] {
            self.select(vec![dd.id]);
        }
        match dd.grab {
            Grab::Center => {
                let local = s.transform.inverse() * p;
                self.change_distortions("Distortion Center", |d| Some(d.with_center(Some(local))));
                return;
            }
            Grab::End | Grab::Frequency => {
                // The handles edit the object's distortion as the bar does:
                // its values with the pointer's, its centre kept.
                let Some(current) = distortion_of(&s.effects) else {
                    self.distort_drag = None;
                    return;
                };
                let mut sett = self.distort;
                sett.take(&current);
                let d = p - dd.start;
                match (sett.mode, dd.grab) {
                    (DistortMode::PushPull, _) => {
                        sett.amount = (100.0 * d.x / dd.reach).clamp(-200.0, 200.0).round();
                    }
                    (DistortMode::Zipper, Grab::Frequency) => {
                        let len = sett.amplitude * AMPLITUDE_REACH;
                        if len > 1e-9 {
                            let t = (d.x / len).clamp(0.0, 1.0);
                            sett.frequency = (t * MAX_FREQUENCY as f64).round() as u32;
                        }
                    }
                    (DistortMode::Zipper, _) => {
                        sett.amplitude = (d.hypot() / AMPLITUDE_REACH).clamp(0.0, 100.0);
                    }
                    (DistortMode::Twister, _) => {
                        self.turn_toward(&mut dd, p);
                        sett.take(&Distort::Twister {
                            angle_deg: Self::turn_degrees(dd.turned),
                            center: None,
                        });
                    }
                }
                let sett = sett.clamped();
                self.distort = sett;
                self.distort_drag = Some(dd);
                self.change_distortions("Distortion", |d| Some(sett.effect(sett.mode, d.center())));
                return;
            }
            Grab::Object => {}
        }
        let d = p - dd.start;
        let mode = self.distort.mode;
        match mode {
            DistortMode::PushPull => {
                self.distort.amount = (100.0 * d.x / dd.reach).clamp(-200.0, 200.0).round();
            }
            DistortMode::Zipper => {
                self.distort.amplitude = (d.hypot() / AMPLITUDE_REACH).clamp(0.0, 100.0);
            }
            DistortMode::Twister => {
                self.turn_toward(&mut dd, p);
                self.distort.take(&Distort::Twister {
                    angle_deg: Self::turn_degrees(dd.turned),
                    center: None,
                });
            }
        }
        let effect = self.distort.effect(mode, Some(dd.center));
        self.distort_drag = Some(dd.clone());
        self.push_effect(Effect::Distort(effect), true);
    }

    /// The Distort tool's clicks and drags.
    pub fn distort_input(&mut self, response: &egui::Response, p: Point) {
        use egui::PointerButton;
        if response.drag_started_by(PointerButton::Primary) {
            let origin = response
                .ctx
                .input(|i| i.pointer.press_origin())
                .map(|o| self.view.to_page(o))
                .unwrap_or(p);
            self.begin_distort(origin);
        }
        if response.dragged_by(PointerButton::Primary) && self.distort_drag.is_some() {
            self.drag_distort(p);
        }
        if response.drag_stopped() {
            self.distort_drag = None;
        }
        if response.clicked_by(PointerButton::Primary) {
            match self.hit_test(p) {
                Some(id) => self.select(vec![id]),
                None => self.select(Vec::new()),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use tracedraw_core::geometry::Rect;

    fn square(app: &mut App) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            radius: 0.0,
            corners: None,
        })
        .expect("square")
    }

    fn effect(app: &App, id: ShapeId) -> Option<Distort> {
        app.doc()
            .find_shape(id)
            .and_then(|s| distortion_of(&s.effects))
    }

    fn steps(app: &App) -> usize {
        app.engine.history_labels().0.len()
    }

    #[test]
    fn a_drag_distorts_about_where_it_started() {
        let mut app = App::headless();
        app.tool = Tool::Distort;
        let id = square(&mut app);
        let before = steps(&app);
        // Push and Pull: half the width to the right is 100.
        assert!(app.begin_distort(Point::new(10.0, 10.0)));
        app.drag_distort(Point::new(15.0, 10.0));
        app.drag_distort(Point::new(20.0, 10.0));
        app.distort_drag = None;
        assert_eq!(
            effect(&app, id),
            Some(Distort::PushPull {
                amount: 100.0,
                center: Some(Point::new(10.0, 10.0)),
            })
        );
        assert_eq!(steps(&app), before + 1, "one undo step for the drag");
        // The bar shows it.
        assert_eq!(app.distort.amount, 100.0);
        // Twister: a quarter turn of the pointer about the start, then
        // a whole turn more.
        app.distort.mode = DistortMode::Twister;
        assert!(app.begin_distort(Point::new(5.0, 5.0)));
        let c = Point::new(5.0, 5.0);
        let mut a: f64 = 0.0;
        while a <= std::f64::consts::FRAC_PI_2 * 5.0 + 1e-9 {
            app.drag_distort(c + tracedraw_core::geometry::Vec2::new(a.cos(), a.sin()) * 8.0);
            a += 0.2;
        }
        app.distort_drag = None;
        let Some(Distort::Twister { angle_deg, center }) = effect(&app, id) else {
            panic!("a twister");
        };
        assert!((angle_deg - 450.0).abs() <= 12.0, "{angle_deg}");
        assert_eq!(center, Some(Point::new(5.0, 5.0)));
        assert_eq!((app.distort.turns, app.distort.clockwise), (1, false));
        // Nothing under the pointer: no drag.
        assert!(!app.begin_distort(Point::new(500.0, 500.0)));
    }

    #[test]
    fn the_bar_edits_the_selected_distortion() {
        let mut app = App::headless();
        app.tool = Tool::Distort;
        let id = square(&mut app);
        app.select(vec![id]);
        // Apply adds the bar's distortion about the middle.
        app.distort.mode = DistortMode::Zipper;
        app.distort.frequency = 4;
        app.add_distortion();
        assert!(matches!(
            effect(&app, id),
            Some(Distort::Zipper {
                frequency: 4,
                center: None,
                ..
            })
        ));
        // Fields edit it; a mode button turns it into another kind.
        app.distort.smooth = true;
        app.apply_distort_settings();
        assert!(matches!(
            effect(&app, id),
            Some(Distort::Zipper { smooth: true, .. })
        ));
        app.distort.degrees = 45.0;
        app.distort.clockwise = true;
        app.set_distort_mode(DistortMode::Twister);
        assert_eq!(
            effect(&app, id),
            Some(Distort::Twister {
                angle_deg: -45.0,
                center: None
            })
        );
        // The centre marker sits in the middle and can be dragged.
        let handles = app.distortion_handles();
        assert_eq!(handles.len(), 1);
        assert!((handles[0].center - Point::new(10.0, 10.0)).hypot() < 1e-9);
        assert!(app.begin_distort(Point::new(10.0, 10.0)));
        app.drag_distort(Point::new(14.0, 12.0));
        app.distort_drag = None;
        assert_eq!(
            effect(&app, id).and_then(|d| d.center()),
            Some(Point::new(14.0, 12.0))
        );
        app.center_distortion();
        assert_eq!(effect(&app, id).and_then(|d| d.center()), None);
        // Convert to Curves bakes it; Clear takes it away.
        let shape = app.doc().find_shape(id).cloned().expect("shape");
        let baked = tracedraw_core::live::evaluate(&shape).main.local_path();
        app.bake_distortion();
        let after = app.doc().find_shape(id).cloned().expect("shape");
        assert!(effect(&app, id).is_none());
        assert_eq!(after.local_path(), baked);
        app.undo();
        app.clear_distortion();
        assert!(effect(&app, id).is_none());
    }

    /// A value changed on the bar shows on the selected object at once,
    /// also on one without a distortion yet (about its middle), and a
    /// run of edits is one undo step (#1030).
    #[test]
    fn a_bar_edit_previews_on_an_undistorted_selection() {
        let mut app = App::headless();
        app.tool = Tool::Distort;
        let id = square(&mut app);
        let picture = app
            .new_shape(ShapeKind::Bitmap {
                rect: Rect::new(30.0, 0.0, 40.0, 10.0),
                width_px: 1,
                height_px: 1,
                png: Vec::new().into(),
                fx: None,
            })
            .expect("bitmap");
        app.select(vec![id, picture]);
        let before = steps(&app);
        // Choosing the kind alone leaves the object as it is.
        app.set_distort_mode(DistortMode::Twister);
        assert_eq!(effect(&app, id), None);
        app.distort.degrees = 30.0;
        app.apply_distort_settings();
        app.distort.degrees = 45.0;
        app.apply_distort_settings();
        assert_eq!(
            effect(&app, id),
            Some(Distort::Twister {
                angle_deg: -45.0,
                center: None
            })
        );
        assert_eq!(effect(&app, picture), None, "bitmaps are not distorted");
        assert_eq!(steps(&app), before + 1, "one undo step for the edits");
        app.undo();
        assert_eq!(effect(&app, id), None);
    }

    /// The handles of a selected distortion (#1122): the end square sets
    /// the Push and Pull amount by its distance (to the left negative)
    /// and the Zipper amplitude; the slider on the Zipper's line sets the
    /// frequency; the Twister's circle sets the angle, whole turns
    /// counted as it goes round. Each drag is one undo step.
    #[test]
    fn the_handles_set_the_values_by_dragging() {
        let mut app = App::headless();
        app.tool = Tool::Distort;
        let id = square(&mut app);
        app.select(vec![id]);
        app.distort.mode = DistortMode::PushPull;
        app.distort.amount = 50.0;
        app.add_distortion();
        // Reach is 10 (half of 20): the square sits 5 mm right of the
        // centre; dragged to 10 mm left it is -100.
        let h = &app.distortion_handles()[0];
        assert_eq!(h.mode, DistortMode::PushPull);
        assert!((h.end - Point::new(15.0, 10.0)).hypot() < 1e-9, "{h:?}");
        assert!(h.slider.is_none());
        let before = steps(&app);
        assert!(app.begin_distort(Point::new(15.0, 10.0)));
        assert_eq!(app.distort_drag.as_ref().map(|d| d.grab), Some(Grab::End));
        app.drag_distort(Point::new(5.0, 10.0));
        app.drag_distort(Point::new(0.0, 10.0));
        app.distort_drag = None;
        assert_eq!(
            effect(&app, id),
            Some(Distort::PushPull {
                amount: -100.0,
                center: None
            })
        );
        assert_eq!(steps(&app), before + 1, "one undo step for the drag");
        // Zipper: the square four times the amplitude away; the slider
        // frequency / 100 of the way.
        app.distort.amplitude = 20.0;
        app.distort.frequency = 10;
        app.set_distort_mode(DistortMode::Zipper);
        let h = app.distortion_handles()[0].clone();
        assert!((h.end - Point::new(90.0, 10.0)).hypot() < 1e-9, "{h:?}");
        assert_eq!(h.slider, Some(Point::new(18.0, 10.0)));
        assert!(app.begin_distort(Point::new(18.0, 10.0)));
        assert_eq!(
            app.distort_drag.as_ref().map(|d| d.grab),
            Some(Grab::Frequency)
        );
        app.drag_distort(Point::new(30.0, 10.5));
        app.distort_drag = None;
        assert!(matches!(
            effect(&app, id),
            Some(Distort::Zipper { frequency: 25, .. })
        ));
        assert!(app.begin_distort(Point::new(90.0, 10.0)));
        assert_eq!(app.distort_drag.as_ref().map(|d| d.grab), Some(Grab::End));
        app.drag_distort(Point::new(10.0, 50.0));
        app.distort_drag = None;
        let Some(Distort::Zipper {
            amplitude,
            frequency,
            ..
        }) = effect(&app, id)
        else {
            panic!("a zipper");
        };
        assert!((amplitude - 10.0).abs() < 1e-9, "{amplitude}");
        assert_eq!(frequency, 25, "the slider's value stays");
        // Twister: the circle on the angle; dragged round one and a
        // quarter turns counterclockwise from 30 degrees it is 480.
        app.distort.turns = 0;
        app.distort.degrees = 30.0;
        app.distort.clockwise = false;
        app.set_distort_mode(DistortMode::Twister);
        let h = app.distortion_handles()[0].clone();
        let c = Point::new(10.0, 10.0);
        let want = c + tracedraw_core::geometry::Vec2::new(
            30f64.to_radians().cos(),
            30f64.to_radians().sin(),
        ) * 10.0;
        assert!((h.end - want).hypot() < 1e-9, "{h:?}");
        assert!(app.begin_distort(h.end));
        let mut a: f64 = 30f64.to_radians();
        let stop = (30.0f64 + 450.0).to_radians();
        while a <= stop + 1e-9 {
            app.drag_distort(c + tracedraw_core::geometry::Vec2::new(a.cos(), a.sin()) * 10.0);
            a += 0.2;
        }
        app.drag_distort(c + tracedraw_core::geometry::Vec2::new(stop.cos(), stop.sin()) * 10.0);
        app.distort_drag = None;
        let Some(Distort::Twister { angle_deg, .. }) = effect(&app, id) else {
            panic!("a twister");
        };
        assert!((angle_deg - 480.0).abs() <= 1.0, "{angle_deg}");
        assert_eq!((app.distort.turns, app.distort.clockwise), (1, false));
    }

    #[test]
    fn settings_round_trip_and_stay_in_range() {
        let mut s = DistortSettings::default();
        s.take(&Distort::Twister {
            angle_deg: -800.0,
            center: None,
        });
        assert_eq!((s.turns, s.degrees, s.clockwise), (2, 80.0, true));
        assert_eq!(s.angle(), -800.0);
        let odd = DistortSettings {
            amount: f64::NAN,
            amplitude: -5.0,
            frequency: 10_000,
            turns: 99,
            degrees: 720.0,
            ..Default::default()
        }
        .clamped();
        assert_eq!(odd.amount, 20.0);
        assert_eq!(odd.amplitude, 0.0);
        assert_eq!(odd.frequency, MAX_FREQUENCY);
        assert_eq!((odd.turns, odd.degrees), (MAX_TURNS, 359.0));
        // Replacing in place keeps the order of the other effects.
        let effects = vec![
            Effect::Distort(Distort::PushPull {
                amount: 1.0,
                center: None,
            }),
            Effect::Envelope {
                nodes: vec![],
                keep_lines: false,
                handles: vec![],
                corners: vec![],
                mapping: Default::default(),
            },
        ];
        let next = with_distortion(
            &effects,
            Some(Distort::PushPull {
                amount: 2.0,
                center: None,
            }),
        );
        assert!(matches!(
            next[0],
            Effect::Distort(Distort::PushPull { amount, .. }) if amount == 2.0
        ));
        assert_eq!(with_distortion(&effects, None).len(), 1);
    }
}
