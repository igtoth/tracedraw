//! Pen pressure from a graphics tablet (#850).
//!
//! The windowing layer reports a stylus as a touch with a force: egui's
//! `Event::Touch` carries `force` in 0..1 where the platform gives one
//! (Windows pens today; see `docs/behavior/pen-pressure.md` for the other
//! platforms). The pointer events a pen also sends drive the tools as a
//! mouse would, so a pen with no pressure data draws exactly like the
//! mouse: constant width.
//!
//! This module keeps the live pressure between frames and turns the
//! pressures recorded along a freehand drag into width nodes of the
//! stroke's outline.

use std::collections::HashSet;

use tracedraw_core::geometry::Point;
use tracedraw_core::style::{Stroke, WidthNode};

/// The pen's touches, kept between frames in the context's memory: the
/// pressure they carried last, and the touches that began with a start
/// (a finger on a touch screen, a pen on Windows).
#[derive(Debug, Clone, Default)]
struct PenInput {
    pressure: Option<f32>,
    started: HashSet<(u64, u64)>,
}

fn pen_input_id() -> egui::Id {
    egui::Id::new("tracedraw-pen-input")
}

/// Read the pen's pressure from the touch events of a frame's raw input,
/// before egui sees them (the app's raw input hook), and take out the
/// touches that only ever move: a pen on X11 and macOS (see
/// `vendor/README.md`). egui counts any touch as a touch screen for the
/// rest of the session (drag-scrolled lists, windows dragged anywhere, no
/// text selection by dragging), and the pen already moves the pointer and
/// presses its button. A touch that started passes as it came, so a real
/// touch screen still reaches egui. A frame without touches costs one
/// scan of its events.
pub fn filter_raw_input(ctx: &egui::Context, raw: &mut egui::RawInput) {
    if !raw
        .events
        .iter()
        .any(|e| matches!(e, egui::Event::Touch { .. }))
    {
        return;
    }
    ctx.data_mut(|d| {
        let state = d.get_temp_mut_or_default::<PenInput>(pen_input_id());
        state.pressure = pressure_after(&raw.events, state.pressure);
        raw.events.retain(|e| match e {
            egui::Event::Touch {
                device_id,
                id,
                phase,
                ..
            } => {
                let key = (device_id.0, id.0);
                match phase {
                    egui::TouchPhase::Start => {
                        state.started.insert(key);
                        true
                    }
                    egui::TouchPhase::Move => state.started.contains(&key),
                    egui::TouchPhase::End | egui::TouchPhase::Cancel => state.started.remove(&key),
                }
            }
            _ => true,
        });
    });
}

/// The pen's pressure as of the last frame's touch events (see
/// `filter_raw_input`), 0 to 1; `None` with the mouse.
pub fn pressure(ctx: &egui::Context) -> Option<f32> {
    ctx.data(|d| d.get_temp::<PenInput>(pen_input_id()))
        .and_then(|s| s.pressure)
}

/// The pen pressure after `events`, starting from `current`: a touch
/// that begins or moves with a force sets it, a touch that ends or is
/// cancelled clears it. A touch without a force (a finger on a screen
/// that cannot measure it, a pen hovering) clears it too, so the width
/// never follows stale data.
pub fn pressure_after(events: &[egui::Event], current: Option<f32>) -> Option<f32> {
    let mut pressure = current;
    for e in events {
        if let egui::Event::Touch { phase, force, .. } = e {
            pressure = match phase {
                egui::TouchPhase::Start | egui::TouchPhase::Move => {
                    force.filter(|f| f.is_finite()).map(|f| f.clamp(0.0, 1.0))
                }
                egui::TouchPhase::End | egui::TouchPhase::Cancel => None,
            };
        }
    }
    pressure
}

/// The pressures recorded along a freehand drag, one per point. A drag
/// that starts without pressure (the mouse) records nothing. Once a drag
/// has pressure, a point that comes without a reading (a frame with no
/// pen event, the pen lifting off) repeats the last one, so a single
/// missing reading never takes the pressure off the whole stroke.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PenStroke {
    pressures: Vec<f32>,
}

impl PenStroke {
    /// A drag begins at its first point with `pressure`.
    pub fn start(pressure: Option<f32>) -> Self {
        PenStroke {
            pressures: pressure.into_iter().collect(),
        }
    }

    /// Another point was added to the drag, which now has `points`
    /// points, with `pressure` at it (the last reading when `None`).
    pub fn record(&mut self, points: usize, pressure: Option<f32>) {
        let Some(&last) = self.pressures.last() else {
            return;
        };
        if self.pressures.len() + 1 == points {
            self.pressures.push(pressure.unwrap_or(last));
        } else {
            // Points the record missed: it no longer matches the drag.
            self.pressures.clear();
        }
    }

