//! The Extrude tool and the settings its bar and the Extrude panel share
//! (`Effect::Extrude`, built by `tracedraw_core::extrude`).
//!
//! A drag on an object extrudes it: a parallel body runs along the drag;
//! the others run toward the point the drag ends on (their vanishing
//! point) by the bar's depth. A drag that starts on the handle of a
//! selected extrusion (its vanishing point, or the middle of a parallel
//! body's copy) moves that handle. Each drag is one undo step.
//!
//! The bar and the panel show the selected object's extrusion and edit
//! the selected extrusions live; the vanishing point is shown and typed
//! in page coordinates, and kept in the object's own space when it is
//! locked to the object, so it moves with it.

use crate::app::App;
use serde::{Deserialize, Serialize};
use tracedraw_core::{
    extrude::ExtrudeKind,
    extrude3d::{Light, Solid, MAX_EXTRA_LIGHTS},
    geometry::{Affine, Point, Vec2},
    live::Effect,
    Color, Command, ShapeId, ShapeKind,
};

/// How the body is coloured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtrudeColor {
    #[default]
    Object,
    Solid,
    Shade,
}

impl ExtrudeColor {
    pub const ALL: [ExtrudeColor; 3] = [
        ExtrudeColor::Object,
        ExtrudeColor::Solid,
        ExtrudeColor::Shade,
    ];

    pub fn key(self) -> &'static str {
        match self {
            ExtrudeColor::Object => "extrude.color_object",
            ExtrudeColor::Solid => "extrude.color_solid",
            ExtrudeColor::Shade => "extrude.color_shade",
        }
    }
}

pub fn kind_key(k: ExtrudeKind) -> &'static str {
    match k {
        ExtrudeKind::SmallBack => "extrude.small_back",
        ExtrudeKind::SmallFront => "extrude.small_front",
        ExtrudeKind::BigBack => "extrude.big_back",
        ExtrudeKind::BigFront => "extrude.big_front",
        ExtrudeKind::BackParallel => "extrude.back_parallel",
        ExtrudeKind::FrontParallel => "extrude.front_parallel",
    }
}

/// The Extrude tool's and panel's values.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtrudeSettings {
    pub kind: ExtrudeKind,
    /// Parallel bodies: from the object to its copy, mm.
    pub vector: Vec2,
    /// The others: percent of the way to (Small) or from (Big) the
    /// vanishing point, 1 to 99.
    pub depth: f64,
    /// The vanishing point, page space, and whether it moves with the
    /// object.
    pub vanishing: Point,
    pub on_object: bool,
    pub color: ExtrudeColor,
    pub solid: Color,
    pub shade_from: Color,
    pub shade_to: Color,
    /// The light: degrees (0 = from the right, counterclockwise) and 0
    /// to 100.
    pub light_angle: f64,
    pub light_intensity: f64,
    /// The turn in space, degrees about x, y and z.
    pub rotation: [f64; 3],
    /// The first light's elevation, degrees.
    pub elevation: f64,
    /// Further lights: on or off, and where they come from.
    pub lights: [(bool, Light); MAX_EXTRA_LIGHTS],
    /// Bevel width on the near cap's edge, mm.
    pub bevel: f64,
}

/// A further light as it starts when switched on.
pub const NEW_LIGHT: Light = Light {
    angle: 45.0,
    elevation: 30.0,
    intensity: 50.0,
};

impl Default for ExtrudeSettings {
    fn default() -> Self {
        ExtrudeSettings {
            kind: ExtrudeKind::SmallBack,
            vector: Vec2::new(6.0, 6.0),
            depth: 20.0,
            vanishing: Point::ZERO,
            on_object: true,
            color: ExtrudeColor::Object,
            solid: Color::rgb8(128, 128, 128),
            shade_from: Color::rgb8(200, 200, 200),
            shade_to: Color::rgb8(40, 40, 40),
            light_angle: 135.0,
            light_intensity: 50.0,
            rotation: [0.0; 3],
            elevation: 0.0,
            lights: [(false, NEW_LIGHT); MAX_EXTRA_LIGHTS],
            bevel: 0.0,
        }
    }
}

