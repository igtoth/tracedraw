//! Blend tool: the controls on the canvas for the selected blend. A
//! square on the start object and one on the end object, the line
//! between them, and on that line two sliders: the object acceleration
//! (above the line) and the colour acceleration (below it). Each slider
//! sits where the middle step lands, so dragging it toward the start
//! bunches the steps there. The blend is picked by clicking its start
//! object, its end object or a step, text as any other object (#965).
//! See docs/behavior/blend.md.

use crate::app::App;
use tracedraw_core::{geometry::Point, live::Effect, ShapeId};

/// Which blend slider a drag holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendHandle {
    /// The object acceleration, above the line.
    Objects,
    /// The colour acceleration, below the line.
    Colors,
}

/// How far from a slider a press still takes it, screen pixels.
const HANDLE_PX: f32 = 7.0;
/// How far the sliders sit off the line, screen pixels.
pub const SLIDER_OFFSET_PX: f32 = 6.0;

/// The blend steps' exponent for acceleration `a`, as the blend draws them
/// (`tracedraw_core::live`): step `t` is placed at `t ^ (1 + a)`.
fn exponent(a: f64) -> f64 {
    let a = if a.is_finite() {
        a.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    (1.0 + a).max(0.05)
}

/// Where along the line, from the start (0) to the end (1), the middle
/// step of a blend with acceleration `a` lands: where the slider shows.
pub fn slider_fraction(a: f64) -> f64 {
    0.5f64.powf(exponent(a))
}

/// The acceleration that puts the middle step at `f` along the line,
/// within -1..1 (the inverse of [`slider_fraction`]).
pub fn accel_for_fraction(f: f64) -> f64 {
    let lo = 0.5f64.powf(exponent(1.0));
    let hi = 0.5f64.powf(exponent(-1.0));
    let f = if f.is_finite() { f.clamp(lo, hi) } else { 0.5 };
    let a = f.ln() / 0.5f64.ln() - 1.0;
    // Two decimals, as the bar and the panel show it (in percent).
    ((a * 100.0).round() / 100.0).clamp(-1.0, 1.0)
}

/// The selected blend's controls, in page space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlendControls {
    /// The object holding the blend.
    pub shape: ShapeId,
    /// The centre of the start object.
    pub start: Point,
    /// The centre of the end object.
    pub end: Point,
    pub accel_objects: f64,
    pub accel_colors: f64,
}

impl BlendControls {
    /// The point on the line where the slider of `h` sits.
    pub fn slider_point(&self, h: BlendHandle) -> Point {
        let a = match h {
            BlendHandle::Objects => self.accel_objects,
            BlendHandle::Colors => self.accel_colors,
        };
        self.start + (self.end - self.start) * slider_fraction(a)
    }

    /// How far along the line `p` lies (0 at the start, 1 at the end).
    pub fn fraction_at(&self, p: Point) -> f64 {
        let d = self.end - self.start;
        let len2 = d.hypot2();
        if len2 < 1e-12 {
            return 0.5;
        }
        ((p - self.start).dot(d) / len2).clamp(0.0, 1.0)
    }
}

/// The unit normal of the line `a` to `b` on screen (pointing to the left
/// of the way from `a` to `b`, so "above" for a line drawn left to right),
/// or straight up when the two points meet.
pub fn screen_normal(a: egui::Pos2, b: egui::Pos2) -> egui::Vec2 {
    let d = b - a;
    let len = d.length();
    if len < 1e-3 {
        egui::vec2(0.0, -1.0)
    } else {
        egui::vec2(d.y, -d.x) / len
    }
}

impl App {
    /// The controls of the first selected blend while the Blend tool is
    /// in hand.
    pub fn blend_controls(&self) -> Option<BlendControls> {
        if self.tool != crate::tools::Tool::Blend {
            return None;
        }
        let (id, e) = self.selected_blends().into_iter().next()?;
        let Effect::Blend {
            end,
            accel_objects,
            accel_colors,
            ..
        } = e
        else {
            return None;
        };
        let start = self.doc().shape_on_page(id)?.bounds().center();
        Some(BlendControls {
            shape: id,
            start,
            end: end.bounds().center(),
            accel_objects,
            accel_colors,
        })
    }

