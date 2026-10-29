//! The slider of the bars, dockers and dialogs: an optional label above
//! the control, left aligned and ending with a colon; under it a thin
//! groove with tick marks along it like a ruler (minor ticks at even
//! steps, longer major ticks at round values), a slim upright thumb
//! standing taller than the groove, and a number box at the right of the
//! groove showing the value (a [`NumField`], typed into, stepped with its
//! arrows or the wheel).
//!
//! The value changes by dragging the thumb, clicking on the groove (the
//! thumb jumps there), the arrow keys while the groove has the keyboard
//! focus (Page Up and Down step ten times, Home and End go to the ends),
//! the wheel over the groove while it has the focus or is dragged (Ctrl
//! a tenth of a step, Shift ten steps) and the number box; the box and
//! the thumb always show the same value. A step snaps every change, a
//! typed value too, to a multiple of it from the range's start; an
//! integer slider steps by one. The value given is clamped to the range,
//! as it is edited.
//!
//! The groove takes the keyboard focus only from Tab, as egui's own
//! slider: a click or a drag leaves the keys to the canvas (Ctrl+Z,
//! Delete, the tool keys and the arrows' nudge keep working), and the
//! wheel over a slider it does not have scrolls the panel around it.
//!
//! Nothing is allocated for the ticks: they are computed from the range
//! and the groove's width each frame and drawn as line segments.

use crate::theme::Tokens;
use crate::ui::field::{self, NumField};
use egui::emath::{GuiRounding, Numeric};
use egui::{Rect, Response, Sense, Ui, Widget};
use std::ops::RangeInclusive;

/// The thumb, points.
const THUMB_W: f32 = 6.0;
const THUMB_H: f32 = 14.0;
/// The groove's thickness, and where it lies in the control's height.
const GROOVE_H: f32 = 3.0;
const GROOVE_TOP: f32 = 6.0;
/// The ticks start under the groove; a minor tick is short, a major
/// tick longer.
const TICK_GAP: f32 = 2.0;
const MINOR_TICK: f32 = 3.0;
const MAJOR_TICK: f32 = 6.0;
/// The height of the groove, thumb and ticks together.
const RAIL_H: f32 = 20.0;
/// The narrowest groove.
const MIN_RAIL_W: f32 = 40.0;
/// The number box with its spin arrows.
const BOX_W: f32 = field::DEFAULT_WIDTH;
/// Points between the groove and the box.
const BOX_GAP: f32 = 6.0;
/// Fewest points between two ticks that are drawn.
const MIN_TICK_PX: f64 = 5.0;

/// A slider; built like `egui::Slider`, drawn as the bars and dockers
/// want it (see the module notes).
pub struct Slider<'a, N: Numeric> {
    value: &'a mut N,
    min: f64,
    max: f64,
    step: Option<f64>,
    /// The label drawn above the control (a colon is added).
    label: Option<String>,
    suffix: String,
    min_decimals: usize,
    max_decimals: Option<usize>,
    show_value: bool,
    ticks: bool,
    logarithmic: bool,
    trailing_fill: bool,
    vertical: bool,
    mixed: bool,
    /// The groove's length when the layout does not decide it.
    length: Option<f32>,
}

impl<'a, N: Numeric> Slider<'a, N> {
    pub fn new(value: &'a mut N, range: RangeInclusive<N>) -> Self {
        let (a, b) = (range.start().to_f64(), range.end().to_f64());
        Slider {
            value,
            min: a.min(b),
            max: a.max(b),
            step: None,
            label: None,
            suffix: String::new(),
            min_decimals: 0,
            max_decimals: None,
            show_value: true,
            ticks: true,
            logarithmic: false,
            trailing_fill: false,
            vertical: false,
            mixed: false,
            length: None,
        }
    }

    /// The label above the control; a colon is added unless it ends
    /// with one.
    pub fn label(mut self, label: impl ToString) -> Self {
        let text = label.to_string();
        self.label = (!text.trim().is_empty()).then_some(text);
        self
    }

    /// `egui::Slider`'s name for the label.
    pub fn text(self, text: impl ToString) -> Self {
        self.label(text)
    }

    pub fn suffix(mut self, suffix: impl ToString) -> Self {
        self.suffix = suffix.to_string();
        self
    }

    /// Every change snaps to a multiple of `step` from the range's start.
    pub fn step_by(mut self, step: f64) -> Self {
        self.step = (step.is_finite() && step > 0.0).then_some(step);
        self
    }

    /// Whole numbers only, stepping by one.
    pub fn integer(self) -> Self {
        self.fixed_decimals(0).step_by(1.0)
    }

    pub fn fixed_decimals(mut self, n: usize) -> Self {
        self.min_decimals = n;
        self.max_decimals = Some(n);
        self
    }

    pub fn max_decimals(mut self, n: usize) -> Self {
        self.max_decimals = Some(n);
        self
    }

    /// Whether the number box is shown at the right of the groove.
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Whether the ticks are drawn under the groove.
    pub fn ticks(mut self, ticks: bool) -> Self {
        self.ticks = ticks;
        self
    }

    /// The thumb's position follows the logarithm of the value (the
    /// range has to be positive; otherwise the slider stays linear).
    pub fn logarithmic(mut self, log: bool) -> Self {
        self.logarithmic = log;
        self
    }

    /// The groove is filled in the accent colour from its start to the
    /// thumb.
    pub fn trailing_fill(mut self, fill: bool) -> Self {
        self.trailing_fill = fill;
        self
    }

