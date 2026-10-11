//! The Shape tool on text: a node at the
//! lower left of every character (click, Ctrl or Shift+click and marquee
//! choose them; dragging moves the chosen characters, which sets their
//! horizontal and vertical offsets; the property bar edits their offsets
//! and angle), the Interactive horizontal spacing arrow at the lower right
//! (character spacing; with Shift, word spacing) and the Interactive
//! vertical spacing arrow at the lower left (line spacing). Text > Straighten
//! Text and Align to Baseline put shifted characters back.

use crate::app::App;
use tracedraw_core::{
    document::{spans_apply, spans_char_count, spans_text, ShapeKind, TextSpan},
    geometry::{Affine, Point, Vec2},
    Command, Shape, ShapeId,
};

/// Millimetres per point.
pub const PT_MM: f64 = 25.4 / 72.0;

/// The span holding character `c` of the concatenated text.
fn span_at(spans: &[TextSpan], c: usize) -> Option<&TextSpan> {
    let mut at = 0;
    for s in spans {
        let n = s.char_count();
        if c < at + n {
            return Some(s);
        }
        at += n;
    }
    None
}

/// Text the Shape tool edits: not fitted to a path.
fn editable(s: &Shape) -> bool {
    matches!(&s.kind, ShapeKind::Text { on_path: None, .. })
}

/// The character nodes of a text object: (character index, page point) at
/// each character's origin on its baseline, moved by its offsets.
/// Newlines have none, nor has text fitted to a path.
pub fn char_nodes(s: &Shape) -> Vec<(usize, Point)> {
    let ShapeKind::Text { spans, .. } = &s.kind else {
        return Vec::new();
    };
    if !editable(s) {
        return Vec::new();
    }
    let Some((layout, to_page)) = crate::text_editing::layout_of_text_shape(s, Affine::IDENTITY)
    else {
        return Vec::new();
    };
    let chars: Vec<char> = spans_text(spans).chars().collect();
    let mut out = Vec::new();
    for line in &layout.lines {
        for c in line.start_char..line.end_char {
            if matches!(chars.get(c), Some('\n') | None) {
                continue;
            }
            let Some(x) = line.edges.get(c - line.start_char) else {
                continue;
            };
            let (dx, dy) = span_at(spans, c)
                .map(|sp| {
                    (
                        sp.offset_x_pct / 100.0 * sp.size_pt * PT_MM,
                        sp.baseline_shift_pt * PT_MM,
                    )
                })
                .unwrap_or((0.0, 0.0));
            out.push((c, to_page * Point::new(x + dx, line.baseline + dy)));
        }
    }
    out
}

/// The linear part of the layout-to-page transform: page moves become
/// layout moves through its inverse.
fn to_layout(s: &Shape, d: Vec2) -> Vec2 {
    let [a, b, c, dd, _, _] = s.transform.as_coeffs();
    let lin = Affine::new([a, b, c, dd, 0.0, 0.0]);
    if lin.determinant().abs() < 1e-12 {
        return d;
    }
    (lin.inverse() * Point::new(d.x, d.y)).to_vec2()
}

/// Runs of consecutive character indices.
fn runs(chars: &[usize]) -> Vec<(usize, usize)> {
    let mut v: Vec<usize> = chars.to_vec();
    v.sort_unstable();
    v.dedup();
    let mut out: Vec<(usize, usize)> = Vec::new();
    for c in v {
        match out.last_mut() {
            Some((_, end)) if *end == c => *end = c + 1,
            _ => out.push((c, c + 1)),
        }
    }
    out
}

/// `spans` with `f` applied to the characters `chars` (every character
/// when empty).
pub fn apply_to_chars(
    spans: &[TextSpan],
    chars: &[usize],
    f: impl Fn(&mut TextSpan),
) -> Vec<TextSpan> {
    let mut out = spans.to_vec();
    if chars.is_empty() {
        let n = spans_char_count(&out);
        spans_apply(&mut out, 0, n, f);
    } else {
        for (a, b) in runs(chars) {
            spans_apply(&mut out, a, b, &f);
        }
    }
    out
}

/// The text kind of `s` with other spans.
fn with_spans(s: &Shape, spans: Vec<TextSpan>) -> Option<ShapeKind> {
    match &s.kind {
        ShapeKind::Text {
            origin,
            frame,
            align,
            para,
            on_path,
            ..
        } => Some(ShapeKind::Text {
            spans,
            origin: *origin,
            frame: *frame,
            align: *align,
            para: para.clone(),
            on_path: on_path.clone(),
        }),
        _ => None,
    }
}