    /// The pressures, when every one of the drag's `points` has one.
    pub fn pressures(&self, points: usize) -> Option<&[f32]> {
        (points >= 2 && self.pressures.len() == points).then_some(self.pressures.as_slice())
    }
}

/// Pressure along a drawn polyline, by distance along it, for sampling at
/// the places of a stroke's width nodes.
pub struct PressureProfile {
    /// Each sample's place along the polyline, 0 (start) to 1 (end), and
    /// its pressure; the places never decrease.
    samples: Vec<(f64, f64)>,
}

impl PressureProfile {
    /// `pressures` go with `points`, one each; `None` when they do not
    /// match or the line has no length.
    pub fn new(points: &[Point], pressures: &[f32]) -> Option<Self> {
        if points.len() != pressures.len() || points.len() < 2 {
            return None;
        }
        let mut cum = Vec::with_capacity(points.len());
        let mut total = 0.0;
        cum.push(0.0);
        for w in points.windows(2) {
            total += (w[1] - w[0]).hypot();
            cum.push(total);
        }
        if total.is_nan() || total <= 1e-9 {
            return None;
        }
        let samples = cum
            .iter()
            .zip(pressures)
            .map(|(c, p)| (c / total, f64::from(p.clamp(0.0, 1.0))))
            .collect();
        Some(PressureProfile { samples })
    }

    /// The pressure at `t` (0 to 1 along the line): the mean of the
    /// samples within `window` of it, so the jitter of a pen's readings
    /// does not reach the outline; the nearest sample when none is that
    /// close.
    pub fn at(&self, t: f64, window: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        let lo = self.samples.partition_point(|(c, _)| *c < t - window);
        let hi = self.samples.partition_point(|(c, _)| *c <= t + window);
        if hi > lo {
            let sum: f64 = self.samples[lo..hi].iter().map(|(_, p)| *p).sum();
            return sum / (hi - lo) as f64;
        }
        // The nearest sample on either side.
        let after = self.samples.get(lo);
        let before = lo.checked_sub(1).and_then(|i| self.samples.get(i));
        match (before, after) {
            (Some(b), Some(a)) if (t - b.0).abs() <= (a.0 - t).abs() => b.1,
            (_, Some(a)) => a.1,
            (Some(b), None) => b.1,
            (None, None) => 1.0,
        }
    }

    /// `nodes + 1` width nodes evenly along the line, each as wide as
    /// `width` scaled by its pressure: `floor` of the width at no
    /// pressure, the full width at full pressure. `shape(t)` scales the
    /// width further (a brush's profile along the stroke; 1 for none).
    pub fn width_nodes(
        &self,
        nodes: usize,
        width: f64,
        floor: f64,
        shape: &dyn Fn(f64) -> f64,
    ) -> Vec<WidthNode> {
        let nodes = nodes.max(1);
        let window = 0.5 / nodes as f64;
        let floor = floor.clamp(0.0, 1.0);
        (0..=nodes)
            .map(|i| {
                let t = i as f64 / nodes as f64;
                let p = self.at(t, window);
                let w = width * (floor + (1.0 - floor) * p) * shape(t).max(0.0);
                WidthNode::symmetric(t, w.max(0.0))
            })
            .collect()
    }

    /// The width nodes of a Freehand line drawn with the pen, `width` its
    /// outline width: `FREEHAND_NODES + 1` nodes from `FREEHAND_FLOOR` of
    /// the width at no pressure to the full width, never thinner than a
    /// hairline (or than `width`, when the outline is thinner still).
    pub fn freehand_widths(&self, width: f64) -> Vec<WidthNode> {
        let min = Stroke::HAIRLINE.min(width).max(0.0);
        let mut nodes = self.width_nodes(FREEHAND_NODES, width, FREEHAND_FLOOR, &|_| 1.0);
        for n in &mut nodes {
            if n.left + n.right < min {
                *n = WidthNode::symmetric(n.at, min);
            }
        }
        nodes
    }
}

/// Width nodes a freehand pen stroke gets along its path: enough to
/// follow the pressure, few enough to draw fast.
pub const FREEHAND_NODES: usize = 24;