    /// An upright slider, the range's end at the top; no label and no
    /// number box.
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// The selected objects differ in this value: the number box is
    /// empty, and a value typed there applies to all of them.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// The groove's length in points. Without it the groove fills the
    /// width of a column (a docker, a dialog's body) and takes
    /// `Spacing::slider_width` in a row (a bar).
    pub fn width(mut self, width: f32) -> Self {
        self.length = Some(width);
        self
    }

    /// The step of the keys and the wheel: the step given, one for an
    /// integer slider, else about a hundredth of the range, rounded to
    /// 1, 2 or 5.
    fn key_step(&self) -> f64 {
        if let Some(s) = self.step {
            return s;
        }
        if N::INTEGRAL {
            return 1.0;
        }
        nice_near((self.max - self.min) / 100.0)
    }

    /// The step of the wheel (and the box's arrows) with Ctrl: a tenth of
    /// a step, or one step on an integer or stepped slider, which holds
    /// nothing finer (a tenth would snap back to where it was).
    fn fine_step(&self) -> f64 {
        if self.step.is_some() || N::INTEGRAL {
            self.key_step()
        } else {
            self.key_step() / 10.0
        }
    }

    /// `v` clamped to the range and snapped to the step.
    fn snapped(&self, v: f64) -> f64 {
        let mut v = v.clamp(self.min, self.max);
        if let Some(s) = self.step {
            v = self.min + ((v - self.min) / s).round() * s;
        }
        if N::INTEGRAL {
            v = v.round();
        }
        v.clamp(self.min, self.max)
    }

    fn log_scale(&self) -> bool {
        self.logarithmic && self.min > 0.0 && self.max > self.min
    }

    /// Where `v` lies along the groove, 0 at the start, 1 at the end.
    fn position(&self, v: f64) -> f64 {
        if self.max <= self.min {
            return 0.0;
        }
        let t = if self.log_scale() {
            (v.ln() - self.min.ln()) / (self.max.ln() - self.min.ln())
        } else {
            (v - self.min) / (self.max - self.min)
        };
        if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// The value at `t` along the groove (0 to 1).
    fn value_at(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        if self.log_scale() {
            (self.min.ln() + t * (self.max.ln() - self.min.ln())).exp()
        } else {
            self.min + t * (self.max - self.min)
        }
    }

    /// Every tick's value and whether it is a major one, for a groove of
    /// `px` points.
    fn for_each_tick(&self, px: f32, mut f: impl FnMut(f64, bool)) {
        let span = self.max - self.min;
        if span <= 0.0 || px <= 0.0 || span.is_nan() || px.is_nan() {
            return;
        }
        if self.log_scale() {
            let decade_px = px as f64 / (self.max / self.min).log10().max(1e-9);
            let (e0, e1) = (
                self.min.log10().floor() as i32,
                self.max.log10().ceil() as i32,
            );
            for e in e0..=e1 {
                let base = 10f64.powi(e);
                for m in 1..10 {
                    let v = m as f64 * base;
                    if v < self.min * (1.0 - 1e-9) || v > self.max * (1.0 + 1e-9) {
                        continue;
                    }
                    let major = m == 1;
                    // Narrow decades keep the 2 and 5 only, the
                    // narrowest their start alone.
                    if !major && (decade_px < 24.0 || (decade_px < 72.0 && m != 2 && m != 5)) {
                        continue;
                    }
                    f(v, major);
                }
            }
            return;
        }
        let step = self.step.or(N::INTEGRAL.then_some(1.0));
        let (major, minor) = linear_spacing(span, px as f64, step);
        let i0 = (self.min / minor - 1e-6).ceil() as i64;
        let i1 = (self.max / minor + 1e-6).floor() as i64;
        if i1 < i0 || i1 - i0 > 4096 {
            return;
        }
        for i in i0..=i1 {
            let v = i as f64 * minor;
            let k = v / major;
            f(v, (k - k.round()).abs() < 1e-6);
        }
    }

    /// Decimals the box shows: as given, else as many as the step has.
    fn box_decimals(&self) -> Option<usize> {
        self.max_decimals.or_else(|| {
            self.step.map(|s| {
                let mut n = 0;
                let mut s = s;
                while n < 6 && (s - s.round()).abs() > 1e-9 {
                    s *= 10.0;
                    n += 1;
                }
                n
            })
        })
    }
}

/// The 1, 2 or 5 times a power of ten nearest to `x` (in ratio).
fn nice_near(x: f64) -> f64 {
    if x <= 0.0 || !x.is_finite() {
        return 1.0;
    }
    let k = 10f64.powi(x.log10().floor() as i32);
    let mut best = k;
    for m in [1.0, 2.0, 5.0, 10.0] {
        if (m * k / x).ln().abs() < (best / x).ln().abs() {
            best = m * k;
        }
    }
    best
}

/// Major and minor tick spacing for a linear range of `span` over `px`
/// points, in the slider's unit: a round major every 45 points or so,
/// minors in fourths (of a 2) or fifths, never closer than the step or
/// than [`MIN_TICK_PX`] points.
fn linear_spacing(span: f64, px: f64, step: Option<f64>) -> (f64, f64) {
    let target = (px / 45.0).clamp(2.0, 12.0);
    let mut major = nice_near(span / target);
    let mantissa = major / 10f64.powi(major.log10().floor() as i32);
    let mut minor = if (mantissa - 2.0).abs() < 1e-6 {
        major / 4.0
    } else {
        major / 5.0
    };
    if let Some(s) = step.filter(|s| *s > 0.0) {
        if minor < s {
            minor = (minor / s).ceil().max(1.0) * s;
        }
        if major < minor {
            major = minor;
        }
        major = (major / minor).round().max(1.0) * minor;
    }
    if minor * px / span < MIN_TICK_PX {
        minor = major;
    }
    (major, minor)
}

/// A slider's response read by an editor that applies its value once
/// (one undo step).
pub trait Settled {
    /// The value is set for good: a drag of the thumb ended, or it
    /// changed by a click, a key, the wheel or the number box.
    /// `drag_stopped()` alone misses every change made without a drag,
    /// which then snaps back.
    fn settled(&self) -> bool;
}

impl Settled for Response {
    fn settled(&self) -> bool {
        self.drag_stopped() || (self.changed() && !self.dragged())
    }
}

#[cfg(test)]
fn record_rail(ctx: &egui::Context, label: &str, rail: Rect, id: egui::Id) {
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new(("ui_slider_rail", label)), rail);
        d.insert_temp(egui::Id::new(("ui_slider_rail_id", label)), id);
    });
}

