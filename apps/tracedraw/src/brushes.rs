//! Strokes of the shaping brushes (Smooth, Smear, Twirl, Attract and
//! Repel, Smudge, Roughen): a drag over the selected objects reshapes the
//! outline under the nib, measured on the page, step by step, the whole
//! stroke one undo step. With nothing selected a drag does nothing (a
//! click selects). Objects that are not curves become curves when the nib
//! first reaches them; groups are reshaped through the objects inside
//! them. The settings come from the property bar (`ui::brush_bars`).

use crate::app::{App, Drag};
use crate::tools::Tool;
use crate::ui::brush_bars::BrushSettings;
use egui::{Modifiers, PointerButton, Response};
use tracedraw_core::geometry::Affine;
use tracedraw_core::{effects, geometry::Point, Command, Shape, ShapeKind};

/// How far Twirl and Attract and Repel go per second while the button is
/// held still, as a pointer move (mm).
pub const HOLD_MM_PER_S: f64 = 10.0;

/// The objects a brush can reshape among `shapes` (each with its
/// transform to the page): curves and objects that can become curves;
/// a group stands for the objects inside it, at any depth.
fn brush_targets(shapes: Vec<Shape>) -> Vec<Shape> {
    fn walk(s: Shape, out: &mut Vec<Shape>, depth: usize) {
        match &s.kind {
            ShapeKind::Group { children } if depth < 32 => {
                for c in children {
                    let mut c = c.clone();
                    c.transform = s.transform * c.transform;
                    walk(c, out, depth + 1);
                }
            }
            ShapeKind::Group { .. }
            | ShapeKind::ClipFrame { .. }
            | ShapeKind::Bitmap { .. }
            | ShapeKind::Table(_)
            | ShapeKind::SymbolInstance { .. } => {}
            _ => out.push(s),
        }
    }
    let mut out = Vec::new();
    for s in shapes {
        walk(s, &mut out, 0);
    }
    out
}

impl App {
    /// A stroke begins at `p`. Brushes change only what is selected: with
    /// nothing selected the stroke does nothing and the status bar says
    /// to select an object first.
    pub fn brush_begin(&mut self, p: Point) {
        self.brush_stepped = false;
        self.brush_stroke.clear();
        self.brush_travel = 0.0;
        if self.selection.is_empty() {
            self.status = crate::i18n::tr("status.brush_select_first");
            self.drag = Drag::None;
            return;
        }
        self.drag = Drag::Node { last: p };
    }

