//! In-place text editing: caret, selection, typing, deleting and
//! character formatting of the text object being edited with the Text
//! tool. Positions are character indices of the concatenated span text
//! (`tracedraw_core::spans_text`); geometry comes from the text layout's
//! line boxes.

use crate::app::App;
use tracedraw_core::document::text_outline::TextRequest;
use tracedraw_core::geometry::{Affine, Point, Rect};
use tracedraw_core::{
    spans_apply, spans_char_count, spans_delete, spans_insert, spans_text, Color, Command, Fill,
    ShapeId, ShapeKind, Stroke, TextSpan,
};
use tracedraw_text::TextLayout;

/// The text object being edited and where the caret is.
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub shape: ShapeId,
    /// Caret position: a character index of the concatenated spans.
    pub caret: usize,
    /// The other end of the selection; equals `caret` when nothing is
    /// selected.
    pub anchor: usize,
    /// Horizontal position remembered across Up/Down moves (layout space).
    pub goal_x: Option<f64>,
}

impl TextEdit {
    pub fn at_end(shape: ShapeId, len: usize) -> Self {
        TextEdit {
            shape,
            caret: len,
            anchor: len,
            goal_x: None,
        }
    }

    /// The selected range, ordered.
    pub fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    pub fn has_selection(&self) -> bool {
        self.caret != self.anchor
    }
}

/// Caret and selection move requests from the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretMove {
    Left,
    Right,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    Up,
    Down,
    TextStart,
    TextEnd,
}

impl App {
    /// The spans of the text being edited.
    pub fn edit_spans(&self) -> Option<Vec<TextSpan>> {
        let te = self.text_edit.as_ref()?;
        match self.doc().find_shape(te.shape).map(|s| &s.kind) {
            Some(ShapeKind::Text { spans, .. }) => Some(spans.clone()),
            _ => None,
        }
    }

    /// The text being edited, concatenated.
    pub fn edit_text(&self) -> String {
        self.edit_spans()
            .map(|s| spans_text(&s))
            .unwrap_or_default()
    }

    /// Layout of a text object and the transform from layout space to the
    /// page (the shape's transform after the origin or frame shift).
    pub fn text_layout_of(&self, id: ShapeId) -> Option<(TextLayout, Affine)> {
        let s = self.doc().find_shape(id)?;
        layout_of_text_shape(s, Affine::IDENTITY)
    }

    /// Replace the spans of the text being edited. Keystrokes collapse
    /// into one undo step per text object.
    pub fn set_edit_spans(&mut self, spans: Vec<TextSpan>) {
        self.set_edit_spans_as(spans, "Edit Text", true);
    }

