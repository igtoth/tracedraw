//! Number fields for the bars: a white box with the value at the left,
//! typed into, and a pair of spin arrows at its right. Enter or leaving
//! the box applies the value, Esc puts it back. Sums such as `10+5` or
//! `2*3,5` work, a comma or a point marks decimals, and a distance field
//! takes a value in another unit (`1 in` in a millimetre field). The
//! arrows and the mouse wheel over the box step the value (one unit, or
//! the step the field was given; Ctrl a tenth, Shift ten times).
//!
//! `Response::changed()` is the one sign that the value was applied, typed
//! or stepped: a step leaves the focus where it was, so a caller waiting
//! for `lost_focus()` takes the first step only (when the step took the
//! focus from the box) and then none.

use crate::app::Units;
use crate::ui::tips::RichTip;
use egui::{emath::Numeric, Response, TextEdit, Ui, Widget};
use std::ops::RangeInclusive;

/// A number field; built like `egui::DragValue`.
pub struct NumField<'a, N: Numeric> {
    value: &'a mut N,
    range: Option<(f64, f64)>,
    suffix: String,
    /// A label shown just before the box ("X:").
    prefix: String,
    min_decimals: usize,
    max_decimals: Option<usize>,
    /// Millimetres per unit of the field, for distances typed with a unit.
    unit_mm: Option<f64>,
    /// Width in points when the field is not given a size by its parent.
    width: f32,
    /// The selection has different values here: the box is empty and a
    /// value typed applies even when it equals the one held.
    mixed: bool,
    /// What the spin arrows and the wheel add or take, in the field's
    /// unit (1 unless given).
    step: f64,
}

/// The width of the spin arrows at the right of a field, points.
pub const SPIN_W: f32 = 11.0;

/// The width of a field added without a size (see [`NumField::width`]).
pub const DEFAULT_WIDTH: f32 = 64.0;

impl<'a, N: Numeric> NumField<'a, N> {
    pub fn new(value: &'a mut N) -> Self {
        NumField {
            value,
            range: None,
            suffix: String::new(),
            prefix: String::new(),
            min_decimals: 0,
            max_decimals: None,
            unit_mm: None,
            width: DEFAULT_WIDTH,
            mixed: false,
            step: 1.0,
        }
    }

    /// What the spin arrows and the mouse wheel add or take (1 unless
    /// given).
    pub fn step(mut self, step: f64) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        self
    }

    /// Show the field empty because the selected objects differ in this
    /// value; any value typed then applies to all of them.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// The field's width when it is added on its own; a field placed with
    /// `ui.add_sized` (or in any justified layout) fills the room it gets.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn range<R: Numeric>(mut self, range: RangeInclusive<R>) -> Self {
        self.range = Some((range.start().to_f64(), range.end().to_f64()));
        self
    }

    /// The step of the arrows and the wheel (`DragValue` calls it the
    /// speed).
    pub fn speed(self, speed: impl Into<f64>) -> Self {
        self.step(speed.into())
    }

    /// A label just before the box, such as `X:`.
    pub fn prefix(mut self, prefix: impl ToString) -> Self {
        self.prefix = prefix.to_string().trim_end().to_string();
        self
    }

    pub fn suffix(mut self, suffix: impl ToString) -> Self {
        self.suffix = suffix.to_string();
        self
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

    /// A distance field whose unit is `mm` millimetres: values typed with
    /// another unit's name are converted.
    pub fn unit_mm(mut self, mm: f64) -> Self {
        self.unit_mm = Some(mm);
        self
    }

    fn text(&self) -> String {
        if self.mixed {
            return String::new();
        }
        format!(
            "{}{}",
            format_number(
                self.value.to_f64(),
                N::INTEGRAL,
                self.min_decimals,
                self.max_decimals
            ),
            self.suffix
        )
    }
}