/// Tests: the id of the groove of the slider labelled `label`, the one
/// that takes the keyboard focus.
#[cfg(test)]
pub fn rail_id(ctx: &egui::Context, label: &str) -> Option<egui::Id> {
    ctx.data(|d| d.get_temp(egui::Id::new(("ui_slider_rail_id", label))))
}

/// Tests: where the groove of the slider labelled `label` (`""` for an
/// unlabelled one) was drawn in the last frame.
#[cfg(test)]
pub fn rail_rect(ctx: &egui::Context, label: &str) -> Option<Rect> {
    ctx.data(|d| d.get_temp(egui::Id::new(("ui_slider_rail", label))))
}

impl<N: Numeric> Widget for Slider<'_, N> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        // The value given is kept in the range, as egui's slider does.
        let given = self.value.to_f64();
        let held = given.clamp(self.min, self.max);
        if held != given && held.is_finite() {
            *self.value = N::from_f64(held);
        }
        // How long the groove is: as asked, the room of a column, the
        // standard width in a row of controls.
        let layout = *ui.layout();
        let fills = !layout.main_dir().is_horizontal() || layout.horizontal_justify();
        let box_w = if self.show_value && !self.vertical {
            BOX_W + BOX_GAP
        } else {
            0.0
        };
        let length = match self.length {
            Some(l) => l,
            None if self.vertical => {
                let h = ui.available_height();
                if h.is_finite() {
                    h.max(MIN_RAIL_W)
                } else {
                    ui.spacing().slider_width
                }
            }
            None if fills => {
                let w = ui.available_width() - box_w;
                if w.is_finite() {
                    w.max(MIN_RAIL_W)
                } else {
                    ui.spacing().slider_width
                }
            }
            None => ui.spacing().slider_width.max(MIN_RAIL_W),
        };
        if self.vertical {
            return self.rail(ui, egui::vec2(RAIL_H, length));
        }
        let label = self.label.clone();
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            if let Some(label) = &label {
                let with_colon = label.ends_with(':') || label.ends_with('\u{ff1a}');
                let text = if with_colon {
                    label.clone()
                } else {
                    format!("{label}:")
                };
                ui.label(text);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = BOX_GAP;
                let mut resp = self.rail(ui, egui::vec2(length, RAIL_H));
                #[cfg(test)]
                record_rail(ui.ctx(), label.as_deref().unwrap_or(""), resp.rect, resp.id);
                if self.show_value {
                    let (step, fine, decimals) =
                        (self.key_step(), self.fine_step(), self.box_decimals());
                    let before = self.value.to_f64();
                    // The wheel over the box steps it only while the
                    // slider has the keyboard (or is dragged); otherwise
                    // it scrolls the panel.
                    let wheel = resp.has_focus() || resp.dragged();
                    let mut f = NumField::new(&mut *self.value)
                        .range(self.min..=self.max)
                        .suffix(&self.suffix)
                        .step(step)
                        .fine_step(fine)
                        .wheel_on_hover(wheel)
                        .mixed(self.mixed)
                        .width(BOX_W);
                    if self.min_decimals > 0 {
                        f = f.fixed_decimals(self.min_decimals);
                    }
                    if let Some(n) = decimals {
                        f = f.max_decimals(n);
                    }
                    let mut field = ui.add(f);
                    if field.changed() {
                        // A typed value takes the step too (12.5 in a
                        // whole-number box is 13, as it shows).
                        let v = self.value.to_f64();
                        let snapped = self.snapped(v);
                        if snapped != v && snapped.is_finite() {
                            *self.value = N::from_f64(snapped);
                        }
                        if self.value.to_f64() == before && !self.mixed {
                            field.flags.set(egui::response::Flags::CHANGED, false);
                        }
                    }
                    resp = resp.union(field);
                }
                resp
            })
            .inner
        })
        .inner
    }
}