    /// The slider under `p` (page space), if any.
    pub fn blend_handle_at(&self, p: Point) -> Option<BlendHandle> {
        let c = self.blend_controls()?;
        let view = &self.view;
        let (a, b) = (view.to_screen(c.start), view.to_screen(c.end));
        if (b - a).length() < 2.0 * HANDLE_PX {
            return None;
        }
        let n = screen_normal(a, b) * SLIDER_OFFSET_PX;
        let at = view.to_screen(p);
        // The object slider wins where the two overlap.
        [(BlendHandle::Objects, n), (BlendHandle::Colors, -n)]
            .into_iter()
            .find(|(h, off)| (view.to_screen(c.slider_point(*h)) + *off - at).length() <= HANDLE_PX)
            .map(|(h, _)| h)
    }

    /// The slider `h` dragged to `p`: the blend's acceleration follows,
    /// the other one too when the accelerations are linked. A drag is one
    /// undo step.
    pub fn drag_blend_handle(&mut self, h: BlendHandle, p: Point) {
        let Some(c) = self.blend_controls() else {
            return;
        };
        let a = accel_for_fraction(c.fraction_at(p));
        let link = self.blend_link_accel;
        let (objects, colors) = match h {
            BlendHandle::Objects => (true, link),
            BlendHandle::Colors => (link, true),
        };
        if objects {
            self.blend_accel_objects = a;
        }
        if colors {
            self.blend_accel_colors = a;
        }
        self.change_blends("Blend", |e| {
            let mut e = e.clone();
            if let Effect::Blend {
                accel_objects,
                accel_colors,
                ..
            } = &mut e
            {
                if objects {
                    *accel_objects = a;
                }
                if colors {
                    *accel_colors = a;
                }
            }
            Some(e)
        });
    }

    /// A press at `p` with the Blend tool: on a slider it is taken and the
    /// drag that follows moves it (a fresh undo step). `false` elsewhere.
    pub fn begin_blend_handle(&mut self, p: Point) -> bool {
        match self.blend_handle_at(p) {
            Some(h) => {
                self.blend_handle_drag = Some(h);
                self.effect_edit_step = None;
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slider_sits_where_the_middle_step_lands() {
        assert!((slider_fraction(0.0) - 0.5).abs() < 1e-9);
        // Speeding up bunches the steps at the start.
        assert!(slider_fraction(0.5) < 0.5);
        assert!(slider_fraction(-0.5) > 0.5);
        for a in [-0.9, -0.5, -0.25, 0.0, 0.3, 0.75, 1.0] {
            let back = accel_for_fraction(slider_fraction(a));
            assert!((back - a).abs() < 0.011, "{a} -> {back}");
        }
        // Out of range and bad input stay within -1..1.
        assert_eq!(accel_for_fraction(0.0), 1.0);
        assert!(accel_for_fraction(1.0) >= -1.0);
        assert_eq!(accel_for_fraction(f64::NAN), 0.0);
    }

    #[test]
    fn the_fraction_is_measured_along_the_line() {
        let c = BlendControls {
            shape: ShapeId(1),
            start: Point::new(0.0, 0.0),
            end: Point::new(100.0, 0.0),
            accel_objects: 0.0,
            accel_colors: 0.0,
        };
        assert!((c.fraction_at(Point::new(25.0, 40.0)) - 0.25).abs() < 1e-9);
        assert_eq!(c.fraction_at(Point::new(-10.0, 0.0)), 0.0);
        assert_eq!(c.fraction_at(Point::new(500.0, 0.0)), 1.0);
        let p = c.slider_point(BlendHandle::Colors);
        assert!((p.x - 50.0).abs() < 1e-9 && p.y.abs() < 1e-9);
        let same = BlendControls { end: c.start, ..c };
        assert_eq!(same.fraction_at(Point::new(3.0, 3.0)), 0.5);
    }
}