/// The widest line's character and space counts, the line pitch and the
/// width of a space (mm), for the spacing arrows.
fn spacing_measures(s: &Shape) -> Option<(usize, usize, f64, f64)> {
    let ShapeKind::Text { spans, .. } = &s.kind else {
        return None;
    };
    let (layout, _) = crate::text_editing::layout_of_text_shape(s, Affine::IDENTITY)?;
    let chars: Vec<char> = spans_text(spans).chars().collect();
    let widest = layout.lines.iter().max_by(|a, b| {
        let w = |l: &tracedraw_text::LineBox| {
            l.edges.last().copied().unwrap_or(0.0) - l.edges.first().copied().unwrap_or(0.0)
        };
        w(a).total_cmp(&w(b))
    })?;
    let line_chars: Vec<char> = chars
        .get(widest.start_char..widest.end_char.min(chars.len()))
        .map(|c| c.to_vec())
        .unwrap_or_default();
    let n = line_chars.len();
    let spaces = line_chars.iter().filter(|c| **c == ' ').count();
    let pitch = if layout.lines.len() > 1 {
        let first = layout.lines[0].baseline;
        let last = layout.lines[layout.lines.len() - 1].baseline;
        (first - last).abs() / (layout.lines.len() - 1) as f64
    } else {
        0.0
    };
    let first = spans.first()?;
    let space = tracedraw_text::fonts()
        .measure(first, " ")
        .max(first.size_pt * PT_MM * 0.25);
    Some((n, spaces, pitch, space))
}

/// The paragraph of `s` after a spacing arrow moved by the layout-space
/// drag `d`: horizontal moves the right end of the widest line by `d.x`
/// (character spacing, or word spacing with `word`), vertical moves the
/// last baseline by `d.y` (line spacing; down spreads the lines).
pub fn spacing_after_drag(s: &Shape, horizontal: bool, word: bool, d: Vec2) -> Option<ShapeKind> {
    let ShapeKind::Text {
        spans,
        origin,
        frame,
        align,
        para,
        on_path,
    } = &s.kind
    else {
        return None;
    };
    let (n, spaces, pitch, space) = spacing_measures(s)?;
    let mut p = para.clone();
    if horizontal {
        if word {
            if spaces == 0 || space <= 0.0 {
                return None;
            }
            let per = d.x / spaces as f64;
            p.word_spacing_pct = (p.word_spacing_pct + per / space * 100.0).clamp(0.0, 2000.0);
        } else {
            if n == 0 || space <= 0.0 {
                return None;
            }
            let per = d.x / n as f64;
            p.char_spacing_pct = (p.char_spacing_pct + per / space * 100.0).clamp(-100.0, 2000.0);
        }
    } else {
        let lines = crate::text_editing::layout_of_text_shape(s, Affine::IDENTITY)?
            .0
            .lines
            .len();
        if lines < 2 || pitch <= 1e-9 {
            return None;
        }
        let new_pitch = (pitch - d.y / (lines - 1) as f64).max(0.0);
        p.leading_pct = (p.leading_pct * new_pitch / pitch).clamp(0.0, 2000.0);
    }
    Some(ShapeKind::Text {
        spans: spans.clone(),
        origin: *origin,
        frame: *frame,
        align: *align,
        para: p,
        on_path: on_path.clone(),
    })
}

impl App {
    /// Selected text objects the Shape tool edits.
    pub fn node_texts(&self) -> Vec<Shape> {
        self.selected_shapes()
            .into_iter()
            .filter(editable)
            .collect()
    }

    /// The chosen characters of `id` (Shape tool).
    pub fn chosen_chars(&self, id: ShapeId) -> Vec<usize> {
        self.node_selection
            .iter()
            .filter(|(s, _)| *s == id)
            .map(|(_, c)| *c)
            .collect()
    }

    /// The character node under the pointer (5 px).
    pub fn text_node_at(&self, p: Point) -> Option<(ShapeId, usize)> {
        let tol = 5.0 / self.view.zoom.max(1e-6) as f64;
        let mut best: Option<(ShapeId, usize, f64)> = None;
        for s in self.node_texts() {
            for (c, q) in char_nodes(&s) {
                let d = (q - p).hypot();
                if d <= tol && best.is_none_or(|b| d < b.2) {
                    best = Some((s.id, c, d));
                }
            }
        }
        best.map(|(id, c, _)| (id, c))
    }

