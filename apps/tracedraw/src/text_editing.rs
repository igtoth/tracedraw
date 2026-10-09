//! In-place text editing: caret, selection, typing, deleting and
//! character formatting of the text object being edited with the Text
//! tool. Positions are character indices of the concatenated span text
//! (`tracedraw_core::spans_text`); geometry comes from the text layout's
//! line boxes.

use crate::app::App;
use tracedraw_core::document::text_outline::TextRequest;
use tracedraw_core::geometry::{Affine, Point, Rect};
use tracedraw_core::{
    spans_apply, spans_char_count, spans_delete, spans_insert, spans_text, Command, ShapeId,
    ShapeKind, TextSpan,
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
        Some((layout, s.transform * shift))
    }

    /// Replace the spans of the text being edited. Keystrokes collapse
    /// into one undo step per text object.
    pub fn set_edit_spans(&mut self, spans: Vec<TextSpan>) {
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
        if self.engine.undo_label() == Some("Edit Text") {
            let _ = self.engine.undo();
        }
        let _ = self.engine.run_with_label(
            &Command::SetShapeKind {
                shape: te.shape,
                kind,
            },
            "Edit Text",
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
        for ev in &events {
            match ev {
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
            CaretMove::Left => {
                if te.has_selection() && !extend {
                    a
                } else {
                    te.caret.saturating_sub(1)
                }
            }
            CaretMove::Right => {
                if te.has_selection() && !extend {
                    b
                } else {
                    (te.caret + 1).min(len)
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
                    let target = if m == CaretMove::Up {
                        li.checked_sub(1)
                    } else {
                        Some(li + 1)
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
        let (x, base, asc, desc) = layout.caret(te.caret).or_else(|| {
            // Empty text: a caret of the first span's size at the origin.
            let size_mm = self.edit_spans()?.first()?.size_pt * 25.4 / 72.0;
            Some((0.0, 0.0, size_mm * 0.8, size_mm * 0.2))
        })?;
        let top = to_page * Point::new(x, base + asc);
        let bottom = to_page * Point::new(x, base - desc);
        let mut quads = Vec::new();
        if te.has_selection() {
            let (a, b) = te.selection();
            for line in &layout.lines {
                let s = a.max(line.start_char);
                let e = b.min(line.end_char);
                if s > e || (s == e && !(a < line.start_char && b > line.end_char)) {
                    continue;
                }
                let x0 = line.edges.get(s - line.start_char).copied().unwrap_or(0.0);
                let x1 = line.edges.get(e - line.start_char).copied().unwrap_or(x0);
                let r = Rect::new(
                    x0,
                    line.baseline - line.descent,
                    x1.max(x0 + 0.3),
                    line.baseline + line.ascent,
                );
                quads.push([
                    to_page * Point::new(r.x0, r.y0),
                    to_page * Point::new(r.x1, r.y0),
                    to_page * Point::new(r.x1, r.y1),
                    to_page * Point::new(r.x0, r.y1),
                ]);
            }
        }
        Some((top, bottom, quads))
    }
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