fn fin(v: f64, lo: f64, hi: f64, or: f64) -> f64 {
    if v.is_finite() {
        v.clamp(lo, hi)
    } else {
        or
    }
}

/// The largest vector or vanishing point coordinate taken, mm.
const FAR: f64 = 1.0e5;

impl ExtrudeSettings {
    /// The same values in their ranges.
    pub fn clamped(self) -> Self {
        let d = ExtrudeSettings::default();
        ExtrudeSettings {
            vector: Vec2::new(
                fin(self.vector.x, -FAR, FAR, d.vector.x),
                fin(self.vector.y, -FAR, FAR, d.vector.y),
            ),
            depth: fin(self.depth, 1.0, 99.0, d.depth),
            vanishing: Point::new(
                fin(self.vanishing.x, -FAR, FAR, 0.0),
                fin(self.vanishing.y, -FAR, FAR, 0.0),
            ),
            light_angle: fin(self.light_angle, 0.0, 360.0, d.light_angle),
            light_intensity: fin(self.light_intensity, 0.0, 100.0, d.light_intensity),
            rotation: self.rotation.map(|a| fin(a, -360.0, 360.0, 0.0)),
            elevation: fin(self.elevation, 0.0, 90.0, 0.0),
            lights: self.lights.map(|(on, l)| {
                (
                    on,
                    Light {
                        angle: fin(l.angle, 0.0, 360.0, NEW_LIGHT.angle),
                        elevation: fin(l.elevation, 0.0, 90.0, NEW_LIGHT.elevation),
                        intensity: fin(l.intensity, 0.0, 100.0, NEW_LIGHT.intensity),
                    },
                )
            }),
            bevel: fin(self.bevel, 0.0, FAR, 0.0),
            ..self
        }
    }

    /// The turn, elevation and lights as the effect keeps them.
    pub fn space(&self) -> Solid {
        Solid {
            rotation: self.rotation,
            elevation: self.elevation,
            lights: self
                .lights
                .iter()
                .filter(|(on, _)| *on)
                .map(|(_, l)| *l)
                .collect(),
        }
    }

    /// The effect these values give object `id` (its transform to its
    /// parent `local`, its parent's to the page `parent`).
    pub fn effect(&self, local: Affine, parent: Affine) -> Effect {
        let s = self.clamped();
        let vanishing = (!s.kind.parallel()).then(|| {
            let to_parent = parent.inverse() * s.vanishing;
            if s.on_object && local.determinant().abs() > 1e-12 {
                local.inverse() * to_parent
            } else {
                to_parent
            }
        });
        let shade = s.color == ExtrudeColor::Shade;
        Effect::Extrude {
            depth: s.vector,
            vanishing,
            amount: s.depth / 100.0,
            shade_from: shade.then_some(s.shade_from),
            shade_to: shade.then_some(s.shade_to),
            light_angle: s.light_angle,
            light_intensity: s.light_intensity,
            bevel: s.bevel,
            kind: Some(s.kind),
            on_object: s.on_object,
            solid: (s.color == ExtrudeColor::Solid).then_some(s.solid),
            space: s.space(),
        }
    }