impl<N: Numeric> Slider<'_, N> {
    /// The groove with its ticks and thumb, `size` points, taking the
    /// pointer, the keys and the wheel.
    fn rail(&mut self, ui: &mut Ui, size: egui::Vec2) -> Response {
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let enabled = ui.is_enabled();
        let old = self.value.to_f64();
        let mut v = old;
        // Along the groove: from `a` to `b` in points, the value's
        // position between them.
        let half = THUMB_W / 2.0;
        let (a, b) = if self.vertical {
            (rect.bottom() - half, rect.top() + half)
        } else {
            (rect.left() + half, rect.right() - half)
        };
        let len = (b - a) as f64;
        if enabled {
            // The pointer: a press or a drag puts the thumb there. It
            // does not take the keyboard focus (and gives up one Tab
            // gave), so the canvas keeps its shortcuts.
            if let Some(p) = resp.interact_pointer_pos() {
                let along = if self.vertical { p.y } else { p.x };
                if len.abs() > 0.0 {
                    v = self.snapped(self.value_at((along - a) as f64 / len));
                }
                if resp.has_focus() {
                    resp.surrender_focus();
                }
            }
            // The keys, with the focus (the arrows along the groove step
            // the value instead of moving the focus on).
            if resp.has_focus() {
                let vertical = self.vertical;
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        resp.id,
                        egui::EventFilter {
                            horizontal_arrows: !vertical,
                            vertical_arrows: vertical,
                            ..Default::default()
                        },
                    );
                });
                let step = self.key_step();
                let steps = ui.input_mut(|i| {
                    use egui::{Key, Modifiers};
                    let mut n = 0.0;
                    let (inc, dec) = if vertical {
                        (Key::ArrowUp, Key::ArrowDown)
                    } else {
                        (Key::ArrowRight, Key::ArrowLeft)
                    };
                    for (key, dir) in [
                        (inc, 1.0),
                        (dec, -1.0),
                        (Key::PageUp, 10.0),
                        (Key::PageDown, -10.0),
                    ] {
                        n += dir * i.count_and_consume_key(Modifiers::NONE, key) as f64;
                    }
                    n
                });
                if steps != 0.0 {
                    v = self.snapped(v + steps * step);
                }
                let (home, end) = ui.input_mut(|i| {
                    (
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Home),
                        i.consume_key(egui::Modifiers::NONE, egui::Key::End),
                    )
                });
                if home {
                    v = self.min;
                }
                if end {
                    v = self.max;
                }
            }
            // The wheel over the groove while it has the keyboard focus
            // or is dragged, a step a notch (Ctrl a tenth, Shift ten);
            // the panel around does not scroll with it then. Hovering
            // alone leaves the wheel to the panel.
            let wheel_id = resp.id.with("wheel");
            if ui.rect_contains_pointer(rect) && (resp.has_focus() || resp.dragged()) {
                if let Some(notches) = field::wheel_steps(ui, wheel_id) {
                    if notches != 0 {
                        let mods = ui.input(|i| {
                            i.raw
                                .events
                                .iter()
                                .rev()
                                .find_map(|e| match e {
                                    egui::Event::MouseWheel { modifiers, .. } => Some(*modifiers),
                                    _ => None,
                                })
                                .unwrap_or(i.modifiers)
                        });
                        let step = if mods.command {
                            self.fine_step()
                        } else if mods.shift {
                            self.key_step() * 10.0
                        } else {
                            self.key_step()
                        };
                        v = self.snapped(v + f64::from(notches) * step);
                    }
                }
            } else {
                ui.data_mut(|d| d.remove::<f32>(wheel_id));
            }
        }
        if v != old && v.is_finite() {
            *self.value = N::from_f64(v);
            resp.mark_changed();
        }
        let shown = self.value.to_f64();
        resp.widget_info(|| {
            egui::WidgetInfo::slider(enabled, shown, self.label.as_deref().unwrap_or(""))
        });
        if ui.is_rect_visible(rect) {
            self.paint(ui, &resp, rect, a, b, shown);
        }
        resp
    }

    /// The groove, its ticks and the thumb at `value`, between `a` and
    /// `b` along the groove.
    fn paint(&self, ui: &Ui, resp: &Response, rect: Rect, a: f32, b: f32, value: f64) {
        let ppp = ui.pixels_per_point();
        let enabled = ui.is_enabled();
        let p = ui.painter();
        let at = |t: f64| (a + (b - a) * t as f32).round_to_pixel_center(ppp);
        // The groove, and its filled part.
        let groove = if self.vertical {
            Rect::from_x_y_ranges(
                egui::Rangef::new(
                    rect.left() + GROOVE_TOP,
                    rect.left() + GROOVE_TOP + GROOVE_H,
                ),
                egui::Rangef::new(rect.top(), rect.bottom()),
            )
        } else {
            Rect::from_x_y_ranges(
                egui::Rangef::new(rect.left(), rect.right()),
                egui::Rangef::new(rect.top() + GROOVE_TOP, rect.top() + GROOVE_TOP + GROOVE_H),
            )
        }
        .round_to_pixels(ppp);
        let groove_color = if enabled {
            Tokens::slider_rail()
        } else {
            Tokens::slider_rail().gamma_multiply(0.5)
        };
        p.rect_filled(groove, 0.0, groove_color);
        let pos = at(self.position(value));
        if self.trailing_fill && enabled {
            let filled = if self.vertical {
                Rect::from_x_y_ranges(groove.x_range(), egui::Rangef::new(pos, groove.bottom()))
            } else {
                Rect::from_x_y_ranges(egui::Rangef::new(groove.left(), pos), groove.y_range())
            };
            p.rect_filled(filled, 0.0, Tokens::ACCENT);
        }
        // The ticks under the groove (beside it when upright).
        if self.ticks {
            let color = if enabled {
                Tokens::ruler_tick()
            } else {
                Tokens::icon_off()
            };
            let stroke = egui::Stroke::new(1.0, color);
            let from = if self.vertical {
                groove.right() + TICK_GAP
            } else {
                groove.bottom() + TICK_GAP
            };
            self.for_each_tick((b - a).abs(), |v, major| {
                let x = at(self.position(v));
                let to = from + if major { MAJOR_TICK } else { MINOR_TICK };
                if self.vertical {
                    p.line_segment([egui::pos2(from, x), egui::pos2(to, x)], stroke);
                } else {
                    p.line_segment([egui::pos2(x, from), egui::pos2(x, to)], stroke);
                }
            });
        }
        // The thumb: an upright slip over the groove.
        let center = if self.vertical {
            egui::pos2(groove.center().x, pos)
        } else {
            egui::pos2(pos, groove.center().y)
        };
        let size = if self.vertical {
            egui::vec2(THUMB_H, THUMB_W)
        } else {
            egui::vec2(THUMB_W, THUMB_H)
        };
        let thumb = Rect::from_center_size(center, size).round_to_pixels(ppp);
        let (fill, border) = if !enabled {
            (Tokens::panel(), Tokens::icon_off())
        } else if resp.dragged() || resp.is_pointer_button_down_on() {
            (Tokens::tool_active(), Tokens::ACCENT)
        } else if resp.hovered() || resp.has_focus() {
            (Tokens::tool_hover(), Tokens::ACCENT)
        } else {
            (Tokens::button(), Tokens::control_border())
        };
        p.rect_filled(thumb, 1.0, fill);
        p.rect_stroke(
            thumb,
            1.0,
            egui::Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> egui::Context {
        crate::theme::ui_context()
    }

    fn input(events: Vec<egui::Event>) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(600.0, 300.0),
            )),
            events,
            ..Default::default()
        }
    }

    /// One frame with a labelled percent slider in a 300 points wide
    /// panel; the shapes drawn come back.
    fn frame(
        ctx: &egui::Context,
        v: &mut f64,
        events: Vec<egui::Event>,
    ) -> (egui::FullOutput, Response) {
        let mut resp = None;
        let mut out = ctx.run_ui(input(events), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                column(ui, |ui| {
                    resp = Some(
                        ui.add(
                            Slider::new(&mut *v, 0.0..=100.0)
                                .text("Detail")
                                .suffix(" %")
                                .step_by(1.0),
                        ),
                    );
                });
            });
        });
        out.textures_delta.clear();
        (out, resp.expect("slider"))
    }

    /// A 300 points wide column, as a docker or a dialog's body.
    fn column(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
        ui.allocate_ui_with_layout(
            egui::vec2(300.0, 200.0),
            egui::Layout::top_down(egui::Align::Min),
            add,
        );
    }

    fn press(pos: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn key(key: egui::Key, pressed: bool) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// The texts drawn in a frame.
    fn texts(out: &egui::FullOutput) -> Vec<String> {
        out.shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_string()),
                _ => None,
            })
            .collect()
    }

    /// Line segments drawn inside `rect` in a frame.
    fn lines_in(out: &egui::FullOutput, rect: Rect) -> usize {
        out.shapes
            .iter()
            .filter(|s| match &s.shape {
                egui::Shape::LineSegment { points, .. } => {
                    rect.contains(points[0]) && rect.contains(points[1])
                }
                _ => false,
            })
            .count()
    }

    /// Where the thumb of `v` is on the groove.
    fn thumb_x(rail: Rect, v: f64) -> f32 {
        let half = THUMB_W / 2.0;
        rail.left() + half + (rail.width() - THUMB_W) * v as f32 / 100.0
    }

    /// The label sits above the groove with a colon, the groove spans
    /// the column up to the number box, which shows the value with its
    /// suffix, and the ticks are drawn as line segments under the groove.
    #[test]
    fn draws_a_label_a_ticked_groove_and_the_value_box() {
        let ctx = ctx();
        let mut v = 40.0;
        frame(&ctx, &mut v, Vec::new());
        let (out, resp) = frame(&ctx, &mut v, Vec::new());
        let rail = rail_rect(&ctx, "Detail").expect("the groove was drawn");
        let texts = texts(&out);
        assert!(texts.iter().any(|t| t == "Detail:"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "40 %"), "{texts:?}");
        // The groove fills the column but leaves the box its room.
        assert!(
            rail.width() > 150.0 && rail.width() < 300.0 - BOX_W,
            "{rail:?}"
        );
        // The box (its typing part; the arrows beside it have their own
        // response) is at the right of the groove.
        assert!(
            resp.rect.right() > rail.right() + BOX_GAP + BOX_W - field::SPIN_W - 1.0,
            "the box is at the right: {:?} {rail:?}",
            resp.rect
        );
        assert!(resp.rect.contains_rect(rail));
        // Ticks: 0..100 over about 220 points gives majors every 20 and
        // minors every 5: 21 line segments under the groove.
        let under = Rect::from_min_max(
            egui::pos2(rail.left() - 1.0, rail.top() + GROOVE_TOP + GROOVE_H),
            egui::pos2(rail.right() + 1.0, rail.bottom() + 1.0),
        );
        assert_eq!(lines_in(&out, under), 21, "{rail:?}");
        // Without ticks, none.
        let mut w = 40.0;
        let mut out = ctx.run_ui(input(Vec::new()), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                column(ui, |ui| {
                    ui.add(Slider::new(&mut w, 0.0..=100.0).text("Plain").ticks(false));
                });
            });
        });
        out.textures_delta.clear();
        let rail = rail_rect(&ctx, "Plain").expect("drawn");
        assert_eq!(lines_in(&out, rail.expand(2.0)), 0);
    }

    /// One notch of the wheel with `modifiers`, down when `down`.
    fn wheel(down: bool, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            phase: egui::TouchPhase::Move,
            delta: egui::vec2(0.0, if down { -1.0 } else { 1.0 }),
            modifiers,
        }
    }

    /// A press on the groove puts the thumb (and the value) there, a
    /// drag moves it, the arrow keys step it once Tab gave it the focus,
    /// the wheel steps it over the groove with the focus, and the box
    /// follows each change.
    #[test]
    fn the_pointer_the_keys_and_the_wheel_change_the_value() {
        let ctx = ctx();
        let mut v = 40.0;
        frame(&ctx, &mut v, Vec::new());
        frame(&ctx, &mut v, Vec::new());
        let rail = rail_rect(&ctx, "Detail").expect("drawn");
        // Click on the groove at 75 %: the value jumps there.
        let at = egui::pos2(thumb_x(rail, 75.0), rail.center().y);
        frame(&ctx, &mut v, vec![egui::Event::PointerMoved(at)]);
        let (_, resp) = frame(&ctx, &mut v, vec![press(at, true)]);
        assert!(resp.changed(), "a press changes the value");
        assert_eq!(v, 75.0);
        // Drag to 20 %.
        let to = egui::pos2(thumb_x(rail, 20.0), rail.center().y);
        frame(&ctx, &mut v, vec![egui::Event::PointerMoved(to)]);
        assert_eq!(v, 20.0);
        let (out, _) = frame(&ctx, &mut v, vec![press(to, false)]);
        assert_eq!(v, 20.0);
        assert!(texts(&out).iter().any(|t| t == "20 %"), "the box follows");
        // Tab gives the groove the focus: arrows step by one, Page Up by
        // ten, End goes to the end.
        frame(
            &ctx,
            &mut v,
            vec![key(egui::Key::Tab, true), key(egui::Key::Tab, false)],
        );
        frame(&ctx, &mut v, Vec::new());
        assert!(ctx.egui_wants_keyboard_input(), "Tab focuses the groove");
        frame(
            &ctx,
            &mut v,
            vec![
                key(egui::Key::ArrowRight, true),
                key(egui::Key::ArrowRight, false),
            ],
        );
        assert_eq!(v, 21.0, "right arrow");
        frame(
            &ctx,
            &mut v,
            vec![
                key(egui::Key::ArrowLeft, true),
                key(egui::Key::ArrowLeft, false),
                key(egui::Key::ArrowLeft, true),
                key(egui::Key::ArrowLeft, false),
            ],
        );
        assert_eq!(v, 19.0, "two left arrows");
        frame(
            &ctx,
            &mut v,
            vec![key(egui::Key::PageUp, true), key(egui::Key::PageUp, false)],
        );
        assert_eq!(v, 29.0, "page up");
        frame(
            &ctx,
            &mut v,
            vec![key(egui::Key::End, true), key(egui::Key::End, false)],
        );
        assert_eq!(v, 100.0, "end, clamped to the range");
        frame(
            &ctx,
            &mut v,
            vec![
                key(egui::Key::ArrowRight, true),
                key(egui::Key::ArrowRight, false),
            ],
        );
        assert_eq!(v, 100.0, "never past the end");
        // The wheel over the groove: a notch down takes one.
        let over = egui::pos2(rail.center().x, rail.center().y);
        frame(&ctx, &mut v, vec![egui::Event::PointerMoved(over)]);
        let (_, resp) = frame(
            &ctx,
            &mut v,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                phase: egui::TouchPhase::Move,
                delta: egui::vec2(0.0, -1.0),
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(resp.changed());
        assert_eq!(v, 99.0, "one notch down");
        frame(
            &ctx,
            &mut v,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                phase: egui::TouchPhase::Move,
                delta: egui::vec2(0.0, -1.0),
                modifiers: egui::Modifiers::SHIFT,
            }],
        );
        assert_eq!(v, 89.0, "Shift: ten");
        let (out, _) = frame(&ctx, &mut v, Vec::new());
        assert!(texts(&out).iter().any(|t| t == "89 %"), "the box follows");
    }

    /// A value typed in the box applies to the slider, clamped to the
    /// range; a step snaps a drag to its multiples.
    #[test]
    fn the_box_sets_the_value_and_the_step_snaps_it() {
        let ctx = ctx();
        let mut v = 40.0;
        frame(&ctx, &mut v, Vec::new());
        frame(&ctx, &mut v, Vec::new());
        let rail = rail_rect(&ctx, "Detail").expect("drawn");
        // Click in the box, type a value over the selected text, Enter.
        let in_box = egui::pos2(rail.right() + BOX_GAP + 20.0, rail.center().y);
        frame(&ctx, &mut v, vec![egui::Event::PointerMoved(in_box)]);
        frame(&ctx, &mut v, vec![press(in_box, true)]);
        frame(&ctx, &mut v, vec![press(in_box, false)]);
        frame(&ctx, &mut v, Vec::new());
        frame(&ctx, &mut v, vec![egui::Event::Text("250".into())]);
        let (_, resp) = frame(
            &ctx,
            &mut v,
            vec![key(egui::Key::Enter, true), key(egui::Key::Enter, false)],
        );
        assert!(resp.changed(), "Enter applies the typed value");
        assert_eq!(v, 100.0, "clamped to the range");
        // A step of 25 snaps a drag to 0, 25, 50, 75, 100.
        let mut s = 0.0f64;
        let show = |ctx: &egui::Context, s: &mut f64, events| {
            let mut out = ctx.run_ui(input(events), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    column(ui, |ui| {
                        ui.add(
                            Slider::new(&mut *s, 0.0..=100.0)
                                .text("Coarse")
                                .step_by(25.0),
                        );
                    });
                });
            });
            out.textures_delta.clear();
        };
        show(&ctx, &mut s, Vec::new());
        show(&ctx, &mut s, Vec::new());
        let rail = rail_rect(&ctx, "Coarse").expect("drawn");
        let at = egui::pos2(thumb_x(rail, 60.0), rail.center().y);
        show(&ctx, &mut s, vec![egui::Event::PointerMoved(at)]);
        show(&ctx, &mut s, vec![press(at, true)]);
        assert_eq!(s, 50.0, "60 snaps to 50");
        show(&ctx, &mut s, vec![press(at, false)]);
        // An integer slider rounds, and a value given outside the range
        // is brought into it.
        let mut n = 500i32;
        let mut out = ctx.run_ui(input(Vec::new()), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                ui.add(Slider::new(&mut n, 0..=255).text("Threshold"));
            });
        });
        out.textures_delta.clear();
        assert_eq!(n, 255);
    }

    /// A click or a drag on the groove leaves the keyboard focus alone,
    /// so the canvas keeps its shortcuts; the wheel over a slider without
    /// the focus (its groove or its box) changes nothing and is left to
    /// the panel around; once dragged or focused by Tab, it steps.
    #[test]
    fn a_click_leaves_the_keys_and_the_wheel_to_the_panel() {
        let ctx = ctx();
        let mut v = 40.0;
        frame(&ctx, &mut v, Vec::new());
        frame(&ctx, &mut v, Vec::new());
        let rail = rail_rect(&ctx, "Detail").expect("drawn");
        let at = egui::pos2(thumb_x(rail, 60.0), rail.center().y);
        frame(&ctx, &mut v, vec![egui::Event::PointerMoved(at)]);
        frame(&ctx, &mut v, vec![press(at, true)]);
        frame(&ctx, &mut v, vec![press(at, false)]);
        frame(&ctx, &mut v, Vec::new());
        assert_eq!(v, 60.0);
        assert!(
            !ctx.egui_wants_keyboard_input(),
            "a click does not keep the keyboard"
        );
        // An arrow key is the canvas's, not the slider's.
        frame(
            &ctx,
            &mut v,
            vec![
                key(egui::Key::ArrowRight, true),
                key(egui::Key::ArrowRight, false),
            ],
        );
        assert_eq!(v, 60.0);
        // The wheel over the groove, then over the box: nothing changes,
        // and nothing takes the wheel from the panel.
        let in_box = egui::pos2(rail.right() + BOX_GAP + 20.0, rail.center().y);
        for over in [at, in_box] {
            frame(&ctx, &mut v, vec![egui::Event::PointerMoved(over)]);
            frame(&ctx, &mut v, vec![wheel(true, egui::Modifiers::NONE)]);
            assert_eq!(v, 60.0, "{over:?}");
            assert!(!crate::ui::field::wheel_taken(&ctx), "{over:?}");
        }
        // While the thumb is held, the wheel steps it.
        frame(&ctx, &mut v, vec![egui::Event::PointerMoved(at)]);
        frame(&ctx, &mut v, vec![press(at, true)]);
        let near = at + egui::vec2(8.0, 0.0);
        frame(&ctx, &mut v, vec![egui::Event::PointerMoved(near)]);
        let held = v;
        frame(&ctx, &mut v, vec![wheel(false, egui::Modifiers::NONE)]);
        assert_eq!(v, held + 1.0, "the wheel steps a dragged slider");
        frame(&ctx, &mut v, vec![press(near, false)]);
        frame(&ctx, &mut v, Vec::new());
        assert!(!ctx.egui_wants_keyboard_input());
    }

    /// Ctrl with the wheel: a tenth of a step on a slider without a
    /// step, one step on a stepped (or integer) one, where a tenth would
    /// snap back and do nothing.
    #[test]
    fn ctrl_wheel_steps_finely_or_by_the_smallest_step() {
        let ctx = ctx();
        let show = |ctx: &egui::Context, v: &mut f64, stepped: bool, events| {
            let mut out = ctx.run_ui(input(events), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    column(ui, |ui| {
                        let s = Slider::new(&mut *v, 0.0..=100.0).text("Fine");
                        ui.add(if stepped { s.step_by(1.0) } else { s });
                    });
                });
            });
            out.textures_delta.clear();
        };
        for (stepped, expect) in [(false, 40.1), (true, 41.0)] {
            let mut v = 40.0;
            show(&ctx, &mut v, stepped, Vec::new());
            show(&ctx, &mut v, stepped, Vec::new());
            let rail = rail_rect(&ctx, "Fine").expect("drawn");
            // Tab to the groove, then the wheel over it with Ctrl.
            show(
                &ctx,
                &mut v,
                stepped,
                vec![key(egui::Key::Tab, true), key(egui::Key::Tab, false)],
            );
            show(
                &ctx,
                &mut v,
                stepped,
                vec![egui::Event::PointerMoved(rail.center())],
            );
            show(
                &ctx,
                &mut v,
                stepped,
                vec![wheel(false, egui::Modifiers::COMMAND)],
            );
            assert!((v - expect).abs() < 1e-9, "stepped {stepped}: {v}");
            ctx.memory_mut(|m| m.stop_text_input());
        }
        let mut i = 3i32;
        assert_eq!(Slider::new(&mut i, 0..=10).fine_step(), 1.0);
    }

    /// A value typed in the box takes the slider's step: 12.5 in a
    /// whole-number slider is 13, the value the box shows.
    #[test]
    fn a_typed_value_snaps_to_the_step() {
        let ctx = ctx();
        let mut v = 40.0;
        frame(&ctx, &mut v, Vec::new());
        frame(&ctx, &mut v, Vec::new());
        let rail = rail_rect(&ctx, "Detail").expect("drawn");
        let in_box = egui::pos2(rail.right() + BOX_GAP + 20.0, rail.center().y);
        frame(&ctx, &mut v, vec![egui::Event::PointerMoved(in_box)]);
        frame(&ctx, &mut v, vec![press(in_box, true)]);
        frame(&ctx, &mut v, vec![press(in_box, false)]);
        frame(&ctx, &mut v, Vec::new());
        frame(&ctx, &mut v, vec![egui::Event::Text("12.5".into())]);
        let (_, resp) = frame(
            &ctx,
            &mut v,
            vec![key(egui::Key::Enter, true), key(egui::Key::Enter, false)],
        );
        assert!(resp.changed());
        assert_eq!(v, 13.0);
        let (out, _) = frame(&ctx, &mut v, Vec::new());
        assert!(texts(&out).iter().any(|t| t == "13 %"), "the box shows it");
    }

    /// A disabled slider takes no input.
    #[test]
    fn a_disabled_slider_keeps_its_value() {
        let ctx = ctx();
        let mut v = 40.0;
        let show = |ctx: &egui::Context, v: &mut f64, events| {
            let mut out = ctx.run_ui(input(events), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    column(ui, |ui| {
                        ui.add_enabled(false, Slider::new(&mut *v, 0.0..=100.0).text("Off"));
                    });
                });
            });
            out.textures_delta.clear();
        };
        show(&ctx, &mut v, Vec::new());
        show(&ctx, &mut v, Vec::new());
        let rail = rail_rect(&ctx, "Off").expect("drawn");
        let at = egui::pos2(thumb_x(rail, 80.0), rail.center().y);
        show(&ctx, &mut v, vec![egui::Event::PointerMoved(at)]);
        show(&ctx, &mut v, vec![press(at, true)]);
        show(&ctx, &mut v, vec![press(at, false)]);
        show(
            &ctx,
            &mut v,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                phase: egui::TouchPhase::Move,
                delta: egui::vec2(0.0, -1.0),
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(v, 40.0);
    }

    /// A converted dialog: the Export dialog's Quality slider (JPEG)
    /// sets the export quality from a press on its groove.
    #[test]
    fn the_export_dialog_quality_slider_sets_the_quality() {
        use crate::ui::dialogs::{Dialog, ExportState};
        crate::i18n::set_language("en");
        let ctx = ctx();
        let mut app = crate::app::App::headless();
        app.dialog = Dialog::Export(ExportState {
            format: 3,
            quality: 80,
            ..Default::default()
        });
        let quality = |app: &crate::app::App| match &app.dialog {
            Dialog::Export(st) => st.quality,
            _ => 0,
        };
        let frame = |app: &mut crate::app::App, events: Vec<egui::Event>| {
            let input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400.0, 900.0),
                )),
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| crate::ui::dialogs::show(app, &ui.ctx().clone()));
            out.textures_delta.clear();
        };
        frame(&mut app, Vec::new());
        frame(&mut app, Vec::new());
        let rail = rail_rect(&ctx, "Quality").expect("the Quality slider is drawn");
        assert_eq!(quality(&app), 80);
        // A press a fifth of the way along the groove: 1 to 100 gives
        // about 21.
        let half = THUMB_W / 2.0;
        let at = egui::pos2(
            rail.left() + half + (rail.width() - THUMB_W) * 0.2,
            rail.center().y,
        );
        frame(&mut app, vec![egui::Event::PointerMoved(at)]);
        frame(&mut app, vec![press(at, true)]);
        frame(&mut app, vec![press(at, false)]);
        let q = quality(&app);
        assert!((18..=24).contains(&q), "quality {q}");
    }

    /// Tick spacing: round majors about every 45 points, minors in
    /// fourths or fifths, never finer than the step or three points.
    #[test]
    fn tick_spacing_is_round_and_readable() {
        assert_eq!(linear_spacing(100.0, 220.0, None), (20.0, 5.0));
        assert_eq!(linear_spacing(200.0, 220.0, None), (50.0, 10.0));
        assert_eq!(linear_spacing(255.0, 100.0, Some(1.0)), (100.0, 20.0));
        // A small integer range: whole numbers only.
        assert_eq!(linear_spacing(14.0, 100.0, Some(1.0)), (5.0, 1.0));
        // Too narrow for minors: majors only.
        let (major, minor) = linear_spacing(100.0, 40.0, None);
        assert_eq!(major, minor);
        // A float range.
        let (major, minor) = linear_spacing(4.9, 220.0, Some(0.05));
        assert!((major - 1.0).abs() < 1e-9 && (minor - 0.2).abs() < 1e-9);
        assert_eq!(nice_near(0.0), 1.0);
        assert_eq!(nice_near(3.0), 2.0);
        assert_eq!(nice_near(3.5), 5.0);
        assert_eq!(nice_near(0.011), 0.01);
        assert_eq!(nice_near(8.0), 10.0);
        // A logarithmic range ticks at 1, 2, 5 per decade.
        let mut v = 10.0;
        let s = Slider::new(&mut v, 1.0..=100.0).logarithmic(true);
        let mut ticks = Vec::new();
        s.for_each_tick(120.0, |v, major| ticks.push((v, major)));
        assert_eq!(
            ticks,
            vec![
                (1.0, true),
                (2.0, false),
                (5.0, false),
                (10.0, true),
                (20.0, false),
                (50.0, false),
                (100.0, true)
            ]
        );
        // Key steps: the step, one for integers, a round hundredth else.
        let mut i = 3i32;
        assert_eq!(Slider::new(&mut i, 0..=10).key_step(), 1.0);
        let mut f = 3.0f64;
        assert!((Slider::new(&mut f, 0.1..=5.0).key_step() - 0.05).abs() < 1e-9);
        assert_eq!(Slider::new(&mut f, 0.0..=100.0).key_step(), 1.0);
    }
}