/// `v` with at least `min` and at most `max` decimals (3 when not given),
/// trailing zeros beyond `min` dropped.
pub fn format_number(v: f64, integral: bool, min: usize, max: Option<usize>) -> String {
    if integral {
        return format!("{}", v.round() as i64);
    }
    let max = max.unwrap_or(3).max(min);
    let mut s = format!("{v:.max$}");
    if s.contains('.') {
        let keep = s.find('.').map(|i| i + 1 + min).unwrap_or(s.len());
        while s.len() > keep && s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

impl<N: Numeric> Widget for NumField<'_, N> {
    fn ui(self, ui: &mut Ui) -> Response {
        // Fill a sized slot; otherwise keep to the field's own width, so a
        // field in a row of controls does not take the rest of the row.
        let slot = ui
            .layout()
            .horizontal_justify()
            .then(|| ui.available_width());
        // The prefix, the box and the arrows stay together on one line, in
        // a row of their own: in a column (or a justified slot) they would
        // land under each other or the prefix would take the whole slot,
        // and in a grid each would take a cell of its own, the arrows
        // standing apart from the box at the right of the next column
        // (#948).
        ui.horizontal(|ui| self.row(ui, slot)).inner
    }
}

impl<N: Numeric> NumField<'_, N> {
    /// The prefix, the box and the arrows, laid out left to right; `slot`
    /// is the width they share when the parent gave one.
    fn row(self, ui: &mut Ui, mut slot: Option<f32>) -> Response {
        let id = ui.next_auto_id().with("num_field");
        let shown = self.text();
        // While typing, the text lives in memory until it is applied.
        let stored = ui.data(|d| d.get_temp::<String>(id));
        let mut buf = stored.clone().unwrap_or_else(|| shown.clone());
        if !self.prefix.is_empty() {
            let label = ui.label(&self.prefix);
            if let Some(s) = &mut slot {
                *s -= label.rect.width() + ui.spacing().item_spacing.x;
            }
        }
        // The spin arrows take their width at the right of the box.
        let width = (slot.unwrap_or(self.width) - SPIN_W).max(16.0);
        let spacing = ui.spacing().item_spacing;
        ui.spacing_mut().item_spacing.x = 0.0;
        let mut resp = ui.add(
            TextEdit::singleline(&mut buf)
                .id(id)
                .margin(egui::Margin::symmetric(4, 0))
                .vertical_align(egui::Align::Center)
                .desired_width(width),
        );
        // The arrows: up over down, each half the box's height.
        let (spin, spin_resp) =
            ui.allocate_exact_size(egui::vec2(SPIN_W, resp.rect.height()), egui::Sense::click());
        ui.spacing_mut().item_spacing = spacing;
        // The direction of a step and the modifiers held for it.
        let mut stepped: Option<(f64, egui::Modifiers)> = None;
        if ui.is_rect_visible(spin) {
            let p = ui.painter();
            p.rect_filled(spin, 0.0, crate::theme::Tokens::field());
            let line = egui::Stroke::new(1.0, crate::theme::Tokens::control_border());
            p.line_segment([spin.left_top(), spin.left_bottom()], line);
            let hover = spin_resp.hover_pos();
            let halves = [
                (
                    egui::Rect::from_min_max(spin.min, egui::pos2(spin.max.x, spin.center().y)),
                    1.0,
                ),
                (
                    egui::Rect::from_min_max(egui::pos2(spin.min.x, spin.center().y), spin.max),
                    -1.0,
                ),
            ];
            for (half, dir) in halves {
                let over = hover.is_some_and(|h| half.contains(h));
                if over {
                    p.rect_filled(half.shrink(1.0), 0.0, crate::theme::Tokens::icon_off());
                }
                let c = half.center();
                let col = if over {
                    crate::theme::Tokens::text()
                } else {
                    crate::theme::Tokens::text_dim()
                };
                let (a, b, t) = (
                    c + egui::vec2(-2.5, 1.5 * dir as f32),
                    c + egui::vec2(2.5, 1.5 * dir as f32),
                    c + egui::vec2(0.0, -1.5 * dir as f32),
                );
                p.add(egui::epaint::PathShape::convex_polygon(
                    vec![a, b, t],
                    col,
                    egui::Stroke::NONE,
                ));
                if spin_resp.clicked()
                    && spin_resp
                        .interact_pointer_pos()
                        .is_some_and(|q| half.contains(q))
                {
                    stepped = Some((dir, ui.input(|i| i.modifiers)));
                }
            }
        }
        // The wheel over the box or the arrows, a step a notch; the bar or
        // panel around the field does not scroll with it (#783).
        if ui.rect_contains_pointer(resp.rect) || ui.rect_contains_pointer(spin) {
            if let Some(steps) = wheel_steps(ui, id.with("wheel")) {
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
                if steps != 0 {
                    stepped = Some((f64::from(steps), mods));
                }
            }
        } else {
            ui.data_mut(|d| d.remove::<f32>(id.with("wheel")));
        }
        if let Some((dir, mods)) = stepped {
            let factor = if mods.command {
                0.1
            } else if mods.shift {
                10.0
            } else {
                1.0
            };
            let mut v = self.value.to_f64() + dir * self.step * factor;
            if let Some((lo, hi)) = self.range {
                v = v.clamp(lo.min(hi), hi.max(lo));
            }
            if N::INTEGRAL {
                v = v.round();
            }
            // A value typed and not applied yet goes; the step counts
            // from the value held.
            ui.data_mut(|d| d.remove::<String>(id));
            if resp.has_focus() {
                resp.surrender_focus();
            }
            if v != self.value.to_f64() {
                *self.value = N::from_f64(v);
                resp.mark_changed();
            }
            return with_arrows(resp, spin_resp);
        }
        if self.mixed {
            resp = resp.rich_tip(crate::i18n::tr("docker.mixed_tip"));
        }
        // Typing does not change the value; applying it does.
        resp.flags.set(egui::response::Flags::CHANGED, false);
        if resp.has_focus() && stored.is_none() {
            // Just focused: the whole value is selected, ready to be typed
            // over.
            if let Some(mut state) = TextEdit::load_state(ui.ctx(), id) {
                let all = egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(buf.chars().count()),
                );
                state.cursor.set_char_range(Some(all));
                state.store(ui.ctx(), id);
            }
        }
        if resp.has_focus() {
            ui.data_mut(|d| d.insert_temp(id, buf.clone()));
        } else if stored.is_some() || resp.lost_focus() {
            ui.data_mut(|d| d.remove::<String>(id));
            let escaped = ui.input(|i| i.key_pressed(egui::Key::Escape));
            if !escaped && buf != shown {
                if let Some(mut v) = parse(&buf, self.unit_mm) {
                    if let Some((lo, hi)) = self.range {
                        v = v.clamp(lo.min(hi), hi.max(lo));
                    }
                    if N::INTEGRAL {
                        v = v.round();
                    }
                    if v != self.value.to_f64() || self.mixed {
                        *self.value = N::from_f64(v);
                        resp.mark_changed();
                    }
                }
            }
        }
        with_arrows(resp, spin_resp)
    }
}

