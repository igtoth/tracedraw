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

use tracedraw_core::geometry::Point;
use tracedraw_core::style::WidthNode;

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

/// The pressures recorded along a freehand drag, one per point. They are
/// only kept while every point of the drag had one: a drag that starts
/// without pressure (the mouse) records nothing, and one that loses it
/// midway drops what it had, so the stroke falls back to its usual width.
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
    /// points, with `pressure` at it.
    pub fn record(&mut self, points: usize, pressure: Option<f32>) {
        match pressure {
            Some(p) if self.pressures.len() + 1 == points => self.pressures.push(p),
            _ => self.pressures.clear(),
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
}

/// Width nodes a freehand pen stroke gets along its path: enough to
/// follow the pressure, few enough to draw fast.
pub const FREEHAND_NODES: usize = 24;

/// How wide a freehand pen stroke is at no pressure, as a share of the
/// outline width: a touch so light leaves a hairline, not nothing.
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
    fn a_stroke_keeps_pressures_only_while_every_point_has_one() {
        let mut s = PenStroke::start(Some(0.5));
        s.record(2, Some(0.6));
        s.record(3, Some(0.7));
        assert_eq!(s.pressures(3), Some(&[0.5, 0.6, 0.7][..]));
        assert_eq!(s.pressures(2), None, "a count that does not match");
        // A point without pressure drops the record for good.
        s.record(4, None);
        assert_eq!(s.pressures(4), None);
        s.record(5, Some(0.9));
        assert_eq!(s.pressures(5), None);
        // The mouse never records anything.
        let mut m = PenStroke::start(None);
        m.record(2, Some(0.5));
        assert_eq!(m.pressures(2), None);
        assert_eq!(PenStroke::start(Some(0.5)).pressures(1), None);
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
