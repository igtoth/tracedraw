//! Number fields as the target design's bars have them: a white box
//! with the value at the left, typed into. Enter or leaving the box
//! applies the value, Esc puts it back. Sums such as `10+5` or `2*3,5`
//! work, a comma or a point marks decimals, and a distance field takes a
//! value in another unit (`1 in` in a millimetre field).

use crate::app::Units;
use egui::{emath::Numeric, Response, TextEdit, Ui, Widget};
use std::ops::RangeInclusive;

/// A number field; built like `egui::DragValue`.
pub struct NumField<'a, N: Numeric> {
    value: &'a mut N,
    range: Option<(f64, f64)>,
    suffix: String,
    min_decimals: usize,
    max_decimals: Option<usize>,
    /// Millimetres per unit of the field, for distances typed with a unit.
    unit_mm: Option<f64>,
}

impl<'a, N: Numeric> NumField<'a, N> {
    pub fn new(value: &'a mut N) -> Self {
        NumField {
            value,
            range: None,
            suffix: String::new(),
            min_decimals: 0,
            max_decimals: None,
            unit_mm: None,
        }
    }

    pub fn range<R: Numeric>(mut self, range: RangeInclusive<R>) -> Self {
        self.range = Some((range.start().to_f64(), range.end().to_f64()));
        self
    }

    /// Kept for the builder's shape: the field is typed into, not dragged.
    pub fn speed(self, _speed: impl Into<f64>) -> Self {
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
        let id = ui.next_auto_id().with("num_field");
        let shown = self.text();
        // While typing, the text lives in memory until it is applied.
        let stored = ui.data(|d| d.get_temp::<String>(id));
        let mut buf = stored.clone().unwrap_or_else(|| shown.clone());
        let mut resp = ui.add(
            TextEdit::singleline(&mut buf)
                .id(id)
                .margin(egui::Margin::symmetric(4, 0))
                .vertical_align(egui::Align::Center)
                .desired_width(f32::INFINITY),
        );
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
                    if v != self.value.to_f64() {
                        *self.value = N::from_f64(v);
                        resp.mark_changed();
                    }
                }
            }
        }
        resp
    }
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

    #[test]
    fn typed_values_read_like_the_reference_editor() {
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