    /// Take the values of extrusion `e` on an object with those
    /// transforms.
    pub fn take(&mut self, e: &Effect, local: Affine, parent: Affine) {
        let Effect::Extrude {
            depth,
            vanishing,
            amount,
            shade_from,
            shade_to,
            light_angle,
            light_intensity,
            kind,
            on_object,
            solid,
            bevel,
            space,
        } = e
        else {
            return;
        };
        self.rotation = space.rotation;
        self.elevation = space.elevation;
        self.bevel = *bevel;
        for (i, slot) in self.lights.iter_mut().enumerate() {
            *slot = match space.lights.get(i) {
                Some(l) => (true, *l),
                None => (false, slot.1),
            };
        }
        self.kind = kind.unwrap_or(if vanishing.is_some() {
            ExtrudeKind::SmallBack
        } else {
            ExtrudeKind::BackParallel
        });
        self.vector = *depth;
        if let Some(v) = vanishing {
            self.vanishing = if *on_object {
                parent * local * *v
            } else {
                parent * *v
            };
            self.depth = amount * 100.0;
        }
        self.on_object = *on_object;
        self.color = match (solid, shade_from) {
            (Some(c), _) => {
                self.solid = *c;
                ExtrudeColor::Solid
            }
            (None, Some(f)) => {
                self.shade_from = *f;
                if let Some(t) = shade_to {
                    self.shade_to = *t;
                }
                ExtrudeColor::Shade
            }
            (None, None) => ExtrudeColor::Object,
        };
        self.light_angle = *light_angle;
        self.light_intensity = *light_intensity;
        *self = self.clamped();
    }
}

/// A drag of the Extrude tool under way.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtrudeDrag {
    pub id: ShapeId,
    pub start: Point,
    /// The drag moves the handle of the object's extrusion, drawn from
    /// `anchor` (the object's middle).
    pub handle: bool,
    pub anchor: Point,
}

fn is_extrude(e: &Effect) -> bool {
    matches!(e, Effect::Extrude { .. })
}

/// `effects` with the extrusion replaced in place by `e` (added at the
/// end when there is none; removed with `None`).
pub fn with_extrusion(effects: &[Effect], e: Option<Effect>) -> Vec<Effect> {
    let mut out = Vec::with_capacity(effects.len() + 1);
    let mut placed = false;
    for x in effects {
        if is_extrude(x) {
            if !placed {
                out.extend(e.clone());
                placed = true;
            }
        } else {
            out.push(x.clone());
        }
    }
    if !placed {
        out.extend(e);
    }
    out
}

impl App {
    /// The object's transform to its parent and its parent's to the page.
    fn extrude_frames(&self, id: ShapeId) -> Option<(Affine, Affine)> {
        let s = self.doc().find_shape(id)?;
        let parent = self.doc().parent_transform(id).unwrap_or(Affine::IDENTITY);
        Some((s.transform, parent))
    }

    /// The first selected object's extrusion.
    pub fn selected_extrusion(&self) -> Option<(ShapeId, Effect)> {
        self.selected_shapes().into_iter().find_map(|s| {
            s.effects
                .iter()
                .find(|e| is_extrude(e))
                .map(|e| (s.id, e.clone()))
        })
    }

    /// The bar and the panel show the selected extrusion.
    pub fn sync_extrude_settings(&mut self) {
        if let Some((id, e)) = self.selected_extrusion() {
            if let Some((local, parent)) = self.extrude_frames(id) {
                let mut s = self.extrude;
                s.take(&e, local, parent);
                self.extrude = s;
            }
        }
    }

    /// Every selected extrusion takes the settings, one undo step (a run
    /// of changes with nothing in between is one step).
    pub fn apply_extrude_settings(&mut self) {
        self.extrude = self.extrude.clamped();
        let mut cmds = Vec::new();
        let mut ids = Vec::new();
        for s in self.selected_shapes() {
            if !s.effects.iter().any(is_extrude) {
                continue;
            }
            let Some((local, parent)) = self.extrude_frames(s.id) else {
                continue;
            };
            let effects = with_extrusion(&s.effects, Some(self.extrude.effect(local, parent)));
            if effects == s.effects {
                continue;
            }
            ids.push(s.id);
            cmds.push(Command::SetEffects {
                shape: s.id,
                effects,
            });
        }
        self.run_extrude_step("Extrude", ids, cmds);
    }