    /// Replace the spans of the text being edited as an undo step named
    /// `label`; with `merge` it takes the place of a step of that name
    /// just before it.
    fn set_edit_spans_as(&mut self, spans: Vec<TextSpan>, label: &'static str, merge: bool) {
        let Some(te) = self.text_edit.clone() else {
            return;
        };
        let Some(s) = self.doc().find_shape(te.shape).cloned() else {
            return;
        };
        let ShapeKind::Text {
            origin,
            frame,
            align,
            para,
            on_path,
            ..
        } = s.kind
        else {
            return;
        };
        let mut para = para;
        para.hyphenate = self.text_hyphenation;
        let kind = ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para,
            on_path,
        };
        if merge && self.engine.undo_label() == Some(label) {
            let _ = self.engine.undo();
        }
        let _ = self.engine.run_with_label(
            &Command::SetShapeKind {
                shape: te.shape,
                kind,
            },
            label,
        );
        if self.is_linked_frame(te.shape) {
            self.reflow_chain(te.shape);
        }
        self.sync_text_defaults_from(te.shape);
    }

    /// Place the caret at a character index (a click); Shift extends.
    pub fn text_set_caret_at(&mut self, idx: usize, extend: bool) {
        self.set_caret(idx, extend);
        if let Some(te) = self.text_edit.as_mut() {
            te.goal_x = None;
        }
        if let Some(id) = self.text_edit.as_ref().map(|t| t.shape) {
            self.sync_text_defaults_from(id);
        }
    }

    /// Keyboard input while a text is being edited.
    pub fn text_keyboard(&mut self, ctx: &egui::Context) {
        use egui::{Event, Key};
        let events = ctx.input(|i| i.events.clone());
        // Right Ctrl + Right Shift or Left Ctrl + Left Shift, let go with
        // no other key: the paragraph direction.
        let (ctrl_shift, other_key) = ctx.input(|i| {
            (
                i.modifiers.ctrl && i.modifiers.shift,
                i.events
                    .iter()
                    .any(|e| matches!(e, Event::Key { pressed: true, .. } | Event::Text(_))),
            )
        });
        let side = if ctrl_shift {
            crate::direction_keys::held_side()
        } else {
            None
        };
        if let Some(d) = self.direction_gesture.step(ctrl_shift, side, other_key) {
            self.apply_text_direction(d);
        }
        for ev in &events {
            // An input method composing text owns the keys until it
            // commits (Enter, Backspace and the arrows edit the
            // composition, not the text).
            if !self.ime_preedit.is_empty() && matches!(ev, Event::Key { .. }) {
                continue;
            }
            match ev {
                Event::Ime(ime) => {
                    if let Some(commit) = self.ime_event(ime) {
                        self.text_insert(&commit);
                    }
                }
                Event::Text(t) => self.text_insert(t),
                Event::Paste(t) => self.text_insert(t),
                Event::Copy => {
                    let sel = self.text_selected();
                    if !sel.is_empty() {
                        ctx.copy_text(sel);
                    }
                }
                Event::Cut => {
                    let sel = self.text_selected();
                    if !sel.is_empty() {
                        ctx.copy_text(sel);
                        self.text_delete_backward();
                    }
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    let shift = modifiers.shift;
                    let ctrl = modifiers.command;
                    match key {
                        Key::Num8 if ctrl => self.step_text_size(true),
                        Key::Num2 if ctrl => self.step_text_size(false),
                        Key::Backspace => self.text_delete_backward(),
                        Key::Delete => self.text_delete_forward(),
                        Key::Enter if !shift => self.text_insert("\n"),
                        Key::Enter => self.text_insert("\u{2028}"),
                        Key::Tab => self.text_insert("\t"),
                        Key::ArrowLeft if ctrl => self.text_move(CaretMove::WordLeft, shift),
                        Key::ArrowRight if ctrl => self.text_move(CaretMove::WordRight, shift),
                        Key::ArrowLeft => self.text_move(CaretMove::Left, shift),
                        Key::ArrowRight => self.text_move(CaretMove::Right, shift),
                        Key::ArrowUp => self.text_move(CaretMove::Up, shift),
                        Key::ArrowDown => self.text_move(CaretMove::Down, shift),
                        Key::Home if ctrl => self.text_move(CaretMove::TextStart, shift),
                        Key::End if ctrl => self.text_move(CaretMove::TextEnd, shift),
                        Key::Home => self.text_move(CaretMove::LineStart, shift),
                        Key::End => self.text_move(CaretMove::LineEnd, shift),
                        Key::A if ctrl => self.text_select_all(),
                        // Alignment of the paragraphs at the caret (the
                        // same keys export, combine and repeat when no
                        // text is being typed).
                        Key::L if ctrl && !shift => {
                            self.apply_text_align(tracedraw_core::TextAlign::Left)
                        }
                        Key::E if ctrl && !shift => {
                            self.apply_text_align(tracedraw_core::TextAlign::Center)
                        }
                        Key::R if ctrl && !shift => {
                            self.apply_text_align(tracedraw_core::TextAlign::Right)
                        }
                        Key::J if ctrl && !shift => {
                            self.apply_text_align(tracedraw_core::TextAlign::Justify)
                        }
                        Key::H if ctrl && !shift => {
                            self.apply_text_align(tracedraw_core::TextAlign::ForceJustify)
                        }
                        Key::N if ctrl && !shift => {
                            self.apply_text_align(tracedraw_core::TextAlign::None)
                        }
                        Key::B if ctrl => {
                            let on = !self.text_bold;
                            self.text_bold = on;
                            self.text_apply_style(move |s| s.bold = on);
                        }
                        Key::I if ctrl => {
                            let on = !self.text_italic;
                            self.text_italic = on;
                            self.text_apply_style(move |s| s.italic = on);
                        }
                        Key::U if ctrl => {
                            let on = !self.text_underline;
                            self.text_underline = on;
                            self.text_apply_style(move |s| s.underline = on);
                        }
                        Key::Escape => {
                            self.finish_text();
                            self.set_tool(crate::tools::Tool::Pick);
                            return;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    /// Follow an input method: its composition is kept to show at the
    /// caret; the committed text is returned to be inserted.
    pub fn ime_event(&mut self, ime: &egui::ImeEvent) -> Option<String> {
        match ime {
            egui::ImeEvent::Preedit { text, .. } => {
                self.ime_preedit = text.clone();
                None
            }
            egui::ImeEvent::Commit(text) => {
                self.ime_preedit.clear();
                (!text.is_empty()).then(|| text.clone())
            }
            _ => None,
        }
    }

    fn set_caret(&mut self, caret: usize, extend: bool) {
        let len = spans_char_count(&self.edit_spans().unwrap_or_default());
        if let Some(te) = self.text_edit.as_mut() {
            te.caret = caret.min(len);
            if !extend {
                te.anchor = te.caret;
            }
        }
    }

    /// Type `s` at the caret, replacing the selection. Autocorrect acts
    /// on the word just finished when the caret is at the end.
    pub fn text_insert(&mut self, s: &str) {
        let Some(te) = self.text_edit.clone() else {
            return;
        };
        let Some(mut spans) = self.edit_spans() else {
            return;
        };
        let (a, b) = te.selection();
        spans_delete(&mut spans, a, b);
        spans_insert(&mut spans, a, s);
        let mut caret = a + s.chars().count();
        if self.settings.autocorrect.enabled {
            let typed: String = spans_text(&spans).chars().take(caret).collect();
            let style = crate::autocorrect::QuoteStyle::for_language(&crate::i18n::language());
            if let Some(fixed) =
                crate::autocorrect::on_typed(&typed, &self.settings.autocorrect, style)
            {
                // Replace from the first differing character.
                let common = typed
                    .chars()
                    .zip(fixed.chars())
                    .take_while(|(x, y)| x == y)
                    .count();
                let tail: String = fixed.chars().skip(common).collect();
                spans_delete(&mut spans, common, caret);
                spans_insert(&mut spans, common, &tail);
                caret = common + tail.chars().count();
            }
        }
        self.set_edit_spans(spans);
        self.set_caret(caret, false);
        if let Some(te) = self.text_edit.as_mut() {
            te.goal_x = None;
        }
    }

    /// Backspace: delete the selection or the character before the caret.
    pub fn text_delete_backward(&mut self) {
        let Some(te) = self.text_edit.clone() else {
            return;
        };
        let Some(mut spans) = self.edit_spans() else {
            return;
        };
        let (a, b) = if te.has_selection() {
            te.selection()
        } else if te.caret > 0 {
            (te.caret - 1, te.caret)
        } else {
            return;
        };
        spans_delete(&mut spans, a, b);
        self.set_edit_spans(spans);
        self.set_caret(a, false);
    }

    /// Delete: delete the selection or the character after the caret.
    pub fn text_delete_forward(&mut self) {
        let Some(te) = self.text_edit.clone() else {
            return;
        };
        let Some(mut spans) = self.edit_spans() else {
            return;
        };
        let len = spans_char_count(&spans);
        let (a, b) = if te.has_selection() {
            te.selection()
        } else if te.caret < len {
            (te.caret, te.caret + 1)
        } else {
            return;
        };
        spans_delete(&mut spans, a, b);
        self.set_edit_spans(spans);
        self.set_caret(a, false);
    }

    /// Move the caret; `extend` keeps the anchor (Shift).
    pub fn text_move(&mut self, m: CaretMove, extend: bool) {
        let Some(te) = self.text_edit.clone() else {
            return;
        };
        let text: Vec<char> = self.edit_text().chars().collect();
        let len = text.len();
        let (a, b) = te.selection();
        let mut goal_x = None;
        let caret = match m {
            CaretMove::Left | CaretMove::Right => {
                let left = m == CaretMove::Left;
                if te.has_selection() && !extend {
                    if left {
                        a
                    } else {
                        b
                    }
                } else {
                    // On a line with right-to-left text the arrows move on
                    // screen; elsewhere one character.
                    let visual = self
                        .text_layout_of(te.shape)
                        .and_then(|(layout, _)| layout.visual_step(te.caret, left));
                    let rtl_line = self
                        .text_layout_of(te.shape)
                        .and_then(|(layout, _)| {
                            layout
                                .line_of(te.caret)
                                .and_then(|i| layout.lines.get(i))
                                .map(|l| l.rtl)
                        })
                        .unwrap_or(false);
                    match visual {
                        Some(c) => c,
                        // Off the end of the line: on to the next line the
                        // way the text reads.
                        None if left != rtl_line => te.caret.saturating_sub(1),
                        None => (te.caret + 1).min(len),
                    }
                }
            }
            CaretMove::WordLeft => word_start_before(&text, te.caret),
            CaretMove::WordRight => word_end_after(&text, te.caret),
            CaretMove::TextStart => 0,
            CaretMove::TextEnd => len,
            CaretMove::LineStart | CaretMove::LineEnd => match self.text_layout_of(te.shape) {
                Some((layout, _)) => {
                    let line = layout.line_of(te.caret).and_then(|i| layout.lines.get(i));
                    match (line, m) {
                        (Some(l), CaretMove::LineStart) => l.start_char,
                        (Some(l), _) => l.end_char,
                        (None, CaretMove::LineStart) => 0,
                        (None, _) => len,
                    }
                }
                None => {
                    if m == CaretMove::LineStart {
                        0
                    } else {
                        len
                    }
                }
            },
            CaretMove::Up | CaretMove::Down => match self.text_layout_of(te.shape) {
                Some((layout, _)) => {
                    let Some(li) = layout.line_of(te.caret) else {
                        return;
                    };
                    let x = te
                        .goal_x
                        .or_else(|| layout.caret(te.caret).map(|c| c.0))
                        .unwrap_or(0.0);
                    goal_x = Some(x);
                    // The line above or below: parts of one line split
                    // around a wrap object share a baseline and count as
                    // one.
                    let here = layout.lines.get(li).map_or(0.0, |l| l.baseline);
                    let target = if m == CaretMove::Up {
                        layout
                            .lines
                            .get(..li)
                            .unwrap_or(&[])
                            .iter()
                            .rposition(|l| l.baseline > here + 1e-6)
                    } else {
                        layout
                            .lines
                            .iter()
                            .enumerate()
                            .skip(li + 1)
                            .find(|(_, l)| l.baseline < here - 1e-6)
                            .map(|(i, _)| i)
                    };
                    match target.and_then(|t| layout.lines.get(t)) {
                        Some(l) => layout
                            .hit_char(Point::new(x, l.baseline))
                            .unwrap_or(l.start_char),
                        None if m == CaretMove::Up => 0,
                        None => len,
                    }
                }
                None => return,
            },
        };
        self.set_caret(caret, extend);
        if let Some(te) = self.text_edit.as_mut() {
            te.goal_x = goal_x;
        }
    }

    pub fn text_select_all(&mut self) {
        let len = spans_char_count(&self.edit_spans().unwrap_or_default());
        if let Some(te) = self.text_edit.as_mut() {
            te.anchor = 0;
            te.caret = len;
        }
    }

    /// Select the word around the character index `idx` (double-click).
    pub fn text_select_word_at(&mut self, idx: usize) {
        let text: Vec<char> = self.edit_text().chars().collect();
        let idx = idx.min(text.len());
        let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '\'';
        let mut a = idx;
        while a > 0 && text.get(a - 1).is_some_and(|c| is_word(*c)) {
            a -= 1;
        }
        let mut b = idx;
        while text.get(b).is_some_and(|c| is_word(*c)) {
            b += 1;
        }
        if a == b {
            // Not on a word: select the single character.
            b = (idx + 1).min(text.len());
        }
        if let Some(te) = self.text_edit.as_mut() {
            te.anchor = a;
            te.caret = b;
        }
    }

    /// Character index under the page point `p` in the text being edited.
    pub fn text_hit_char(&self, p: Point) -> Option<usize> {
        let te = self.text_edit.as_ref()?;
        let (layout, to_page) = self.text_layout_of(te.shape)?;
        let local = to_page.inverse() * p;
        layout.hit_char(local)
    }

    /// Apply a style change to the selected characters, or to the whole
    /// text when nothing is selected.
    pub fn text_apply_style(&mut self, f: impl Fn(&mut TextSpan)) {
        let Some(te) = self.text_edit.clone() else {
            return;
        };
        let Some(mut spans) = self.edit_spans() else {
            return;
        };
        let (a, b) = if te.has_selection() {
            te.selection()
        } else {
            (0, spans_char_count(&spans))
        };
        if a == b {
            for sp in spans.iter_mut() {
                f(sp);
            }
        } else {
            spans_apply(&mut spans, a, b, f);
        }
        self.set_edit_spans(spans);
    }

    /// Characters are selected in the text being edited: a colour picked
    /// (a palette swatch, the Properties fill, the Color docker) goes to
    /// them instead of the whole object.
    pub fn text_selection_active(&self) -> bool {
        self.text_edit.as_ref().is_some_and(|te| te.has_selection()) && self.edit_spans().is_some()
    }

    /// The selected characters get their own fill, as an undo step of its
    /// own ("Character Fill"). False (nothing done) without a selection.
    pub fn text_apply_fill(&mut self, fill: Fill) -> bool {
        self.text_style_selection("Character Fill", move |sp| sp.fill = Some(fill.clone()))
    }

    /// The selected characters get their own outline in `color` (from
    /// their outline, else the object's, else the default pen); `None`
    /// takes the outline off them. One undo step ("Character Outline").
    pub fn text_apply_outline_color(&mut self, color: Option<Color>) -> bool {
        let base = self
            .text_edit
            .as_ref()
            .and_then(|te| self.doc().find_shape(te.shape))
            .and_then(|s| s.stroke.clone());
        self.text_style_selection("Character Outline", move |sp| {
            sp.outline = Some(match color {
                Some(c) => {
                    let mut st = sp
                        .outline
                        .clone()
                        .filter(|o| o.width > 0.0)
                        .or_else(|| base.clone())
                        .unwrap_or_default();
                    st.color = c;
                    st
                }
                // Width 0: no outline on these characters.
                None => Stroke {
                    width: 0.0,
                    ..Stroke::default()
                },
            });
        })
    }

    /// Apply `f` to the selected characters only, as an undo step named
    /// `label`.
    fn text_style_selection(&mut self, label: &'static str, f: impl Fn(&mut TextSpan)) -> bool {
        if !self.text_selection_active() {
            return false;
        }
        let (Some(te), Some(mut spans)) = (self.text_edit.clone(), self.edit_spans()) else {
            return false;
        };
        let (a, b) = te.selection();
        spans_apply(&mut spans, a, b, f);
        self.set_edit_spans_as(spans, label, false);
        true
    }

    /// The span the property bar shows while editing: the one of the
    /// character before the caret (the first span at the start).
    pub fn text_span_at_caret(&self) -> Option<TextSpan> {
        let te = self.text_edit.as_ref()?;
        let spans = self.edit_spans()?;
        let at = te.caret.saturating_sub(1);
        let mut seen = 0usize;
        spans
            .iter()
            .find(|sp| {
                let n = sp.char_count();
                let hit = at < seen + n;
                seen += n;
                hit
            })
            .or_else(|| spans.first())
            .cloned()
    }

    /// The selected text (empty without a selection).
    pub fn text_selected(&self) -> String {
        let Some(te) = self.text_edit.as_ref() else {
            return String::new();
        };
        let (a, b) = te.selection();
        self.edit_text().chars().skip(a).take(b - a).collect()
    }

    /// Screen-independent caret and selection geometry on the page: the
    /// caret segment and one rectangle per selected line.
    pub fn text_caret_geometry(&self) -> Option<(Point, Point, Vec<[Point; 4]>)> {
        let te = self.text_edit.as_ref()?;
        let (layout, to_page) = self.text_layout_of(te.shape)?;
        let size_pt = self
            .edit_spans()?
            .first()
            .map(|s| s.size_pt)
            .unwrap_or(12.0);
        caret_geometry_sized(&layout, &to_page, te.caret, te.selection(), size_pt)
    }
}

/// Layout of a text shape (any `ShapeKind::Text`, placed under `parent`)
/// and the transform from layout space to the page.
pub fn layout_of_text_shape(
    s: &tracedraw_core::Shape,
    parent: Affine,
) -> Option<(TextLayout, Affine)> {
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
    let layout = tracedraw_text::fonts().layout(&TextRequest {
        spans,
        frame: *frame,
        align: *align,
        para,
        on_path: on_path.as_ref(),
    });
    let shift = match frame {
        Some(f) if on_path.is_none() => Affine::translate((origin.x, origin.y + f.height)),
        _ => Affine::translate(origin.to_vec2()),
    };
    Some((layout, parent * s.transform * shift))
}

/// Caret segment (top, bottom) and one quad per selected line, on the page.
pub fn caret_geometry(
    layout: &TextLayout,
    to_page: &Affine,
    caret: usize,
    selection: (usize, usize),
) -> Option<(Point, Point, Vec<[Point; 4]>)> {
    caret_geometry_sized(layout, to_page, caret, selection, 12.0)
}

fn caret_geometry_sized(
    layout: &TextLayout,
    to_page: &Affine,
    caret: usize,
    (a, b): (usize, usize),
    size_pt: f64,
) -> Option<(Point, Point, Vec<[Point; 4]>)> {
    let (x, base, asc, desc) = layout.caret(caret).unwrap_or_else(|| {
        // Empty text: a caret of the text size at the origin.
        let size_mm = size_pt * 25.4 / 72.0;
        (0.0, 0.0, size_mm * 0.8, size_mm * 0.2)
    });
    let top = *to_page * Point::new(x, base + asc);
    let bottom = *to_page * Point::new(x, base - desc);
    let mut quads = Vec::new();
    if a != b {
        let quad = |x0: f64, x1: f64, line: &tracedraw_text::LineBox| {
            let r = Rect::new(
                x0,
                line.baseline - line.descent,
                x1.max(x0 + 0.3),
                line.baseline + line.ascent,
            );
            [
                *to_page * Point::new(r.x0, r.y0),
                *to_page * Point::new(r.x1, r.y0),
                *to_page * Point::new(r.x1, r.y1),
                *to_page * Point::new(r.x0, r.y1),
            ]
        };
        // The selected characters' boxes, in as many pieces as a line
        // that mixes directions needs.
        let pieces = layout.selection_pieces(a, b);
        let mut on_line = vec![false; layout.lines.len()];
        for (li, x0, x1) in pieces {
            if let Some(line) = layout.lines.get(li) {
                quads.push(quad(x0, x1, line));
                if let Some(f) = on_line.get_mut(li) {
                    *f = true;
                }
            }
        }
        // An empty line inside the selection shows a sliver.
        for (li, line) in layout.lines.iter().enumerate() {
            if !on_line[li]
                && line.start_char == line.end_char
                && a < line.start_char
                && b > line.end_char
            {
                let x = line.edges.first().copied().unwrap_or(0.0);
                quads.push(quad(x, x, line));
            }
        }
    }
    Some((top, bottom, quads))
}

fn word_start_before(text: &[char], from: usize) -> usize {
    let mut i = from.min(text.len());
    while i > 0 && text[i - 1].is_whitespace() {
        i -= 1;
    }
    while i > 0 && !text[i - 1].is_whitespace() {
        i -= 1;
    }
    i
}

fn word_end_after(text: &[char], from: usize) -> usize {
    let mut i = from.min(text.len());
    while i < text.len() && !text[i].is_whitespace() {
        i += 1;
    }
    while i < text.len() && text[i].is_whitespace() {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fonts_available() -> bool {
        !tracedraw_text::fonts().families().is_empty()
    }

    #[test]
    fn typing_moves_the_caret_and_edits_at_it() {
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        app.start_text(Point::new(10.0, 10.0), None);
        app.text_insert("Hello world");
        assert_eq!(app.edit_text(), "Hello world");
        assert_eq!(app.text_edit.as_ref().map(|t| t.caret), Some(11));
        // Shift+Left five times selects "world"; typing replaces it.
        for _ in 0..5 {
            app.text_move(CaretMove::Left, true);
        }
        assert_eq!(app.text_selected(), "world");
        app.text_insert("there");
        assert_eq!(app.edit_text(), "Hello there");
        // Home, then Delete removes the first character; End, Backspace the last.
        app.text_move(CaretMove::LineStart, false);
        app.text_delete_forward();
        app.text_move(CaretMove::LineEnd, false);
        app.text_delete_backward();
        assert_eq!(app.edit_text(), "ello ther");
        // Word moves.
        app.text_move(CaretMove::WordLeft, false);
        assert_eq!(app.text_edit.as_ref().map(|t| t.caret), Some(5));
        app.text_move(CaretMove::WordLeft, false);
        assert_eq!(app.text_edit.as_ref().map(|t| t.caret), Some(0));
        app.text_move(CaretMove::WordRight, false);
        assert_eq!(app.text_edit.as_ref().map(|t| t.caret), Some(5));
        // Typing at the caret inserts in the middle.
        app.text_insert("X");
        assert_eq!(app.edit_text(), "ello Xther");
        // One undo step undoes the whole typing session.
        app.finish_text();
        assert_eq!(app.engine.undo_label(), Some("Edit Text"));
    }

    #[test]
    fn selection_formatting_splits_spans() {
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        app.start_text(Point::new(0.0, 0.0), None);
        app.text_insert("abcdef");
        app.text_move(CaretMove::LineStart, false);
        app.text_move(CaretMove::Right, false);
        app.text_move(CaretMove::Right, true);
        app.text_move(CaretMove::Right, true);
        assert_eq!(app.text_selected(), "bc");
        app.text_apply_style(|s| s.bold = true);
        let spans = app.edit_spans().unwrap();
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[1].text, "bc");
        assert!(spans[1].bold && !spans[0].bold && !spans[2].bold);
        // Deleting the bold run merges the plain neighbours back.
        app.text_delete_backward();
        let spans = app.edit_spans().unwrap();
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "adef");
        // Select all and the double-click word selection.
        app.text_select_all();
        assert_eq!(app.text_selected(), "adef");
        app.text_select_word_at(2);
        assert_eq!(app.text_selected(), "adef");
    }

    /// Text "abcdef" with "cd" selected, typed in one step.
    fn text_with_cd_selected() -> App {
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        app.start_text(Point::new(0.0, 0.0), None);
        app.text_insert("abcdef");
        app.text_move(CaretMove::LineStart, false);
        app.text_move(CaretMove::Right, false);
        app.text_move(CaretMove::Right, false);
        app.text_move(CaretMove::Right, true);
        app.text_move(CaretMove::Right, true);
        assert_eq!(app.text_selected(), "cd");
        app
    }

    fn edited_shape(app: &App) -> tracedraw_core::Shape {
        let id = app.text_edit.as_ref().map(|t| t.shape).expect("editing");
        app.doc().find_shape(id).cloned().expect("text")
    }

    #[test]
    fn a_colour_picked_with_characters_selected_colours_only_them() {
        let mut app = text_with_cd_selected();
        let object_fill = edited_shape(&app).fill.clone();
        let red = Color::rgb8(255, 0, 0);
        // A palette swatch (left click): the fill of "cd" only.
        app.apply_fill(Fill::Solid(red));
        let spans = app.edit_spans().expect("spans");
        let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, ["ab", "cd", "ef"]);
        assert_eq!(spans[1].fill, Some(Fill::Solid(red)));
        assert!(spans[0].fill.is_none() && spans[2].fill.is_none());
        assert_eq!(edited_shape(&app).fill, object_fill);
        // Its own step, not merged into the typing, and the colour is in
        // the document palette.
        assert_eq!(app.engine.undo_label(), Some("Character Fill"));
        assert!(app.doc().palette.contains(&red));
        // A right click: the outline of "cd" only, from the default pen.
        let blue = Color::rgb8(0, 0, 255);
        app.apply_outline_color(Some(blue));
        let spans = app.edit_spans().expect("spans");
        let o = spans[1].outline.clone().expect("outline");
        assert_eq!(o.color, blue);
        assert!(o.width > 0.0);
        assert!(spans[0].outline.is_none() && spans[2].outline.is_none());
        assert_eq!(app.engine.undo_label(), Some("Character Outline"));
        // The no-colour swatch takes the outline off them.
        app.apply_outline_color(None);
        let spans = app.edit_spans().expect("spans");
        assert!(spans[1].outline.as_ref().is_some_and(|o| o.width <= 0.0));
        // Each colour is one undo step; undoing them leaves the typing.
        for _ in 0..3 {
            let _ = app.engine.undo();
        }
        assert_eq!(app.engine.undo_label(), Some("Edit Text"));
        let spans = app.edit_spans().expect("spans");
        assert!(spans
            .iter()
            .all(|s| s.fill.is_none() && s.outline.is_none()));
        assert_eq!(spans_text(&spans), "abcdef");
    }

    #[test]
    fn without_a_character_selection_the_colour_goes_to_the_object() {
        let mut app = text_with_cd_selected();
        app.text_move(CaretMove::LineEnd, false);
        let red = Color::rgb8(255, 0, 0);
        app.apply_fill(Fill::Solid(red));
        assert_eq!(edited_shape(&app).fill, Fill::Solid(red));
        let spans = app.edit_spans().expect("spans");
        assert!(spans.iter().all(|s| s.fill.is_none()));
    }

    #[test]
    fn the_bar_changes_only_what_it_changed_on_the_selection() {
        let mut app = text_with_cd_selected();
        // "d" in another font first.
        app.text_move(CaretMove::LineStart, false);
        for _ in 0..3 {
            app.text_move(CaretMove::Right, false);
        }
        app.text_move(CaretMove::Right, true);
        assert_eq!(app.text_selected(), "d");
        app.text_apply_style(|s| s.font_family = "Other".into());
        app.text_move(CaretMove::LineStart, false);
        app.text_move(CaretMove::Right, false);
        app.text_move(CaretMove::Right, false);
        app.text_move(CaretMove::Right, true);
        app.text_move(CaretMove::Right, true);
        let id = app.text_edit.as_ref().map(|t| t.shape).expect("editing");
        app.sync_text_defaults_from(id);
        // A new size keeps both fonts; it reaches the selection only.
        app.text_size_pt = 40.0;
        app.apply_text_style();
        let spans = app.edit_spans().expect("spans");
        let by_text = |t: &str| spans.iter().find(|s| s.text == t).cloned().expect(t);
        assert!((by_text("c").size_pt - 40.0).abs() < 1e-9, "{spans:?}");
        assert!((by_text("d").size_pt - 40.0).abs() < 1e-9);
        assert_eq!(by_text("d").font_family, "Other");
        assert_ne!(by_text("c").font_family, "Other");
        assert!((by_text("ab").size_pt - 40.0).abs() > 1e-9);
        assert!((by_text("ef").size_pt - 40.0).abs() > 1e-9);
    }

    #[test]
    fn caret_geometry_and_hits_follow_the_layout() {
        if !fonts_available() {
            return;
        }
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        app.start_text(Point::new(20.0, 30.0), None);
        app.text_insert("ab\ncd");
        let (top, bottom, quads) = app.text_caret_geometry().unwrap();
        assert!(top.y > bottom.y);
        assert!(quads.is_empty());
        // The caret after "cd" is on the second line, below the origin.
        assert!(top.y < 30.0);
        // Up moves to the first line keeping the x; Down comes back.
        app.text_move(CaretMove::Up, false);
        assert_eq!(app.text_edit.as_ref().map(|t| t.caret), Some(2));
        app.text_move(CaretMove::Down, false);
        assert_eq!(app.text_edit.as_ref().map(|t| t.caret), Some(5));
        // Clicking left of the first character puts the caret at 0.
        let hit = app.text_hit_char(Point::new(19.0, 30.5));
        assert_eq!(hit, Some(0));
        // A selection across both lines gives two quads.
        app.text_select_all();
        let (_, _, quads) = app.text_caret_geometry().unwrap();
        assert_eq!(quads.len(), 2);
    }
}