/// The box's response, also hovered over its spin arrows: a tip given
/// to the field shows over the arrows too. The focus stays the box's.
fn with_arrows(mut resp: Response, arrows: Response) -> Response {
    // Only the hover joins: a click on an arrow is a step, not a click
    // on the box.
    if arrows.hovered() {
        resp.flags.set(egui::response::Flags::HOVERED, true);
    }
    resp
}

/// The points a wheel or touchpad sends for one notch when it counts in
/// points (browsers send 100 points or 3 lines a notch).
pub const WHEEL_POINTS_PER_NOTCH: f32 = 50.0;

/// The memory key that says a field took the wheel and the smoothed
/// scroll of that turn is still running.
fn wheel_taken_id() -> egui::Id {
    egui::Id::new("field_wheel_taken")
}

/// The wheel turned over a field, list or tab strip that steps with it:
/// nothing around it scrolls with that turn. The smoothed scroll egui
/// makes of a notch reaches the next frames too, and those frames slid the
/// bar or panel holding the field, carrying the field away from under the
/// pointer after a step or two (#783); [`hold_taken_wheel`] keeps them
/// from scrolling anything.
pub fn take_wheel(ui: &Ui) {
    ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
    ui.ctx().data_mut(|d| d.insert_temp(wheel_taken_id(), true));
}

/// Called at the start of every frame, before anything reads the scroll:
/// while the smoothed scroll of a notch a field took is still running,
/// it scrolls nothing. A new turn of the wheel ends the hold (a field
/// under the pointer takes that one again).
pub fn hold_taken_wheel(ctx: &egui::Context) {
    let id = wheel_taken_id();
    if !ctx.data(|d| d.get_temp::<bool>(id)).unwrap_or(false) {
        return;
    }
    let (turned, scrolling) = ctx.input(|i| {
        (
            i.raw
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::MouseWheel { .. })),
            i.is_scrolling(),
        )
    });
    if turned || !scrolling {
        ctx.data_mut(|d| d.remove::<bool>(id));
        return;
    }
    ctx.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
}

/// The wheel notches turned this frame over a widget that steps with the
/// wheel, up positive; `None` when the wheel did not turn. An event of a
/// notch or more is one step whatever its size; parts of a notch from a
/// touchpad or a fine wheel add up (in memory under `acc_id`) until they
/// make one, and start again when the wheel turns the other way. A turn
/// takes the wheel ([`take_wheel`]), so nothing around scrolls with it.
pub fn wheel_steps(ui: &Ui, acc_id: egui::Id) -> Option<i32> {
    let deltas: Vec<f32> = ui.input(|i| {
        i.raw
            .events
            .iter()
            .filter_map(|e| match e {
                egui::Event::MouseWheel { unit, delta, .. } if delta.y != 0.0 => Some(match unit {
                    egui::MouseWheelUnit::Line if delta.y.abs() >= 1.0 => delta.y.signum(),
                    egui::MouseWheelUnit::Line => delta.y,
                    egui::MouseWheelUnit::Point if delta.y.abs() >= WHEEL_POINTS_PER_NOTCH => {
                        delta.y.signum()
                    }
                    egui::MouseWheelUnit::Point => delta.y / WHEEL_POINTS_PER_NOTCH,
                    egui::MouseWheelUnit::Page => delta.y.signum(),
                }),
                _ => None,
            })
            .collect()
    });
    if deltas.is_empty() {
        return None;
    }
    take_wheel(ui);
    let mut acc = ui.data(|d| d.get_temp::<f32>(acc_id)).unwrap_or(0.0);
    let mut steps = 0i32;
    for d in deltas {
        if acc != 0.0 && acc.signum() != d.signum() {
            acc = 0.0;
        }
        acc += d;
        let whole = acc.trunc();
        steps += whole as i32;
        acc -= whole;
    }
    ui.data_mut(|d| d.insert_temp(acc_id, acc));
    Some(steps)
}