    fn run_extrude_step(&mut self, label: &'static str, ids: Vec<ShapeId>, cmds: Vec<Command>) {
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

    /// Apply: the settings' extrusion on the selected objects that have
    /// none (a vanishing point locked to the page at the page's origin
    /// is put up and to the right of each object instead).
    pub fn add_extrusion(&mut self) {
        let mut cmds = Vec::new();
        for s in self.selected_shapes() {
            if s.effects.iter().any(is_extrude)
                || matches!(s.kind, ShapeKind::Bitmap { .. } | ShapeKind::Group { .. })
            {
                continue;
            }
            let Some((local, parent)) = self.extrude_frames(s.id) else {
                continue;
            };
            let mut settings = self.extrude.clamped();
            if !settings.kind.parallel() && settings.vanishing == Point::ZERO {
                let b = self.visual_bounds(&s);
                settings.vanishing = Point::new(b.x1 + b.width(), b.y1 + b.height());
            }
            cmds.push(Command::SetEffects {
                shape: s.id,
                effects: with_extrusion(&s.effects, Some(settings.effect(local, parent))),
            });
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Extrude", &cmds) {
            self.status = e.to_string();
        }
    }

    /// Clear Extrude.
    pub fn clear_extrusion(&mut self) {
        let mut cmds = Vec::new();
        let mut ids = Vec::new();
        for s in self.selected_shapes() {
            if s.effects.iter().any(is_extrude) {
                ids.push(s.id);
                cmds.push(Command::SetEffects {
                    shape: s.id,
                    effects: with_extrusion(&s.effects, None),
                });
            }
        }
        self.run_extrude_step("Clear Extrude", ids, cmds);
    }

    /// The handle of each selected extrusion and the point it is drawn
    /// from (page space): the vanishing point from the object's middle,
    /// or the middle of a parallel body's copy.
    pub fn extrusion_handles(&self) -> Vec<(ShapeId, Point, Point)> {
        let mut out = Vec::new();
        for id in &self.selection {
            let Some(s) = self.doc().find_shape(*id) else {
                continue;
            };
            let Some(e) = s.effects.iter().find(|e| is_extrude(e)) else {
                continue;
            };
            let Some((local, parent)) = self.extrude_frames(*id) else {
                continue;
            };
            let mut st = ExtrudeSettings::default();
            st.take(e, local, parent);
            let Some(page) = self.doc().shape_on_page(*id) else {
                continue;
            };
            let mid = page.bounds().center();
            let handle = if st.kind.parallel() {
                mid + parent.as_coeffs_vec(st.vector)
            } else {
                st.vanishing
            };
            if handle.x.is_finite() && handle.y.is_finite() {
                out.push((*id, handle, mid));
            }
        }
        out
    }

    /// Start a drag at `at` (page space). False when nothing is there.
    pub fn begin_extrude(&mut self, at: Point) -> bool {
        let tol = 6.0 / self.view.zoom.max(1e-6) as f64;
        if let Some((id, h, anchor)) = self
            .extrusion_handles()
            .into_iter()
            .find(|(_, h, _)| (*h - at).hypot() <= tol)
        {
            self.extrude_drag = Some(ExtrudeDrag {
                id,
                start: h,
                handle: true,
                anchor,
            });
            return true;
        }
        let Some(id) = self.hit_test_inside(at) else {
            return false;
        };
        if self
            .doc()
            .find_shape(id)
            .is_none_or(|s| matches!(s.kind, ShapeKind::Bitmap { .. } | ShapeKind::Group { .. }))
        {
            return false;
        }
        self.select(vec![id]);
        self.extrude_drag = Some(ExtrudeDrag {
            id,
            start: at,
            handle: false,
            anchor: at,
        });
        true
    }

    /// The pointer moved to `p` during a drag.
    pub fn drag_extrude(&mut self, p: Point) {
        let Some(dd) = self.extrude_drag else {
            return;
        };
        let Some((local, parent)) = self.extrude_frames(dd.id) else {
            self.extrude_drag = None;
            return;
        };
        if self.selection != vec![dd.id] {
            self.select(vec![dd.id]);
        }
        if dd.handle {
            self.sync_extrude_settings();
        }
        if self.extrude.kind.parallel() {
            // From where the drag started, or (the handle at the copy's
            // middle) from the object's middle.
            let from = if dd.handle { dd.anchor } else { dd.start };
            self.extrude.vector = parent.inverse().as_coeffs_vec(p - from);
        } else {
            self.extrude.vanishing = p;
        }
        let effect = self.extrude.effect(local, parent);
        self.push_effect(effect, true);
    }

    /// The Extrude tool's clicks and drags.
    pub fn extrude_input(&mut self, response: &egui::Response, p: Point) {
        use egui::PointerButton;
        if response.drag_started_by(PointerButton::Primary) {
            let origin = response
                .ctx
                .input(|i| i.pointer.press_origin())
                .map(|o| self.view.to_page(o))
                .unwrap_or(p);
            self.begin_extrude(origin);
        }
        if response.dragged_by(PointerButton::Primary) && self.extrude_drag.is_some() {
            self.drag_extrude(p);
        }
        if response.drag_stopped() {
            self.extrude_drag = None;
        }
        if response.clicked_by(PointerButton::Primary) {
            match self.hit_test_inside(p) {
                Some(id) => self.select(vec![id]),
                None => self.select(Vec::new()),
            }
        }
    }
}

/// A vector through the linear part of an affine map.
trait LinearPart {
    fn as_coeffs_vec(&self, v: Vec2) -> Vec2;
}

impl LinearPart for Affine {
    fn as_coeffs_vec(&self, v: Vec2) -> Vec2 {
        let c = self.as_coeffs();
        Vec2::new(c[0] * v.x + c[2] * v.y, c[1] * v.x + c[3] * v.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use tracedraw_core::{geometry::Rect, Fill};

    fn square(app: &mut App) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                radius: 0.0,
                corners: None,
            })
            .expect("square");
        app.run(Command::SetFill {
            shapes: vec![id],
            fill: Fill::Solid(Color::rgb8(220, 40, 40)),
        });
        id
    }

