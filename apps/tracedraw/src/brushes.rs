//! Strokes of the shaping brushes (Smooth, Smear, Twirl, Attract and
//! Repel, Smudge, Roughen): a drag over the selected objects reshapes the
//! curves under the nib, step by step, the whole stroke one undo step.
//! Brushes change only what is selected: with nothing selected a drag
//! does nothing and a click selects (#271). Objects that are
//! not curves are converted first. The settings come from the property bar
//! (`ui::brush_bars`).

use crate::app::{App, Drag};
use crate::tools::Tool;
use crate::ui::brush_bars::BrushSettings;
use egui::{Modifiers, PointerButton, Response};
use tracedraw_core::{effects, geometry::Point, Command, ShapeKind};

/// How far Twirl and Attract and Repel go per second while the button is
/// held still, as a pointer move (mm).
pub const HOLD_MM_PER_S: f64 = 10.0;

impl App {
    /// A stroke begins at `p`. Brushes change only what is selected:
    /// with nothing selected the stroke does nothing and the status bar
    /// says to select an object first (a click selects, see
    /// [`crate::tools::Tool::click_selects`]).
    pub fn brush_begin(&mut self, p: Point) {
        self.brush_stepped = false;
        if self.selection.is_empty() {
            self.status = crate::i18n::tr("status.brush_select_first");
            self.drag = Drag::None;
            return;
        }
        // Brushes work on curves. A group or clip frame is never brushed,
        // so it keeps its envelope and objects as they are.
        let shapes = self
            .selected_shapes()
            .into_iter()
            .filter(|s| {
                !matches!(
                    s.kind,
                    ShapeKind::Group { .. } | ShapeKind::ClipFrame { .. }
                )
            })
            .collect();
        self.convert_shapes_to_curves(shapes, true);
        self.drag = Drag::Node { last: p };
        self.brush_stepped = false;
    }

    /// The stroke moves on to `p`, `dt` seconds after the last step;
    /// `swap` (Alt or Shift held) swaps Attract and Repel and Twirl's
    /// turn. Twirl and Attract and Repel go on while the button is held
    /// still, as if the pointer moved [`HOLD_MM_PER_S`].
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
        let mut cmds = Vec::new();
        for s in self.selected_shapes() {
            let ShapeKind::Path { path, closed } = &s.kind else {
                continue;
            };
            // The nib, its reach and the settings' millimetres are page
            // measures, so the brush works on the outline as it lies on
            // the page and the result is taken back into the object's own
            // space; a scaled object (text converted to curves, a resized
            // drawing) is reshaped within the nib as drawn, not within a
            // nib scaled with it.
            let page_path = s.transform * path;
            let new_page = match self.tool {
                Tool::Smear => effects::smear(
                    &page_path,
                    p,
                    delta * BrushSettings::pressure(b.smear_pressure),
                    radius,
                ),
                Tool::Twirl => {
                    effects::twirl(&page_path, p, motion * b.twirl_rate() * turn, radius)
                }
                Tool::AttractRepel => {
                    effects::attract(&page_path, p, radius, motion * b.attract_rate(swap))
                }
                Tool::Smudge => effects::smear(
                    &effects::smooth(&page_path, p, radius, 0.2),
                    p,
                    delta * BrushSettings::pressure(b.smudge_pressure),
                    radius,
                ),
                Tool::Roughen => effects::roughen(
                    &page_path,
                    p,
                    radius,
                    self.roughen_amount,
                    b.roughen_spacing(),
                ),
                Tool::Smooth => effects::smooth(&page_path, p, radius, b.smooth_tolerance()),
                _ => continue,
            };
            if new_page == page_path {
                continue;
            }
            let new_path = s.transform.inverse() * new_page;
            cmds.push(Command::SetShapeKind {
                shape: s.id,
                kind: ShapeKind::Path {
                    path: new_path,
                    closed: *closed,
                },
            });
        }
        self.drag = Drag::Node { last: p };
        if cmds.is_empty() {
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
        // One undo step for the whole stroke.
        if self.brush_stepped && self.engine.undo_label() == Some(label) {
            let _ = self.engine.undo();
        }
        if self.engine.run_batch(label, &cmds).is_ok() {
            self.brush_stepped = true;
        }
    }

    /// The brushes' pointer input.
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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