/// Cover a colour button whose objects have different colours: an empty
/// swatch with a slash, so no colour is shown; the button still opens
/// its picker, and the colour picked goes to every object.
pub fn mixed_swatch(ui: &Ui, rect: egui::Rect) {
    let p = ui.painter();
    let r = rect.shrink(1.0);
    p.rect_filled(r, 2.0, crate::theme::Tokens::field());
    let line = egui::Stroke::new(1.0, crate::theme::Tokens::text_dim());
    p.rect_stroke(r, 2.0, line, egui::StrokeKind::Inside);
    p.line_segment([r.left_bottom(), r.right_top()], line);
}

/// The dim "Mixed" word shown where the selected objects differ, with its
/// explanation on hover.
pub fn mixed_note(ui: &mut Ui) -> Response {
    ui.label(
        egui::RichText::new(crate::i18n::tr("docker.mixed"))
            .color(crate::theme::Tokens::text_dim())
            .italics(),
    )
    .rich_tip(crate::i18n::tr("docker.mixed_tip"))
}

/// Millimetres per unit for a unit name typed after a value.
fn unit_named(name: &str) -> Option<f64> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    if name == "\"" || name.eq_ignore_ascii_case("in") || name.eq_ignore_ascii_case("inch") {
        return Some(Units::Inches.mm());
    }
    Units::ALL
        .into_iter()
        .find(|u| u.short().eq_ignore_ascii_case(name) || u.id().eq_ignore_ascii_case(name))
        .map(|u| u.mm())
}

/// The value typed in `text`: numbers joined by `+ - * /` (with the usual
/// precedence and brackets), a comma or a point as the decimal mark,
/// then optionally a unit name, converted to the field's unit when the
/// field measures distances (`unit_mm`). Other trailing marks (`%`, `°`,
/// an unknown unit) are ignored.
pub fn parse(text: &str, unit_mm: Option<f64>) -> Option<f64> {
    let t = text.trim();
    // Split off what follows the expression.
    let end = t
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || "+-*/.,() eE".contains(*c)))
        .map(|(i, _)| i)
        .unwrap_or(t.len());
    let (expr, tail) = t.split_at(end);
    // A trailing "e" would be read as an exponent; keep it with the unit.
    let expr = expr.trim();
    let v = Expr::new(expr).parse()?;
    if !v.is_finite() {
        return None;
    }
    match (unit_mm, unit_named(tail)) {
        (Some(field), Some(typed)) if field > 0.0 => Some(v * typed / field),
        _ => Some(v),
    }
}