/// How wide a freehand pen stroke is at no pressure, as a share of the
/// outline width; `freehand_widths` keeps it at a hairline at least, so a
/// touch so light leaves a line, not nothing.
pub const FREEHAND_FLOOR: f64 = 0.05;

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(phase: egui::TouchPhase, force: Option<f32>) -> egui::Event {
        egui::Event::Touch {
            device_id: egui::TouchDeviceId(1),
            id: egui::TouchId(7),
            phase,
            pos: egui::pos2(1.0, 1.0),
            force,
        }
    }

    #[test]
    fn the_pressure_follows_the_touch_events() {
        use egui::TouchPhase::*;
        assert_eq!(pressure_after(&[], None), None);
        assert_eq!(pressure_after(&[], Some(0.4)), Some(0.4));
        assert_eq!(
            pressure_after(&[touch(Start, Some(0.3)), touch(Move, Some(0.8))], None),
            Some(0.8)
        );
        // A move without a force clears it: no stale pressure.
        assert_eq!(pressure_after(&[touch(Move, None)], Some(0.8)), None);
        assert_eq!(pressure_after(&[touch(End, Some(0.5))], Some(0.8)), None);
        assert_eq!(pressure_after(&[touch(Cancel, None)], Some(0.8)), None);
        // Out of range readings are clamped, not trusted.
        assert_eq!(pressure_after(&[touch(Move, Some(3.0))], None), Some(1.0));
        assert_eq!(pressure_after(&[touch(Move, Some(f32::NAN))], None), None);
        // Pointer events change nothing.
        assert_eq!(
            pressure_after(
                &[egui::Event::PointerMoved(egui::pos2(3.0, 3.0))],
                Some(0.6)
            ),
            Some(0.6)
        );
    }

    #[test]
    fn a_stroke_repeats_the_last_pressure_for_a_point_without_one() {
        let mut s = PenStroke::start(Some(0.5));
        s.record(2, Some(0.6));
        s.record(3, Some(0.7));
        assert_eq!(s.pressures(3), Some(&[0.5, 0.6, 0.7][..]));
        assert_eq!(s.pressures(2), None, "a count that does not match");
        // A point without a reading (the pen lifting, a frame with no pen
        // event) keeps the pressure the stroke had.
        s.record(4, None);
        assert_eq!(s.pressures(4), Some(&[0.5, 0.6, 0.7, 0.7][..]));
        s.record(5, Some(0.9));
        assert_eq!(s.pressures(5), Some(&[0.5, 0.6, 0.7, 0.7, 0.9][..]));
        // Points the record missed: it no longer matches the drag.
        s.record(7, Some(0.4));
        assert_eq!(s.pressures(7), None);
        // The mouse never records anything.
        let mut m = PenStroke::start(None);
        m.record(2, Some(0.5));
        assert_eq!(m.pressures(2), None);
        assert_eq!(PenStroke::start(Some(0.5)).pressures(1), None);
    }

    #[test]
    fn a_freehand_line_is_never_thinner_than_a_hairline() {
        let points: Vec<Point> = (0..=10).map(|i| Point::new(i as f64 * 10.0, 0.0)).collect();
        let pressures = vec![0.0; points.len()];
        let prof = PressureProfile::new(&points, &pressures).expect("profile");
        // 5 % of a 0.2 mm outline would be 0.01 mm.
        let nodes = prof.freehand_widths(0.2);
        assert_eq!(nodes.len(), FREEHAND_NODES + 1);
        for n in &nodes {
            assert!((n.left + n.right - Stroke::HAIRLINE).abs() < 1e-12, "{n:?}");
            assert!((n.left - n.right).abs() < 1e-12);
        }
        // Wide outlines keep the 5 % floor, full pressure the full width.
        let nodes = prof.freehand_widths(4.0);
        assert!((nodes[0].left + nodes[0].right - 0.2).abs() < 1e-9);
        let full = PressureProfile::new(&points, &vec![1.0; points.len()]).expect("profile");
        assert!(full
            .freehand_widths(0.2)
            .iter()
            .all(|n| (n.left + n.right - 0.2).abs() < 1e-9));
        // An outline thinner than a hairline is not made wider.
        let thin = prof.freehand_widths(0.05);
        assert!(thin.iter().all(|n| (n.left + n.right - 0.05).abs() < 1e-12));
    }

    fn touch_at(phase: egui::TouchPhase, id: u64, force: Option<f32>) -> egui::Event {
        egui::Event::Touch {
            device_id: egui::TouchDeviceId(2),
            id: egui::TouchId(id),
            phase,
            pos: egui::pos2(40.0, 40.0),
            force,
        }
    }

    /// One frame of `events` through the raw input hook's filter, as the
    /// window runs it; the events that reached egui.
    fn run(ctx: &egui::Context, events: Vec<egui::Event>) -> Vec<egui::Event> {
        let mut raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(200.0, 200.0),
            )),
            events,
            ..Default::default()
        };
        filter_raw_input(ctx, &mut raw);
        let passed = raw.events.clone();
        let mut out = ctx.run_ui(raw, |_| {});
        out.textures_delta.clear();
        passed
    }

    /// A pen on X11 and macOS sends touches that only move: its pressure
    /// is read, and egui never sees them, so it does not switch to touch
    /// screen mode for the session (#850).
    #[test]
    fn pen_touches_that_only_move_give_pressure_and_never_reach_egui() {
        use egui::TouchPhase::*;
        let ctx = egui::Context::default();
        assert_eq!(pressure(&ctx), None);
        for f in [0.2, 0.6, 0.8] {
            let passed = run(
                &ctx,
                vec![
                    egui::Event::PointerMoved(egui::pos2(40.0, 40.0)),
                    touch_at(Move, 0, Some(f)),
                ],
            );
            assert_eq!(
                passed,
                vec![egui::Event::PointerMoved(egui::pos2(40.0, 40.0))]
            );
            assert_eq!(pressure(&ctx), Some(f));
            assert!(!ctx.input(|i| i.any_touches()));
            assert!(!ctx.input(|i| i.has_touch_screen()));
        }
        // The pen lifted: no force, no pressure.
        run(&ctx, vec![touch_at(Move, 0, None)]);
        assert_eq!(pressure(&ctx), None);
        assert!(!ctx.input(|i| i.has_touch_screen()));
        // A frame without touches changes nothing.
        run(&ctx, vec![touch_at(Move, 0, Some(0.4))]);
        run(&ctx, Vec::new());
        assert_eq!(pressure(&ctx), Some(0.4));
    }

    /// A real touch screen's touches start, move and end: they reach egui
    /// as they came, and a pen's moving touches beside them still do not.
    #[test]
    fn a_touch_screen_still_reaches_egui() {
        use egui::TouchPhase::*;
        let ctx = egui::Context::default();
        let start = touch_at(Start, 5, None);
        assert_eq!(run(&ctx, vec![start.clone()]), vec![start]);
        assert!(ctx.input(|i| i.has_touch_screen()));
        assert!(ctx.input(|i| i.any_touches()));
        let moved = touch_at(Move, 5, None);
        let pen = touch_at(Move, 0, Some(0.5));
        assert_eq!(run(&ctx, vec![moved.clone(), pen]), vec![moved]);
        assert!(ctx.input(|i| i.any_touches()));
        let end = touch_at(End, 5, None);
        assert_eq!(run(&ctx, vec![end.clone()]), vec![end]);
        assert!(!ctx.input(|i| i.any_touches()));
        // The touch is over: a stray move of its id no longer passes.
        assert!(run(&ctx, vec![touch_at(Move, 5, None)]).is_empty());
    }

    #[test]
    fn the_profile_averages_nearby_samples_and_scales_the_width() {
        let points: Vec<Point> = (0..=10).map(|i| Point::new(i as f64 * 10.0, 0.0)).collect();
        // Heavy over the first half, light over the second.
        let pressures: Vec<f32> = (0..=10).map(|i| if i <= 5 { 1.0 } else { 0.2 }).collect();
        let prof = PressureProfile::new(&points, &pressures).expect("profile");
        assert!((prof.at(0.0, 0.01) - 1.0).abs() < 1e-9);
        assert!((prof.at(1.0, 0.01) - 0.2).abs() < 1e-6);
        // A window over both halves averages them.
        let mid = prof.at(0.55, 0.1);
        assert!(mid > 0.2 && mid < 1.0, "{mid}");
        // Nothing within the window: the nearest sample.
        assert!((prof.at(0.26, 0.01) - 1.0).abs() < 1e-9);
        let nodes = prof.width_nodes(4, 8.0, 0.0, &|_| 1.0);
        assert_eq!(nodes.len(), 5);
        assert!((nodes[0].at, nodes[4].at) == (0.0, 1.0));
        assert!((nodes[0].left * 2.0 - 8.0).abs() < 1e-9);
        assert!((nodes[4].left * 2.0 - 1.6).abs() < 1e-6);
        // The floor keeps a light touch from vanishing.
        let floored = prof.width_nodes(4, 8.0, 0.5, &|_| 1.0);
        assert!((floored[4].left * 2.0 - 4.8).abs() < 1e-6);
        // The shape scales the width too.
        let shaped = prof.width_nodes(4, 8.0, 0.0, &|t| 1.0 - t);
        assert!(shaped[4].left.abs() < 1e-9);
        // Mismatched or empty input gives no profile.
        assert!(PressureProfile::new(&points, &pressures[..5]).is_none());
        assert!(PressureProfile::new(&points[..1], &pressures[..1]).is_none());
        assert!(PressureProfile::new(&[Point::ZERO, Point::ZERO], &[0.5, 0.5]).is_none());
    }
}