    /// Brushes change only the selected objects: a stroke over a
    /// selected square and an unselected one beside it reshapes the
    /// selected one alone, and with nothing selected a stroke changes
    /// nothing and says to select an object first (#271).
    #[test]
    fn brushes_change_only_the_selected_objects() {
        for tool in [Tool::Smear, Tool::Roughen, Tool::Smudge, Tool::Twirl] {
            let mut app = App::headless();
            let a = square(&mut app);
            let b = app
                .new_shape(ShapeKind::Rect {
                    rect: Rect::new(40.0, 0.0, 80.0, 40.0),
                    radius: 0.0,
                    corners: None,
                })
                .expect("shape");
            app.select(vec![a]);
            app.tool = tool;
            app.brush_radius = 10.0;
            let kind_b = app.doc().find_shape(b).expect("b").kind.clone();
            // Along the edge both squares share.
            app.brush_begin(Point::new(40.0, 5.0));
            for y in [10.0, 15.0, 20.0, 25.0, 30.0] {
                app.brush_step(Point::new(40.0 + y / 10.0, y), false, 0.05);
            }
            assert!(
                matches!(
                    app.doc().find_shape(a).expect("a").kind,
                    ShapeKind::Path { .. }
                ),
                "{tool:?}: the selected square is brushed"
            );
            assert_eq!(
                app.doc().find_shape(b).expect("b").kind,
                kind_b,
                "{tool:?}: the other square is left alone"
            );
            // Nothing selected: nothing changes.
            app.select(Vec::new());
            let doc = app.doc().clone();
            app.brush_begin(Point::new(60.0, 5.0));
            for y in [10.0, 20.0, 30.0] {
                app.brush_step(Point::new(62.0, y), false, 0.05);
            }
            assert_eq!(app.doc().pages, doc.pages, "{tool:?}");
            assert!(app.selection.is_empty());
            assert_eq!(app.status, crate::i18n::tr("status.brush_select_first"));
        }
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
}

#[cfg(test)]
mod page_space_tests {
    use super::*;
    use tracedraw_core::geometry::Affine;
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

    /// Points along the object's outline on the page, half a millimetre
    /// apart at most (a straight side gives points all along it).
    fn page_points(app: &App, id: ShapeId) -> Vec<Point> {
        let s = app.doc().shape_on_page(id).expect("shape");
        let ShapeKind::Path { path, .. } = &s.kind else {
            panic!("not a curve");
        };
        let mut verts = Vec::new();
        tracedraw_core::geometry::flatten(&(s.transform * path), 0.01, &mut |el| match el {
            tracedraw_core::geometry::PathEl::MoveTo(q)
            | tracedraw_core::geometry::PathEl::LineTo(q) => verts.push(q),
            _ => {}
        });
        let mut pts = Vec::new();
        for w in verts.windows(2) {
            let n = ((w[1] - w[0]).hypot() / 0.5).ceil().max(1.0) as usize;
            for k in 0..n {
                pts.push(w[0].lerp(w[1], k as f64 / n as f64));
            }
        }
        pts.extend(verts.last().copied());
        pts
    }

    /// The nib is measured on the page: on a square scaled five times, a
    /// smear with a 10 mm nib at the middle of the right side moves the
    /// side within 10 mm of the pointer, not within 50 mm, and by the
    /// pointer's move, not by five times it.
    #[test]
    fn the_nib_is_measured_on_the_page_whatever_the_object_scale() {
        let mut app = App::headless();
        let id = square(&mut app);
        app.transform_selection(Affine::scale(5.0));
        app.tool = Tool::Smear;
        app.brush_radius = 10.0;
        app.brush_begin(Point::new(200.0, 100.0));
        app.brush_step(Point::new(202.0, 100.0), false, 0.0);
        let pts = page_points(&app, id);
        // The side well away from the nib (and from the corners, which
        // the refit rounds a little) is where it was.
        let far = pts
            .iter()
            .filter(|q| (5.0..=80.0).contains(&q.y) || (120.0..=195.0).contains(&q.y))
            .map(|q| q.x)
            .fold(0.0, f64::max);
        assert!((far - 200.0).abs() < 1e-6, "far side moved to {far}");
        // What the new curve was fitted through stays on the old side
        // just past the nib.
        let edge = pts
            .iter()
            .filter(|q| !(85.0..=115.0).contains(&q.y))
            .map(|q| q.x)
            .fold(0.0, f64::max);
        assert!(edge < 200.5, "edge of the nib at {edge}");
        let near = pts.iter().map(|q| q.x).fold(0.0, f64::max);
        assert!(near > 201.0 && near < 202.5, "near side at {near}");
    }

    /// Smooth's tolerance is in page millimetres too: a stroke over a
    /// scaled, jagged outline leaves the part outside the nib exactly as
    /// it was.
    #[test]
    fn smooth_leaves_the_outline_outside_the_nib_alone() {
        let mut app = App::headless();
        let id = square(&mut app);
        app.transform_selection(Affine::scale(5.0));
        app.tool = Tool::Smooth;
        app.brush_radius = 10.0;
        app.brush_begin(Point::new(200.0, 100.0));
        app.brush_step(Point::new(201.0, 100.0), false, 0.0);
        let after = page_points(&app, id);
        // The corners and the far sides are still where they were.
        for q in [
            Point::new(0.0, 0.0),
            Point::new(200.0, 0.0),
            Point::new(200.0, 200.0),
            Point::new(0.0, 200.0),
            Point::new(200.0, 60.0),
            Point::new(200.0, 140.0),
        ] {
            let near = |pts: &[Point]| {
                pts.iter()
                    .map(|r| (*r - q).hypot())
                    .fold(f64::INFINITY, f64::min)
            };
            assert!(near(&after) < 1e-3, "{q:?} left by {}", near(&after));
        }
    }
}