    fn extrusion(app: &App, id: ShapeId) -> Option<Effect> {
        app.doc()
            .find_shape(id)?
            .effects
            .iter()
            .find(|e| is_extrude(e))
            .cloned()
    }

    #[test]
    fn a_drag_extrudes_toward_the_pointer() {
        let mut app = App::headless();
        app.tool = Tool::Extrude;
        let id = square(&mut app);
        let steps = app.engine.history_labels().0.len();
        // Small back toward where the drag ends, locked to the object.
        assert!(app.begin_extrude(Point::new(10.0, 10.0)));
        app.drag_extrude(Point::new(40.0, 30.0));
        app.drag_extrude(Point::new(60.0, 60.0));
        app.extrude_drag = None;
        assert_eq!(app.engine.history_labels().0.len(), steps + 1);
        let Some(Effect::Extrude {
            vanishing,
            kind,
            on_object,
            amount,
            ..
        }) = extrusion(&app, id)
        else {
            panic!("an extrusion");
        };
        assert_eq!(kind, Some(ExtrudeKind::SmallBack));
        assert!(on_object);
        assert_eq!(vanishing, Some(Point::new(60.0, 60.0)));
        assert!((amount - 0.2).abs() < 1e-9);
        // Moved with the object, the vanishing point goes along.
        app.run(Command::TransformShapes {
            shapes: vec![id],
            transform: Affine::translate((5.0, 0.0)),
        });
        app.sync_extrude_settings();
        assert!((app.extrude.vanishing - Point::new(65.0, 60.0)).hypot() < 1e-9);
        // Its handle is the vanishing point; dragging it moves it.
        let h = app.extrusion_handles();
        assert_eq!(h.len(), 1);
        assert!((h[0].1 - Point::new(65.0, 60.0)).hypot() < 1e-9);
        assert!(app.begin_extrude(Point::new(65.0, 60.0)));
        app.drag_extrude(Point::new(70.0, 20.0));
        app.extrude_drag = None;
        app.sync_extrude_settings();
        assert!((app.extrude.vanishing - Point::new(70.0, 20.0)).hypot() < 1e-9);
        // Parallel: along the drag.
        app.extrude.kind = ExtrudeKind::BackParallel;
        app.apply_extrude_settings();
        assert!(app.begin_extrude(Point::new(12.0, 8.0)));
        app.drag_extrude(Point::new(20.0, 12.0));
        app.extrude_drag = None;
        let Some(Effect::Extrude { depth, kind, .. }) = extrusion(&app, id) else {
            panic!("an extrusion");
        };
        assert_eq!(kind, Some(ExtrudeKind::BackParallel));
        assert!((depth - Vec2::new(8.0, 4.0)).hypot() < 1e-9);
        // The body is drawn: two faces, one object each; the far face
        // lies hidden behind the filled square and them (#1281).
        let s = app.doc().find_shape(id).cloned().expect("square");
        let ev = tracedraw_core::live::evaluate(&s);
        assert_eq!(ev.below.iter().filter(|s| s.fill != Fill::None).count(), 2);
        assert_eq!(ev.below.len(), 2);
    }