    /// Where the spacing arrows of a text object are drawn, screen space:
    /// (horizontal, at the lower right; vertical, at the lower left).
    pub fn spacing_arrow_spots(&self, s: &Shape) -> (egui::Pos2, egui::Pos2) {
        let b = s.bounds();
        let r = self.view.to_screen(Point::new(b.x1, b.y0));
        let l = self.view.to_screen(Point::new(b.x0, b.y0));
        (r + egui::vec2(12.0, 12.0), l + egui::vec2(-12.0, 12.0))
    }

    /// The spacing arrow under the pointer: the text and whether it is the
    /// horizontal one.
    pub fn text_arrow_at(&self, p: Point) -> Option<(ShapeId, bool)> {
        let at = self.view.to_screen(p);
        for s in self.node_texts() {
            let (h, v) = self.spacing_arrow_spots(&s);
            if h.distance(at) <= 8.0 {
                return Some((s.id, true));
            }
            if v.distance(at) <= 8.0 {
                return Some((s.id, false));
            }
        }
        None
    }

    /// Move the characters `chars` of the text as it was (`start`) by the
    /// page vector `d`: their offsets change, the other characters stay.
    pub fn move_text_chars(&mut self, start: &Shape, chars: &[usize], d: Vec2, begun: bool) {
        let ShapeKind::Text { spans, .. } = &start.kind else {
            return;
        };
        if chars.is_empty() {
            return;
        }
        let l = to_layout(start, d);
        let spans = apply_to_chars(spans, chars, |sp| {
            let size = (sp.size_pt * PT_MM).max(1e-9);
            sp.offset_x_pct += l.x / size * 100.0;
            sp.baseline_shift_pt += l.y / PT_MM;
        });
        let Some(kind) = with_spans(start, spans) else {
            return;
        };
        self.record(start.id, kind, "Move Characters", begun);
    }

    /// A spacing arrow dragged by the page vector `d` from where the text
    /// was (`start`).
    pub fn drag_text_spacing(
        &mut self,
        start: &Shape,
        horizontal: bool,
        word: bool,
        d: Vec2,
        begun: bool,
    ) {
        let l = to_layout(start, d);
        let Some(kind) = spacing_after_drag(start, horizontal, word, l) else {
            return;
        };
        let label = if !horizontal {
            "Line Spacing"
        } else if word {
            "Word Spacing"
        } else {
            "Character Spacing"
        };
        self.record(start.id, kind, label, begun);
    }

    /// One drag step: the first is recorded, later ones join it.
    fn record(&mut self, id: ShapeId, kind: ShapeKind, label: &'static str, begun: bool) {
        let cmd = Command::SetShapeKind { shape: id, kind };
        let r = if begun {
            self.engine.amend(&cmd)
        } else {
            self.engine.run_with_label(&cmd, label)
        };
        if let Err(e) = r {
            self.status = e.to_string();
        }
    }

    /// The characters a character attribute change applies to: the text
    /// being edited (its selection, or all of it), else the chosen
    /// character nodes of each selected text (all characters of a text
    /// with none chosen).
    fn char_targets(&self) -> Vec<(Shape, Vec<usize>)> {
        if let Some(te) = &self.text_edit {
            if let Some(s) = self.doc().find_shape(te.shape) {
                let chars = if te.has_selection() {
                    let (a, b) = te.selection();
                    (a..b).collect()
                } else {
                    Vec::new()
                };
                return vec![(s.clone(), chars)];
            }
        }
        self.selected_shapes()
            .into_iter()
            .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
            .map(|s| {
                let chars = self.chosen_chars(s.id);
                (s, chars)
            })
            .collect()
    }

    /// The offsets and angle the fields show: those of the first target
    /// character (horizontal %, vertical % of the size, degrees).
    pub fn char_attrs(&self) -> Option<(f64, f64, f64)> {
        let (s, chars) = self.char_targets().into_iter().next()?;
        let ShapeKind::Text { spans, .. } = &s.kind else {
            return None;
        };
        let sp = span_at(spans, chars.first().copied().unwrap_or(0)).or(spans.first())?;
        let v = if sp.size_pt > 0.0 {
            sp.baseline_shift_pt / sp.size_pt * 100.0
        } else {
            0.0
        };
        Some((sp.offset_x_pct, v, sp.angle_deg))
    }