/// A small recursive-descent reader for sums and products.
struct Expr<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Expr<'a> {
    fn new(s: &'a str) -> Self {
        Expr {
            s: s.as_bytes(),
            i: 0,
        }
    }

    fn parse(mut self) -> Option<f64> {
        let v = self.sum()?;
        self.ws();
        (self.i == self.s.len()).then_some(v)
    }

    fn ws(&mut self) {
        while self.s.get(self.i) == Some(&b' ') {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }

    fn sum(&mut self) -> Option<f64> {
        let mut v = self.product()?;
        while let Some(c) = self.peek() {
            match c {
                b'+' => {
                    self.i += 1;
                    v += self.product()?;
                }
                b'-' => {
                    self.i += 1;
                    v -= self.product()?;
                }
                _ => break,
            }
        }
        Some(v)
    }

    fn product(&mut self) -> Option<f64> {
        let mut v = self.unary()?;
        while let Some(c) = self.peek() {
            match c {
                b'*' => {
                    self.i += 1;
                    v *= self.unary()?;
                }
                b'/' => {
                    self.i += 1;
                    let d = self.unary()?;
                    if d == 0.0 {
                        return None;
                    }
                    v /= d;
                }
                _ => break,
            }
        }
        Some(v)
    }

    fn unary(&mut self) -> Option<f64> {
        match self.peek()? {
            b'-' => {
                self.i += 1;
                Some(-self.unary()?)
            }
            b'+' => {
                self.i += 1;
                self.unary()
            }
            b'(' => {
                self.i += 1;
                let v = self.sum()?;
                (self.peek()? == b')').then(|| {
                    self.i += 1;
                    v
                })
            }
            _ => self.number(),
        }
    }

    fn number(&mut self) -> Option<f64> {
        self.ws();
        let start = self.i;
        let mut text = String::new();
        let mut seen_mark = false;
        while let Some(&c) = self.s.get(self.i) {
            match c {
                b'0'..=b'9' => text.push(c as char),
                b'.' | b',' if !seen_mark => {
                    seen_mark = true;
                    text.push('.');
                }
                b'e' | b'E'
                    if !text.is_empty()
                        && self
                            .s
                            .get(self.i + 1)
                            .is_some_and(|n| n.is_ascii_digit() || *n == b'-' || *n == b'+') =>
                {
                    text.push('e');
                    self.i += 1;
                    if let Some(&s) = self.s.get(self.i) {
                        if s == b'-' || s == b'+' {
                            text.push(s as char);
                            self.i += 1;
                        }
                    }
                    continue;
                }
                _ => break,
            }
            self.i += 1;
        }
        if self.i == start {
            return None;
        }
        text.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One notch of a mouse wheel, as Windows sends it (one line).
    fn notch(dy: f32) -> egui::Event {
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, dy),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        }
    }

    /// The wheel over a number field in a bar longer than its window (the
    /// property bar, which slides with the wheel) steps the field every
    /// notch and leaves the bar where it is, however many frames the
    /// smoothed scroll of a notch takes to run out: the bar slid on the
    /// frames after a notch, carrying the field away from under the
    /// pointer, so the wheel stopped working on it after a step or two
    /// (#783). Away from the fields the wheel still slides the bar.
    #[test]
    fn the_wheel_steps_a_field_in_a_long_bar_without_sliding_the_bar() {
        let ctx = crate::theme::ui_context();
        let mut v = 100.0f64;
        // Frames a sixtieth of a second apart, as on screen.
        let clock = std::cell::Cell::new(0.0f64);
        let run = |v: &mut f64, events: Vec<egui::Event>| -> egui::Rect {
            let mut field = egui::Rect::NOTHING;
            clock.set(clock.get() + 1.0 / 60.0);
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 200.0),
                    )),
                    events,
                    time: Some(clock.get()),
                    ..Default::default()
                },
                |ui| {
                    hold_taken_wheel(ui.ctx());
                    crate::ui::overflow::row(ui, "bar", |ui| {
                        ui.add_space(150.0);
                        field = ui.add(NumField::new(v).range(0.0..=360.0)).rect;
                        // Wider than the window, as a full bar is.
                        ui.add_space(900.0);
                    });
                },
            );
            out.textures_delta.clear();
            field
        };
        for _ in 0..3 {
            run(&mut v, Vec::new());
        }
        let start = run(&mut v, Vec::new());
        assert!(start.is_positive(), "{start:?}");
        run(&mut v, vec![egui::Event::PointerMoved(start.center())]);
        for k in 1..=8 {
            // Down: up would push the bar back past its start, where it
            // cannot go.
            let now = run(&mut v, vec![notch(-1.0)]);
            assert!(
                (now.left() - start.left()).abs() < 0.5,
                "notch {k}: {now:?}"
            );
            // The frames between two notches, while the smoothed scroll
            // of the last one runs.
            for f in 0..4 {
                let now = run(&mut v, Vec::new());
                assert!(
                    (now.left() - start.left()).abs() < 0.5,
                    "notch {k}, frame {f}: the bar slid to {now:?}"
                );
            }
            assert_eq!(v, 100.0 - k as f64, "notch {k}");
        }
        // The smoothed scroll runs out with the bar in place.
        for _ in 0..40 {
            assert!((run(&mut v, Vec::new()).left() - start.left()).abs() < 0.5);
        }
        // Two notches in one frame (a slow frame): two steps.
        run(&mut v, vec![notch(-1.0), notch(-1.0)]);
        assert_eq!(v, 90.0);
        for _ in 0..40 {
            run(&mut v, Vec::new());
        }
        // Over the bar away from the field the wheel slides the bar.
        let away = egui::pos2(start.left() - 60.0, start.center().y);
        run(&mut v, vec![egui::Event::PointerMoved(away)]);
        run(&mut v, vec![notch(-1.0)]);
        let mut moved = start;
        for _ in 0..20 {
            moved = run(&mut v, Vec::new());
        }
        assert!(moved.left() < start.left() - 10.0, "{moved:?}");
        assert_eq!(v, 90.0);
    }

    /// The same in a docker or dialog that scrolls up and down: the wheel
    /// over a field steps it and the panel stays put.
    #[test]
    fn the_wheel_steps_a_field_in_a_scrolling_panel_without_scrolling_it() {
        let ctx = crate::theme::ui_context();
        let mut v = 5i32;
        let clock = std::cell::Cell::new(0.0f64);
        let run = |v: &mut i32, events: Vec<egui::Event>| -> egui::Rect {
            let mut field = egui::Rect::NOTHING;
            clock.set(clock.get() + 1.0 / 60.0);
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(300.0, 300.0),
                    )),
                    events,
                    time: Some(clock.get()),
                    ..Default::default()
                },
                |ui| {
                    hold_taken_wheel(ui.ctx());
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.add_space(100.0);
                        field = ui.add(NumField::new(v).range(0..=100)).rect;
                        ui.add_space(2000.0);
                    });
                },
            );
            out.textures_delta.clear();
            field
        };
        for _ in 0..3 {
            run(&mut v, Vec::new());
        }
        let start = run(&mut v, Vec::new());
        run(&mut v, vec![egui::Event::PointerMoved(start.center())]);
        for k in 1..=5 {
            run(&mut v, vec![notch(-1.0)]);
            for _ in 0..6 {
                let now = run(&mut v, Vec::new());
                assert!((now.top() - start.top()).abs() < 0.5, "notch {k}: {now:?}");
            }
            assert_eq!(v, 5 - k, "notch {k}");
        }
    }

    /// A touchpad or a fine wheel sends parts of a notch: they add up to
    /// one step instead of a step each.
    #[test]
    fn parts_of_a_notch_add_up_to_one_step() {
        let ctx = crate::theme::ui_context();
        let mut v = 0.0f64;
        let mut rect = egui::Rect::NOTHING;
        let mut run = |v: &mut f64, events: Vec<egui::Event>| {
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(300.0, 200.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| rect = ui.add(NumField::new(v)).rect,
            );
            out.textures_delta.clear();
            rect
        };
        run(&mut v, Vec::new());
        let r = run(&mut v, Vec::new());
        run(&mut v, vec![egui::Event::PointerMoved(r.center())]);
        for _ in 0..3 {
            run(&mut v, vec![notch(0.25)]);
            assert_eq!(v, 0.0);
        }
        run(&mut v, vec![notch(0.25)]);
        assert_eq!(v, 1.0);
        run(
            &mut v,
            vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, 100.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(v, 2.0);
    }

    /// The spin arrows at the right of a field and the mouse wheel over
    /// it step the value: a unit (or the field's step), ten with Shift,
    /// a tenth with Ctrl, within the range; a whole-number field stays
    /// whole.
    #[test]
    fn arrows_and_the_wheel_step_the_value() {
        let ctx = crate::theme::ui_context();
        let mut v = 10.0f64;
        let mut n = 3i32;
        let origin = egui::pos2(100.0, 100.0);
        let run =
            |v: &mut f64, n: &mut i32, events: Vec<egui::Event>| -> (egui::Rect, egui::Rect) {
                let mut rects = (egui::Rect::NOTHING, egui::Rect::NOTHING);
                let mut out = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(600.0, 400.0),
                        )),
                        events,
                        ..Default::default()
                    },
                    |ui| {
                        egui::Area::new(egui::Id::new("fields"))
                            .fixed_pos(origin)
                            .show(ui.ctx(), |ui| {
                                ui.horizontal(|ui| {
                                    let a = ui.add(NumField::new(v).range(0.0..=12.0).step(0.5));
                                    let b = ui.add(NumField::new(n).range(0..=100));
                                    rects = (a.rect, b.rect);
                                });
                            });
                    },
                );
                out.textures_delta.clear();
                rects
            };
        // The first frame lays the area out; the second has it in place.
        run(&mut v, &mut n, Vec::new());
        let (a, b) = run(&mut v, &mut n, Vec::new());
        assert!(a.width() > 0.0 && b.left() > a.right());
        assert!(a.left() >= origin.x && a.top() >= origin.y, "{a:?}");
        // The pointer arrives one frame, the wheel turns the next (a
        // widget is hovered from the frame after the pointer reaches it).
        let wheel = |v: &mut f64, n: &mut i32, at: egui::Pos2, up: bool, mods: egui::Modifiers| {
            run(v, n, vec![egui::Event::PointerMoved(at)]);
            run(
                v,
                n,
                vec![egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, if up { 1.0 } else { -1.0 }),
                    phase: egui::TouchPhase::Move,
                    modifiers: mods,
                }],
            );
        };
        // The wheel over the first field: half a unit up, ten halves
        // with Shift, within the range; Ctrl a twentieth.
        wheel(&mut v, &mut n, a.center(), true, egui::Modifiers::NONE);
        assert_eq!(v, 10.5);
        wheel(&mut v, &mut n, a.center(), true, egui::Modifiers::SHIFT);
        assert_eq!(v, 12.0, "clamped to the range");
        wheel(&mut v, &mut n, a.center(), false, egui::Modifiers::COMMAND);
        assert!((v - 11.95).abs() < 1e-9, "{v}");
        // The arrows at the right of the second field (past its box): up
        // then down.
        let spin_x = b.right() + SPIN_W / 2.0;
        let click = |v: &mut f64, n: &mut i32, at: egui::Pos2| {
            run(v, n, vec![egui::Event::PointerMoved(at)]);
            for pressed in [true, false] {
                run(
                    v,
                    n,
                    vec![egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
            }
            run(v, n, Vec::new());
        };
        click(
            &mut v,
            &mut n,
            egui::pos2(spin_x, b.top() + b.height() * 0.25),
        );
        assert_eq!(n, 4);
        click(
            &mut v,
            &mut n,
            egui::pos2(spin_x, b.top() + b.height() * 0.75),
        );
        click(
            &mut v,
            &mut n,
            egui::pos2(spin_x, b.top() + b.height() * 0.75),
        );
        assert_eq!(n, 2);
        // The wheel on the whole-number field with Ctrl: still whole.
        wheel(&mut v, &mut n, b.center(), true, egui::Modifiers::COMMAND);
        assert_eq!(n, 2);
        wheel(&mut v, &mut n, b.center(), true, egui::Modifiers::NONE);
        assert_eq!(n, 3);
    }

    #[test]
    fn typed_values_read_units_decimal_commas_and_sums() {
        assert_eq!(parse("12.5 mm", Some(1.0)), Some(12.5));
        assert_eq!(parse("783,16 px", None), Some(783.16));
        assert_eq!(parse("10+5", None), Some(15.0));
        assert_eq!(parse("2*3,5", None), Some(7.0));
        assert_eq!(parse("(1+2)*4 - 6/3", None), Some(10.0));
        assert_eq!(parse("-3", None), Some(-3.0));
        assert_eq!(parse("45 °", None), Some(45.0));
        assert_eq!(parse("150 %", None), Some(150.0));
        assert_eq!(parse("1e2", None), Some(100.0));
        // A unit converts into the field's.
        assert_eq!(parse("1 in", Some(1.0)), Some(25.4));
        assert_eq!(parse("2\"", Some(1.0)), Some(50.8));
        assert_eq!(parse("1 cm", Some(1.0)), Some(10.0));
        assert_eq!(parse("25.4 mm", Some(25.4)), Some(1.0));
        // Nonsense keeps the old value.
        assert_eq!(parse("abc", None), None);
        assert_eq!(parse("", None), None);
        assert_eq!(parse("1/0", None), None);
        assert_eq!(parse("1+", None), None);
    }

    #[test]
    fn numbers_show_their_decimals() {
        assert_eq!(format_number(12.5, false, 0, None), "12.5");
        assert_eq!(format_number(12.0, false, 0, None), "12");
        assert_eq!(format_number(12.0, false, 3, Some(3)), "12.000");
        assert_eq!(format_number(0.123456, false, 1, Some(4)), "0.1235");
        assert_eq!(format_number(-0.0001, false, 0, Some(2)), "0");
        assert_eq!(format_number(7.6, true, 0, None), "8");
    }

    /// A field alone in a grid cell, with a wider row below setting the
    /// column, as Print Preview's gap between pages: its arrows sit at
    /// the right of its box, in the same cell, and step the value (#948).
    #[test]
    fn a_field_in_a_grid_keeps_its_arrows_at_its_box() {
        let ctx = egui::Context::default();
        let mut v = 5.0f64;
        let run = |v: &mut f64, events: Vec<egui::Event>| -> (egui::Rect, egui::Rect) {
            let mut rects = (egui::Rect::NOTHING, egui::Rect::NOTHING);
            let mut out = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 300.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    egui::Grid::new("gap_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Gap between pages");
                        let field = ui.add(NumField::new(v).range(0.0..=50.0).suffix(" mm"));
                        ui.end_row();
                        ui.label("Margins");
                        let (wide, _) =
                            ui.allocate_exact_size(egui::vec2(320.0, 20.0), egui::Sense::hover());
                        ui.end_row();
                        rects = (field.rect, wide);
                    });
                },
            );
            out.textures_delta.clear();
            rects
        };
        // The grid sizes itself on the first frames.
        for _ in 0..3 {
            run(&mut v, Vec::new());
        }
        let (field, wide) = run(&mut v, Vec::new());
        assert!(field.right() + SPIN_W < wide.right(), "{field:?} {wide:?}");
        // A click just right of the box, on the up arrow, steps the value.
        let at = egui::pos2(
            field.right() + SPIN_W / 2.0,
            field.top() + field.height() * 0.25,
        );
        run(&mut v, vec![egui::Event::PointerMoved(at)]);
        for pressed in [true, false] {
            run(
                &mut v,
                vec![egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
        run(&mut v, Vec::new());
        assert_eq!(v, 6.0, "the arrows are at the box");
        // The next widget in the row comes after the arrows, not over them.
        let mut rects = (egui::Rect::NOTHING, egui::Rect::NOTHING);
        let mut w = 1.0f64;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.horizontal(|ui| {
                let a = ui.add(NumField::new(&mut w));
                let b = ui.label("x");
                rects = (a.rect, b.rect);
            });
        });
        out.textures_delta.clear();
        assert!(
            rects.1.left() >= rects.0.right() + SPIN_W - 0.5,
            "{rects:?}"
        );
    }

    #[test]
    fn a_mixed_field_is_empty() {
        let mut v = 2.5f64;
        assert_eq!(NumField::new(&mut v).text(), "2.5");
        assert_eq!(NumField::new(&mut v).mixed(true).text(), "");
    }

    #[test]
    fn a_field_keeps_its_width_in_a_row_and_fills_a_sized_slot() {
        let ctx = egui::Context::default();
        let mut v = 5.0f64;
        let mut w = 5.0f64;
        let (mut row_width, mut slot_width) = (0.0, 0.0);
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 200.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            ui.horizontal(|ui| {
                row_width = ui.add(NumField::new(&mut v)).rect.width();
                slot_width = ui
                    .add_sized([120.0, 20.0], NumField::new(&mut w))
                    .rect
                    .width();
            });
        });
        out.textures_delta.clear();
        assert!(row_width < 100.0, "{row_width}");
        // The box and its arrows fill the slot together.
        assert!((slot_width + SPIN_W - 120.0).abs() < 1.0, "{slot_width}");
    }

    /// The symmetry bar's X and Y: fields with a prefix stacked in a
    /// column keep the prefix, the box and the arrows on one line, inside
    /// the slot they are given.
    #[test]
    fn prefixed_fields_in_a_column_stay_on_one_line() {
        let ctx = egui::Context::default();
        let (mut x, mut y) = (1.0f64, 2.0f64);
        let mut rects = Vec::new();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 200.0),
            )),
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| {
            ui.vertical(|ui| {
                let left = ui.cursor().left();
                for (label, v) in [("X:", &mut x), ("Y:", &mut y)] {
                    let r = ui.add_sized([120.0, 20.0], NumField::new(v).prefix(label));
                    rects.push((left, r.rect));
                }
            });
        });
        out.textures_delta.clear();
        let (left, a) = rects[0];
        let (_, b) = rects[1];
        assert!(
            a.left() > left + 8.0,
            "the prefix sits before the box: {a:?}"
        );
        assert!(a.right() + SPIN_W <= left + 121.0, "inside the slot: {a:?}");
        assert!(b.top() - a.top() < 30.0, "one line each: {a:?} {b:?}");
    }

    #[test]
    fn a_field_applies_typed_values_on_enter() {
        let ctx = egui::Context::default();
        let mut v = 5.0f64;
        let enter = || egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        // One frame; `focus` asks for the keyboard. True when applied.
        let frame = |events: Vec<egui::Event>, focus: bool, v: &mut f64| -> bool {
            let mut changed = false;
            let input = egui::RawInput {
                events,
                ..Default::default()
            };
            let mut out = ctx.run_ui(input, |ui| {
                let r = ui.add(NumField::new(v).range(0.0..=100.0).suffix(" mm"));
                changed = r.changed();
                if focus {
                    r.request_focus();
                }
            });
            out.textures_delta.clear();
            changed
        };
        frame(Vec::new(), true, &mut v);
        frame(Vec::new(), true, &mut v);
        // Typing alone changes nothing yet.
        assert!(!frame(vec![egui::Event::Text("250".into())], false, &mut v));
        assert_eq!(v, 5.0);
        let mut applied = frame(vec![enter()], false, &mut v);
        applied |= frame(Vec::new(), false, &mut v);
        assert!(applied);
        // Clamped to the range.
        assert_eq!(v, 100.0);
        // Esc puts the value back.
        frame(Vec::new(), true, &mut v);
        frame(Vec::new(), true, &mut v);
        frame(vec![egui::Event::Text("7".into())], false, &mut v);
        let esc = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        let mut applied = frame(vec![esc], false, &mut v);
        applied |= frame(Vec::new(), false, &mut v);
        assert!(!applied);
        assert_eq!(v, 100.0);
    }
}