    #[test]
    fn the_bar_edits_the_selected_extrusion() {
        let mut app = App::headless();
        let id = square(&mut app);
        app.select(vec![id]);
        app.extrude.kind = ExtrudeKind::BigFront;
        app.add_extrusion();
        let Some(Effect::Extrude { vanishing, .. }) = extrusion(&app, id) else {
            panic!("an extrusion");
        };
        // No vanishing point yet: up and to the right of the object.
        assert_eq!(vanishing, Some(Point::new(40.0, 40.0)));
        app.sync_extrude_settings();
        app.extrude.color = ExtrudeColor::Shade;
        app.extrude.depth = 50.0;
        app.extrude.light_intensity = 0.0;
        app.apply_extrude_settings();
        app.extrude.light_angle = 45.0;
        app.apply_extrude_settings();
        let labels = app.engine.history_labels().0;
        assert_eq!(
            labels.iter().filter(|l| **l == "Extrude").count(),
            2,
            "{labels:?}"
        );
        let Some(Effect::Extrude {
            shade_from,
            amount,
            light_angle,
            ..
        }) = extrusion(&app, id)
        else {
            panic!("an extrusion");
        };
        assert!(shade_from.is_some());
        assert!((amount - 0.5).abs() < 1e-9);
        assert_eq!(light_angle, 45.0);
        // A front body draws over the object.
        let s = app.doc().find_shape(id).cloned().expect("square");
        let ev = tracedraw_core::live::evaluate(&s);
        assert!(ev.below.is_empty() && !ev.above.is_empty());
        app.clear_extrusion();
        assert!(extrusion(&app, id).is_none());
    }

    #[test]
    fn settings_read_older_effects_and_stay_in_range() {
        // An older effect: a vanishing point on the page, no kind.
        let old = Effect::Extrude {
            depth: Vec2::new(8.0, -8.0),
            vanishing: Some(Point::new(100.0, 50.0)),
            amount: 0.3,
            shade_from: Some(Color::WHITE),
            shade_to: None,
            light_angle: 90.0,
            light_intensity: 20.0,
            bevel: 0.0,
            kind: None,
            on_object: false,
            solid: None,
            space: Default::default(),
        };
        let mut s = ExtrudeSettings::default();
        s.take(&old, Affine::translate((3.0, 0.0)), Affine::IDENTITY);
        assert_eq!(s.kind, ExtrudeKind::SmallBack);
        assert_eq!(s.vanishing, Point::new(100.0, 50.0));
        assert!((s.depth - 30.0).abs() < 1e-9);
        assert_eq!(s.color, ExtrudeColor::Shade);
        let odd = ExtrudeSettings {
            depth: 500.0,
            light_angle: f64::NAN,
            vector: Vec2::new(f64::INFINITY, 1.0),
            ..Default::default()
        }
        .clamped();
        assert_eq!(odd.depth, 99.0);
        assert_eq!(odd.light_angle, 135.0);
        assert_eq!(odd.vector, Vec2::new(6.0, 1.0));
        let json = serde_json::to_string(&s).expect("json");
        let back: ExtrudeSettings = serde_json::from_str(&json).expect("read back");
        assert_eq!(back, s);
    }
}