    /// Change the character attributes of the targets in one undo step.
    pub fn set_char_attrs(&mut self, label: &'static str, f: impl Fn(&mut TextSpan)) {
        let mut cmds = Vec::new();
        for (s, chars) in self.char_targets() {
            let ShapeKind::Text { spans, .. } = &s.kind else {
                continue;
            };
            let spans = apply_to_chars(spans, &chars, &f);
            if let Some(kind) = with_spans(&s, spans) {
                cmds.push(Command::SetShapeKind { shape: s.id, kind });
            }
        }
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch(label, &cmds) {
            self.status = e.to_string();
        }
    }

    /// Text > Straighten Text: the target characters lose their offsets
    /// and angle; text fitted to a path is taken off it.
    pub fn straighten_chars(&mut self) {
        self.set_char_attrs("Straighten Text", |sp| {
            sp.offset_x_pct = 0.0;
            sp.baseline_shift_pt = 0.0;
            sp.angle_deg = 0.0;
        });
        if self.text_edit.is_none()
            && self.selected_shapes().iter().any(|s| {
                matches!(
                    s.kind,
                    ShapeKind::Text {
                        on_path: Some(_),
                        ..
                    }
                )
            })
        {
            self.straighten_text();
        }
    }

    /// Text > Align to Baseline: the target characters go back to the
    /// baseline (their vertical offset only).
    pub fn align_chars_to_baseline(&mut self) {
        self.set_char_attrs("Align to Baseline", |sp| sp.baseline_shift_pt = 0.0);
    }

    /// Choose the character nodes of the selected texts inside `r` (or
    /// `poly`); false when there were no texts to choose in.
    pub fn choose_chars_in(&mut self, inside: impl Fn(Point) -> bool) -> bool {
        let texts = self.node_texts();
        if texts.is_empty() {
            return false;
        }
        let mut sel = Vec::new();
        for s in texts {
            for (c, q) in char_nodes(&s) {
                if inside(q) {
                    sel.push((s.id, c));
                }
            }
        }
        let curves: Vec<(ShapeId, usize)> = self
            .node_selection
            .iter()
            .copied()
            .filter(|(id, _)| {
                self.doc()
                    .find_shape(*id)
                    .is_some_and(|s| !matches!(s.kind, ShapeKind::Text { .. }))
            })
            .collect();
        self.node_selection = curves;
        self.node_selection.extend(sel);
        true
    }
}