    /// The stroke moves on to `p`, `dt` seconds after the last step;
    /// `swap` (Alt or Shift held) swaps Attract and Repel and Twirl's
    /// turn. Twirl and Attract and Repel go on while the button is held
    /// still, as if the pointer moved [`HOLD_MM_PER_S`].
    ///
    /// Each selected object the nib reaches (the objects inside a selected
    /// group too) is reshaped where the nib is, measured on the page; an
    /// object that is not a curve becomes one when the nib first reaches
    /// it, in the same undo step as the stroke.
    pub fn brush_step(&mut self, p: Point, swap: bool, dt: f64) {
        let Drag::Node { last } = self.drag else {
            return;
        };
        let delta = p - last;
        let radius = self.brush_radius;
        let b = self.brush;
        let held = if dt.is_finite() {
            dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        let motion = delta.hypot() + HOLD_MM_PER_S * held;
        let turn = if swap { -1.0 } else { 1.0 };
        let travelled = self.brush_travel;
        let mut changed = false;
        for s in brush_targets(self.selected_shapes()) {
            // Quick test: the nib does not come near the object.
            let page_bounds = s.bounds().inflate(radius, radius);
            if !page_bounds.contains(p) && !page_bounds.contains(last) {
                continue;
            }
            // The outline so far in this stroke, else the object's own.
            let (path, closed) = match self.brush_stroke.iter().find(|(id, _)| *id == s.id) {
                Some((_, ShapeKind::Path { path, closed })) => (path.clone(), *closed),
                _ => match &s.kind {
                    ShapeKind::Path { path, closed } => (path.clone(), *closed),
                    _ => (s.local_path(), true),
                },
            };
            let to_page: Affine = s.transform;
            let new_path = match self.tool {
                // The outline under the nib's last place follows the
                // pointer.
                Tool::Smear => effects::smear(
                    &path,
                    to_page,
                    last,
                    delta * BrushSettings::pressure(b.smear_pressure),
                    radius,
                ),
                Tool::Twirl => {
                    effects::twirl(&path, to_page, p, motion * b.twirl_rate() * turn, radius)
                }
                Tool::AttractRepel => {
                    effects::attract(&path, to_page, p, radius, motion * b.attract_rate(swap))
                }
                Tool::Smudge => effects::smudge(
                    &path,
                    to_page,
                    last,
                    delta,
                    radius,
                    travelled,
                    BrushSettings::pressure(b.smudge_pressure),
                ),
                Tool::Roughen => effects::roughen(
                    &path,
                    to_page,
                    p,
                    radius,
                    self.roughen_amount,
                    b.roughen_spacing(),
                ),
                Tool::Smooth => effects::smooth(&path, to_page, p, radius, b.smooth_tolerance()),
                _ => return,
            };
            if new_path == path {
                continue;
            }
            changed = true;
            let kind = ShapeKind::Path {
                path: new_path,
                closed,
            };
            match self.brush_stroke.iter_mut().find(|(id, _)| *id == s.id) {
                Some(entry) => entry.1 = kind,
                None => self.brush_stroke.push((s.id, kind)),
            }
        }
        self.drag = Drag::Node { last: p };
        self.brush_travel += delta.hypot();
        if !changed {
            return;
        }
        let label = match self.tool {
            Tool::Smear => "Smear",
            Tool::Twirl => "Twirl",
            Tool::AttractRepel => "Attract",
            Tool::Smudge => "Smudge",
            Tool::Roughen => "Roughen",
            _ => "Smooth",
        };
        let cmds: Vec<Command> = self
            .brush_stroke
            .iter()
            .map(|(id, kind)| Command::SetShapeKind {
                shape: *id,
                kind: kind.clone(),
            })
            .collect();
        // One undo step for the whole stroke: the last step's batch goes
        // and one with every object the stroke changed takes its place.
        if self.brush_stepped && self.engine.undo_label() == Some(label) {
            let _ = self.engine.undo();
        }
        match self.engine.run_batch(label, &cmds) {
            Ok(_) => self.brush_stepped = true,
            Err(e) => self.status = e.to_string(),
        }
    }

    /// The brushes' pointer input. A click (no drag) selects, see
    /// `Tool::click_selects`.
    pub fn brush_input(&mut self, response: &Response, p: Point, mods: Modifiers) {
        if response.drag_started_by(PointerButton::Primary) {
            self.brush_begin(p);
        }
        if response.dragged_by(PointerButton::Primary) {
            let held = matches!(self.tool, Tool::Twirl | Tool::AttractRepel);
            let dt = if held {
                // Keep going while the button is held still.
                response.ctx.request_repaint();
                response.ctx.input(|i| i.stable_dt) as f64
            } else {
                0.0
            };
            self.brush_step(p, mods.alt || mods.shift, dt);
        }
        if response.drag_stopped() {
            self.brush_stroke.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Shape as _;
    use tracedraw_core::{Rect, ShapeId};

    fn square(app: &mut App) -> ShapeId {
        let id = app
            .new_shape(ShapeKind::Rect {
                rect: Rect::new(0.0, 0.0, 40.0, 40.0),
                radius: 0.0,
                corners: None,
            })
            .expect("shape");
        app.select(vec![id]);
        id
    }

    fn bounds(app: &App, id: ShapeId) -> Rect {
        app.doc().shape_on_page(id).expect("shape").bounds()
    }

    /// A stroke is one undo step whatever its length; the next stroke is
    /// another. Smear's pressure scales how far the curve follows.
    #[test]
    fn a_stroke_is_one_step_and_follows_the_settings() {
        let mut app = App::headless();
        let id = square(&mut app);
        app.tool = Tool::Smear;
        app.brush_radius = 10.0;
        app.brush_begin(Point::new(40.0, 20.0));
        let depth = app.engine.history_labels().0.len();
        for x in [42.0, 44.0, 46.0] {
            app.brush_step(Point::new(x, 20.0), false, 0.0);
        }
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let full = bounds(&app, id).x1;
        assert!(full > 42.5, "{full}");
        app.undo();
        // Half the pressure: half as far.
        app.brush.smear_pressure = 50;
        app.brush_begin(Point::new(40.0, 20.0));
        for x in [42.0, 44.0, 46.0] {
            app.brush_step(Point::new(x, 20.0), false, 0.0);
        }
        let half = bounds(&app, id).x1;
        assert!(half > 40.5 && half < full - 0.5, "{half} vs {full}");
    }

    /// Attract pulls and Repel pushes; Shift swaps them. Twirl turns the
    /// other way when set clockwise.
    #[test]
    fn attract_repel_and_twirl_follow_their_modes() {
        let mut app = App::headless();
        let id = square(&mut app);
        app.tool = Tool::AttractRepel;
        app.brush_radius = 8.0;
        // How far out the corner by the nib, (40, 40), now is (x + y).
        let stroke = |app: &mut App, shift: bool| {
            app.brush_begin(Point::new(36.0, 36.0));
            app.brush_step(Point::new(37.0, 36.0), shift, 0.0);
            app.brush_step(Point::new(36.0, 36.0), shift, 0.0);
            let reach = match &app.doc().find_shape(id).expect("shape").kind {
                ShapeKind::Path { path, .. } => path
                    .elements()
                    .iter()
                    .filter_map(|e| match e {
                        tracedraw_core::geometry::PathEl::MoveTo(q)
                        | tracedraw_core::geometry::PathEl::LineTo(q) => Some(q.x + q.y),
                        _ => None,
                    })
                    .fold(f64::MIN, f64::max),
                _ => 0.0,
            };
            app.undo();
            reach
        };
        assert!(stroke(&mut app, false) < 80.0);
        assert!(stroke(&mut app, true) > 80.0);
        app.brush.repel = true;
        assert!(stroke(&mut app, false) > 80.0);
        assert!(stroke(&mut app, true) < 80.0);
        // Twirl near the top right corner: the corner goes up one way and
        // across the other.
        app.tool = Tool::Twirl;
        app.brush_radius = 15.0;
        let twirl = |app: &mut App| {
            app.brush_begin(Point::new(35.0, 35.0));
            for x in [36.0, 38.0, 40.0, 42.0] {
                app.brush_step(Point::new(x, 35.0), false, 0.0);
            }
            let b = bounds(app, id);
            app.undo();
            b
        };
        let ccw = twirl(&mut app);
        app.brush.twirl_clockwise = true;
        let cw = twirl(&mut app);
        assert_ne!(ccw, cw);
        // Held still, Twirl still turns.
        let before = bounds(&app, id);
        app.brush_begin(Point::new(35.0, 35.0));
        for _ in 0..5 {
            app.brush_step(Point::new(35.0, 35.0), false, 0.1);
        }
        assert_ne!(bounds(&app, id), before);
    }

    fn kind(app: &App, id: ShapeId) -> ShapeKind {
        app.doc().find_shape(id).expect("shape").kind.clone()
    }

    fn rect(app: &mut App, r: Rect) -> ShapeId {
        app.new_shape(ShapeKind::Rect {
            rect: r,
            radius: 0.0,
            corners: None,
        })
        .expect("shape")
    }

    /// With nothing selected a drag changes nothing and says to select an
    /// object first; an object that is not selected is never changed,
    /// even when the stroke crosses it.
    #[test]
    fn brushes_change_only_what_is_selected() {
        let mut app = App::headless();
        let a = rect(&mut app, Rect::new(0.0, 0.0, 40.0, 40.0));
        let b = rect(&mut app, Rect::new(50.0, 0.0, 90.0, 40.0));
        app.select(Vec::new());
        let (ka, kb) = (kind(&app, a), kind(&app, b));
        for tool in [
            Tool::Smear,
            Tool::Twirl,
            Tool::AttractRepel,
            Tool::Smudge,
            Tool::Roughen,
            Tool::Smooth,
        ] {
            app.tool = tool;
            let depth = app.engine.history_labels().0.len();
            app.brush_begin(Point::new(40.0, 20.0));
            for x in [42.0, 46.0, 50.0, 54.0] {
                app.brush_step(Point::new(x, 20.0), false, 0.05);
            }
            assert_eq!(kind(&app, a), ka, "{tool:?}");
            assert_eq!(kind(&app, b), kb, "{tool:?}");
            assert_eq!(app.engine.history_labels().0.len(), depth);
            assert_eq!(app.status, crate::i18n::tr("status.brush_select_first"));
        }
        // Only `a` selected: the stroke over both changes `a` alone.
        app.select(vec![a]);
        app.tool = Tool::Smear;
        app.brush_begin(Point::new(40.0, 20.0));
        for x in [44.0, 48.0, 52.0, 56.0] {
            app.brush_step(Point::new(x, 20.0), false, 0.0);
        }
        assert_ne!(kind(&app, a), ka);
        assert_eq!(kind(&app, b), kb);
    }

    /// The conversion to curves and the stroke are one undo step, and an
    /// object the nib left earlier in the stroke keeps its change.
    #[test]
    fn one_undo_step_with_the_conversion_and_every_object_reached() {
        let mut app = App::headless();
        let a = rect(&mut app, Rect::new(0.0, 0.0, 40.0, 40.0));
        let b = rect(&mut app, Rect::new(0.0, 60.0, 40.0, 100.0));
        app.select(vec![a, b]);
        let (ka, kb) = (kind(&app, a), kind(&app, b));
        app.tool = Tool::Smear;
        app.brush_radius = 6.0;
        let depth = app.engine.history_labels().0.len();
        // Down the right side of `a`, then on to `b`.
        app.brush_begin(Point::new(40.0, 20.0));
        app.brush_step(Point::new(43.0, 20.0), false, 0.0);
        app.brush_step(Point::new(43.0, 80.0), false, 0.0);
        app.brush_step(Point::new(46.0, 80.0), false, 0.0);
        assert!(
            matches!(kind(&app, a), ShapeKind::Path { .. }),
            "a kept its change"
        );
        assert!(matches!(kind(&app, b), ShapeKind::Path { .. }));
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        app.undo();
        assert_eq!(kind(&app, a), ka);
        assert_eq!(kind(&app, b), kb);
    }

    /// A group is reshaped through the objects inside it.
    #[test]
    fn a_brush_on_a_group_reshapes_its_objects() {
        let mut app = App::headless();
        let a = rect(&mut app, Rect::new(0.0, 0.0, 40.0, 40.0));
        let b = rect(&mut app, Rect::new(100.0, 0.0, 140.0, 40.0));
        app.select(vec![a, b]);
        app.group_selection();
        assert_eq!(app.selection.len(), 1);
        let (ka, kb) = (kind(&app, a), kind(&app, b));
        app.tool = Tool::Smear;
        app.brush_begin(Point::new(40.0, 20.0));
        for x in [43.0, 46.0] {
            app.brush_step(Point::new(x, 20.0), false, 0.0);
        }
        assert_ne!(kind(&app, a), ka, "the object under the nib changed");
        assert_eq!(kind(&app, b), kb, "the one out of reach did not");
        assert!(bounds(&app, a).x1 > 41.0);
    }

    /// On an object scaled three times wide, only the outline inside the
    /// nib circle on the page moves.
    #[test]
    fn the_nib_matches_the_circle_on_screen_on_scaled_objects() {
        let mut app = App::headless();
        let id = rect(&mut app, Rect::new(0.0, 0.0, 20.0, 20.0));
        app.select(vec![id]);
        app.run(Command::TransformShapes {
            shapes: vec![id],
            transform: Affine::scale_non_uniform(3.0, 1.0),
        });
        app.tool = Tool::Smear;
        app.brush_radius = 5.0;
        app.brush_begin(Point::new(30.0, 20.0));
        app.brush_step(Point::new(30.0, 24.0), false, 0.0);
        let s = app.doc().shape_on_page(id).expect("shape");
        let page = s.page_path();
        for q in effects::resample(&page, 600) {
            if q.y > 20.0 + 1e-6 {
                assert!((q.x - 30.0).abs() < 5.0 + 1e-6, "{q:?}");
            }
        }
        assert!(page.bounding_box().y1 > 22.0);
    }

    /// Smudge out of a shape makes it bigger, into it smaller.
    #[test]
    fn smudge_pulls_extensions_out_and_dents_in() {
        let mut app = App::headless();
        let id = square(&mut app);
        app.tool = Tool::Smudge;
        app.brush_radius = 6.0;
        let area = |app: &App| {
            let p = app.doc().shape_on_page(id).expect("shape").page_path();
            let pts = effects::resample(&p, 800);
            let n = pts.len();
            (0..n)
                .map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y)
                .sum::<f64>()
                .abs()
                / 2.0
        };
        let a0 = area(&app);
        app.brush_begin(Point::new(38.0, 20.0));
        for k in 1..=6 {
            app.brush_step(Point::new(38.0 + 2.0 * k as f64, 20.0), false, 0.0);
        }
        assert!(area(&app) > a0 + 5.0);
        app.undo();
        app.brush_begin(Point::new(42.0, 20.0));
        for k in 1..=6 {
            app.brush_step(Point::new(42.0 - 2.0 * k as f64, 20.0), false, 0.0);
        }
        assert!(area(&app) < a0 - 5.0);
    }
}