/// The box of a text's character nodes.
#[cfg(test)]
pub fn nodes_box(s: &Shape) -> Option<tracedraw_core::geometry::Rect> {
    let pts = char_nodes(s);
    let first = pts.first()?.1;
    Some(pts.iter().fold(
        tracedraw_core::geometry::Rect::from_points(first, first),
        |r, (_, p)| r.union_pt(*p),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::document::{ParagraphStyle, TextAlign};

    fn text_app(text: &str) -> (App, ShapeId) {
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new(text, "DejaVu Sans", 24.0)],
                origin: Point::new(10.0, 10.0),
                frame: None,
                align: TextAlign::Left,
                para: ParagraphStyle::default(),
                on_path: None,
            })
            .expect("a text");
        app.select(vec![id]);
        app.tool = crate::tools::Tool::Shape;
        (app, id)
    }

    fn has_fonts() -> bool {
        !tracedraw_text::fonts().families().is_empty()
    }

    #[test]
    fn every_character_has_a_node_on_its_baseline() {
        let (app, id) = text_app("Ab\ncd");
        let s = app.doc().find_shape(id).cloned().expect("text");
        let nodes = char_nodes(&s);
        // Four characters, no node for the newline.
        assert_eq!(
            nodes.iter().map(|n| n.0).collect::<Vec<_>>(),
            vec![0, 1, 3, 4]
        );
        assert!((nodes[0].1 - Point::new(10.0, 10.0)).hypot() < 1e-9);
        if has_fonts() {
            assert!(nodes[1].1.x > nodes[0].1.x);
            assert!(nodes[2].1.y < nodes[0].1.y);
        }
        assert_eq!(app.text_node_at(Point::new(10.0, 10.0)), Some((id, 0)));
    }

    #[test]
    fn dragging_chosen_characters_shifts_only_them() {
        let (mut app, id) = text_app("abc");
        let s = app.doc().find_shape(id).cloned().expect("text");
        let before = char_nodes(&s);
        app.node_selection = vec![(id, 1)];
        // 24 pt is 8.4667 mm: a move of 4.2333 mm right is 50 %.
        let d = Vec2::new(24.0 * PT_MM / 2.0, 2.0 * PT_MM);
        let depth = app.engine.history_labels().0.len();
        app.move_text_chars(&s, &[1], d, false);
        app.move_text_chars(&s, &[1], d * 2.0, true);
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let now = app.doc().find_shape(id).cloned().expect("text");
        let ShapeKind::Text { spans, .. } = &now.kind else {
            panic!();
        };
        assert_eq!(spans.len(), 3, "{spans:?}");
        assert!((spans[1].offset_x_pct - 100.0).abs() < 1e-9);
        assert!((spans[1].baseline_shift_pt - 4.0).abs() < 1e-9);
        let after = char_nodes(&now);
        assert_eq!(after[0], before[0]);
        assert_eq!(after[2], before[2]);
        assert!((after[1].1 - (before[1].1 + d * 2.0)).hypot() < 1e-9);
        // The fields show it; Align to Baseline drops the vertical part,
        // Straighten Text the rest, and the spans merge again.
        let (h, v, a) = app.char_attrs().expect("attributes");
        assert!((h - 100.0).abs() < 1e-9 && (v - 4.0 / 24.0 * 100.0).abs() < 1e-9 && a == 0.0);
        app.align_chars_to_baseline();
        let (_, v, _) = app.char_attrs().expect("attributes");
        assert_eq!(v, 0.0);
        app.set_char_attrs("Character Angle", |sp| sp.angle_deg = 30.0);
        assert_eq!(app.char_attrs().map(|a| a.2), Some(30.0));
        app.straighten_chars();
        let ShapeKind::Text { spans, .. } = &app.doc().find_shape(id).expect("text").kind else {
            panic!();
        };
        assert_eq!(spans.len(), 1, "{spans:?}");
    }

    #[test]
    fn a_marquee_chooses_character_nodes() {
        let (mut app, id) = text_app("abc");
        assert!(app.choose_chars_in(|q| q.x < 10.5));
        assert_eq!(app.node_selection, vec![(id, 0)]);
        app.select(vec![]);
        assert!(!app.choose_chars_in(|_| true));
    }

    #[test]
    fn spacing_arrows_change_character_word_and_line_spacing() {
        if !has_fonts() {
            return;
        }
        let (mut app, id) = text_app("ab cd\nef gh");
        let s = app.doc().find_shape(id).cloned().expect("text");
        let (n, spaces, pitch, space) = spacing_measures(&s).expect("measures");
        assert_eq!((n, spaces), (5, 1));
        assert!(pitch > 0.0 && space > 0.0);
        // The right end follows the arrow.
        let w0 = nodes_box(&s).expect("box").width();
        app.drag_text_spacing(&s, true, false, Vec2::new(10.0, 0.0), false);
        let now = app.doc().find_shape(id).cloned().expect("text");
        let ShapeKind::Text { para, .. } = &now.kind else {
            panic!();
        };
        assert!((para.char_spacing_pct - 2.0 / space * 100.0).abs() < 1e-6);
        assert!(nodes_box(&now).expect("box").width() > w0);
        // Shift: word spacing; vertical: line spacing, one step each.
        app.drag_text_spacing(&now, true, true, Vec2::new(3.0, 0.0), false);
        let ShapeKind::Text { para, .. } = &app.doc().find_shape(id).expect("text").kind else {
            panic!();
        };
        assert!(para.word_spacing_pct > 100.0);
        let now = app.doc().find_shape(id).cloned().expect("text");
        app.drag_text_spacing(&now, false, false, Vec2::new(0.0, -pitch), false);
        let ShapeKind::Text { para, .. } = &app.doc().find_shape(id).expect("text").kind else {
            panic!();
        };
        assert!(
            (para.leading_pct - 200.0).abs() < 1e-6,
            "{}",
            para.leading_pct
        );
        assert_eq!(app.text_arrow_at(Point::new(-1000.0, -1000.0)), None);
    }
}
