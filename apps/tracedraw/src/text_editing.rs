//! In-place text editing: caret, selection, typing, deleting and
//! character formatting of the text object being edited with the Text
//! tool. Positions are character indices of the concatenated span text
//! (`tracedraw_core::spans_text`); geometry comes from the text layout's
//! line boxes.

use crate::app::App;
use tracedraw_core::document::text_outline::{merge_stretch, TextRequest};
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
    /// How deep the undo list was right after this editing session's own
    /// step: typing on folds into that step only while it is still the
    /// last one (never into a step of another text, #373).
    pub step: Option<usize>,
}

impl TextEdit {
    pub fn at_end(shape: ShapeId, len: usize) -> Self {
        TextEdit {
            shape,
            caret: len,
            anchor: len,
            goal_x: None,
            step: None,
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

    /// The text objects Vertical Text acts on: the selected ones and the
    /// one being edited.
    fn vertical_text_targets(&self) -> Vec<tracedraw_core::Shape> {
        let mut out: Vec<tracedraw_core::Shape> = self
            .selected_shapes()
            .into_iter()
            .filter(|s| matches!(s.kind, ShapeKind::Text { .. }))
            .collect();
        if let Some(te) = &self.text_edit {
            if !out.iter().any(|s| s.id == te.shape) {
                if let Some(s) = self.doc().find_shape(te.shape) {
                    out.push(s.clone());
                }
            }
        }
        out
    }

    /// Vertical Text is offered (Text menu, property bar) with a CJK
    /// interface language, and for text that holds letters which stand
    /// upright in columns (ideographs, kana, hangul), whatever the
    /// language.
    pub fn vertical_text_offered(&self) -> bool {
        if vertical_text_language(&crate::i18n::language()) {
            return true;
        }
        self.vertical_text_targets().iter().any(|s| match &s.kind {
            ShapeKind::Text { spans, para, .. } => {
                para.vertical
                    || spans
                        .iter()
                        .any(|sp| sp.text.chars().any(tracedraw_text::upright))
            }
            _ => false,
        })
    }

    /// Any text acted on is vertical (the menu's check, the bar's button).
    pub fn vertical_text_on(&self) -> bool {
        self.vertical_text_targets()
            .iter()
            .any(|s| matches!(&s.kind, ShapeKind::Text { para, .. } if para.vertical))
    }

    /// The text being edited is vertical.
    pub fn editing_vertical(&self) -> bool {
        self.text_edit
            .as_ref()
            .and_then(|te| self.doc().find_shape(te.shape))
            .is_some_and(|s| matches!(&s.kind, ShapeKind::Text { para, .. } if para.vertical))
    }

    /// Text > Vertical Text: set the selected text (and the one being
    /// edited) in columns, or back in lines. One undo step.
    pub fn set_vertical_text(&mut self, on: bool) {
        let cmds: Vec<Command> = self
            .vertical_text_targets()
            .into_iter()
            .filter_map(|s| {
                let mut kind = s.kind.clone();
                let ShapeKind::Text { para, .. } = &mut kind else {
                    return None;
                };
                if para.vertical == on {
                    return None;
                }
                para.vertical = on;
                Some(Command::SetShapeKind { shape: s.id, kind })
            })
            .collect();
        if cmds.is_empty() {
            return;
        }
        if let Err(e) = self.engine.run_batch("Vertical Text", &cmds) {
            self.status = e.to_string();
        }
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
        // The text keeps its own hyphenation: typing took the setting for
        // new text, turning it on or off for the text edited (#470).
        let kind = ShapeKind::Text {
            spans,
            origin,
            frame,
            align,
            para,
            on_path,
        };
        let cmd = Command::SetShapeKind {
            shape: te.shape,
            kind,
        };
        // Typing goes on in the step this session made, while that step
        // is still the last one; otherwise (the first keystroke, another
        // text edited since, any other change) it is a step of its own.
        let depth = self.engine.history_labels().0.len();
        let own_step = te.step == Some(depth) && self.engine.undo_label() == Some(label);
        let _ = if merge && own_step {
            self.engine.amend(&cmd)
        } else {
            self.engine.run_with_label(&cmd, label)
        };
        let depth = self.engine.history_labels().0.len();
        if let Some(t) = self.text_edit.as_mut() {
            t.step = Some(depth);
        }
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
        // Ctrl+Space leaves the text for the Pick tool, as in the
        // reference program (Space alone types a space), before any space
        // the key would type (#478).
        if ctx.input(|i| (i.modifiers.command || i.modifiers.ctrl) && i.key_pressed(Key::Space)) {
            self.finish_text();
            self.set_tool(crate::tools::Tool::Pick);
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        // Alt let go after digits: the character they number (0 first: code
        // page 1252, as the Glyphs docker shows; else the Unicode number).
        if !self.alt_code.is_empty() && !ctx.input(|i| i.modifiers.alt) {
            let code = std::mem::take(&mut self.alt_code);
            if let Some(ch) = alt_code_char(&code) {
                self.text_insert(&ch.to_string());
                self.alt_composed = Some(ch);
            }
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
                // The digits of an Alt code also come as text (the keypad
                // with Num Lock on): the code names the character. The
                // character the system composes of the same code, should
                // it come too, is the one typed already (#469).
                Event::Text(t)
                    if !self.alt_code.is_empty() && t.chars().all(|c| c.is_ascii_digit()) => {}
                Event::Text(t) if self.alt_composed.is_some_and(|c| t.chars().eq([c])) => {
                    self.alt_composed = None;
                }
                Event::Text(t) => {
                    self.alt_composed = None;
                    self.text_insert(t);
                }
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
                    physical_key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    self.alt_composed = None;
                    let shift = modifiers.shift;
                    let ctrl = modifiers.command;
                    // Alt and digits: a character by its number, typed when
                    // Alt is let go. On Windows too: the windowing layer
                    // drops the character the system makes of them (#469).
                    // The physical key reads the keypad with Num Lock off.
                    if modifiers.alt && !ctrl {
                        if let Some(d) = physical_key.and_then(digit_of).or_else(|| digit_of(*key))
                        {
                            self.alt_code.push(d);
                            continue;
                        }
                    }
                    // In vertical text the caret goes down the column with
                    // Down and across the columns with Left and Right.
                    let vertical = self.editing_vertical();
                    match key {
                        Key::Num8 if ctrl => self.step_text_size(true),
                        Key::Num2 if ctrl => self.step_text_size(false),
                        Key::Backspace => self.text_delete_backward(),
                        Key::Delete => self.text_delete_forward(),
                        Key::Enter if !shift => self.text_insert("\n"),
                        Key::Enter => self.text_insert("\u{2028}"),
                        Key::Tab => self.text_insert("\t"),
                        Key::ArrowUp if vertical && !ctrl => self.text_move(CaretMove::Left, shift),
                        Key::ArrowDown if vertical && !ctrl => {
                            self.text_move(CaretMove::Right, shift)
                        }
                        Key::ArrowLeft if vertical && !ctrl => {
                            self.text_move(CaretMove::Down, shift)
                        }
                        Key::ArrowRight if vertical && !ctrl => {
                            self.text_move(CaretMove::Up, shift)
                        }
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
        let mut web_link = None;
        if self.settings.autocorrect.enabled {
            let typed: String = spans_text(&spans).chars().take(caret).collect();
            if self.settings.autocorrect.auto_hyperlinks {
                web_link = crate::autocorrect::typed_web_address(&typed);
            }
            let style = self.settings.autocorrect.style();
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
        // A web address just typed links the text, unless it has a link.
        if let Some(link) = web_link {
            let unlinked = self
                .doc()
                .find_shape(te.shape)
                .is_some_and(|s| s.link.is_none());
            if unlinked {
                self.run(Command::SetLink {
                    shapes: vec![te.shape],
                    link: Some(link),
                });
            }
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

    /// The span of the first selected character (the Properties docker
    /// shows the selected characters' own fill and outline); `None`
    /// without a selection.
    pub fn text_selection_span(&self) -> Option<TextSpan> {
        if !self.text_selection_active() {
            return None;
        }
        let te = self.text_edit.as_ref()?;
        let spans = self.edit_spans()?;
        let (a, _) = te.selection();
        let mut seen = 0usize;
        spans
            .iter()
            .find(|sp| {
                let n = sp.char_count();
                let hit = a < seen + n;
                seen += n;
                hit
            })
            .or_else(|| spans.last())
            .cloned()
    }

    /// The fill the selected characters show: their own, else the text
    /// object's. `None` without a selection.
    pub fn text_selection_fill(&self) -> Option<Fill> {
        let sp = self.text_selection_span()?;
        if let Some(f) = sp.fill {
            return Some(f);
        }
        let te = self.text_edit.as_ref()?;
        self.doc().find_shape(te.shape).map(|s| s.fill.clone())
    }

    /// The outline the selected characters show: their own (width 0:
    /// none), else the text object's. `None` without a selection.
    pub fn text_selection_stroke(&self) -> Option<Option<Stroke>> {
        let sp = self.text_selection_span()?;
        if let Some(o) = sp.outline {
            return Some((o.width > 0.0).then_some(o));
        }
        let te = self.text_edit.as_ref()?;
        self.doc().find_shape(te.shape).map(|s| s.stroke.clone())
    }

    /// The selected characters get this outline (`None`: no outline), as
    /// one undo step ("Character Outline"). False without a selection.
    pub fn text_apply_outline(&mut self, stroke: Option<Stroke>) -> bool {
        self.text_style_selection("Character Outline", move |sp| {
            sp.outline = Some(stroke.clone().unwrap_or(Stroke {
                width: 0.0,
                ..Stroke::default()
            }));
        })
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
    let shift = tracedraw_core::document::text_layout_shift(
        *origin,
        *frame,
        on_path.is_some(),
        para.vertical,
    );
    // The lines of vertical text are reported as written; their frame
    // turns them into the columns.
    let frame = layout.line_frame();
    Some((layout, parent * s.transform * shift * frame))
}

/// The interface languages for which Vertical Text is always offered:
/// the CJK ones.
pub fn vertical_text_language(code: &str) -> bool {
    matches!(code, "ja" | "zh-CN" | "zh-TW" | "ko")
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
        // The stretches the selected characters cover on a line: one for
        // plain text, from the right for right-to-left text (#629),
        // several where the directions mix.
        let mut stretches: Vec<(f64, f64)> = Vec::new();
        for line in &layout.lines {
            let s = a.max(line.start_char);
            let e = b.min(line.end_char);
            if s > e || (s == e && !(a < line.start_char && b > line.end_char)) {
                continue;
            }
            stretches.clear();
            for i in s - line.start_char..e - line.start_char {
                if let (Some(&x0), Some(&x1)) = (line.starts.get(i), line.ends.get(i)) {
                    merge_stretch(&mut stretches, x0.min(x1), x0.max(x1));
                }
            }
            if stretches.is_empty() {
                // A line with nothing on it inside the selection.
                let x0 = line.edges.get(s - line.start_char).copied().unwrap_or(0.0);
                stretches.push((x0, x0));
            }
            for &(x0, x1) in &stretches {
                let r = Rect::new(
                    x0,
                    line.baseline - line.descent,
                    x1.max(x0 + 0.3),
                    line.baseline + line.ascent,
                );
                quads.push([
                    *to_page * Point::new(r.x0, r.y0),
                    *to_page * Point::new(r.x1, r.y0),
                    *to_page * Point::new(r.x1, r.y1),
                    *to_page * Point::new(r.x0, r.y1),
                ]);
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

/// The digit a key types, for Alt codes.
fn digit_of(key: egui::Key) -> Option<char> {
    use egui::Key::*;
    Some(match key {
        Num0 => '0',
        Num1 => '1',
        Num2 => '2',
        Num3 => '3',
        Num4 => '4',
        Num5 => '5',
        Num6 => '6',
        Num7 => '7',
        Num8 => '8',
        Num9 => '9',
        _ => return None,
    })
}

/// The character an Alt code names: with a leading zero a code page 1252
/// number (Alt+0169 is ©), else a Unicode number.
pub fn alt_code_char(code: &str) -> Option<char> {
    let n: u32 = code.parse().ok()?;
    if code.starts_with('0') {
        crate::ui::glyphs_docker::from_cp1252(u8::try_from(n).ok()?)
    } else {
        char::from_u32(n).filter(|c| !c.is_control())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Alt codes as the Glyphs docker shows them: a leading zero reads
    /// code page 1252, else the Unicode number (#447).
    #[test]
    fn alt_codes_name_characters() {
        assert_eq!(alt_code_char("0169"), Some('©'));
        assert_eq!(alt_code_char("0128"), Some('€'));
        assert_eq!(alt_code_char("8364"), Some('€'));
        assert_eq!(alt_code_char("65"), Some('A'));
        assert_eq!(alt_code_char("0129"), None, "no character there");
        assert_eq!(alt_code_char("7"), None, "a control character");
        assert_eq!(alt_code_char(""), None);
    }

    fn fonts_available() -> bool {
        !tracedraw_text::fonts().families().is_empty()
    }

    #[test]
    fn a_scaled_text_shows_and_takes_its_size_as_seen() {
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        app.text_size_pt = 24.0;
        app.start_text(Point::new(10.0, 10.0), None);
        app.text_insert("Size");
        app.finish_text();
        let id = *app.selection.first().expect("text selected");
        let sizes = |app: &App| match app.doc().find_shape(id).map(|s| &s.kind) {
            Some(ShapeKind::Text { spans, .. }) => {
                spans.iter().map(|s| s.size_pt).collect::<Vec<_>>()
            }
            other => panic!("not a text: {other:?}"),
        };
        // Twice as large with the Pick tool: the bar reads 48 pt.
        app.transform_selection(Affine::scale(2.0));
        app.sync_text_defaults_from(id);
        assert!(
            (app.text_size_pt - 48.0).abs() < 1e-9,
            "{}",
            app.text_size_pt
        );
        // 36 pt typed: it shows 36 pt (18 pt before its scale).
        app.text_size_pt = 36.0;
        app.apply_text_style();
        assert!(sizes(&app).iter().all(|s| (s - 18.0).abs() < 1e-9));
        app.sync_text_defaults_from(id);
        assert!((app.text_size_pt - 36.0).abs() < 1e-9);
        // Another field changed: the size stays as it is, with no drift.
        app.text_bold = !app.text_bold;
        app.apply_text_style();
        assert!(sizes(&app).iter().all(|s| (s - 18.0).abs() < 1e-12));
        // Stretched sideways only: the size follows the height.
        app.transform_selection(Affine::scale_non_uniform(3.0, 1.0));
        app.sync_text_defaults_from(id);
        assert!((app.text_size_pt - 36.0).abs() < 1e-9);
    }

    /// Options > Text > Autocorrect > Automatically create hyperlinks: a
    /// web address typed and finished with a space links the text,
    /// keeping a link it already has; off, nothing is linked.
    #[test]
    fn a_typed_web_address_links_the_text() {
        let mut app = App::headless();
        app.start_text(Point::new(10.0, 10.0), None);
        for c in "see www.tracedraw.org ".chars() {
            app.text_insert(&c.to_string());
        }
        let id = app.text_edit.as_ref().map(|t| t.shape).expect("editing");
        assert_eq!(app.doc().find_shape(id).and_then(|s| s.link.clone()), None);
        app.settings.autocorrect.auto_hyperlinks = true;
        for c in "or www.tracedraw.org ".chars() {
            app.text_insert(&c.to_string());
        }
        assert_eq!(
            app.doc()
                .find_shape(id)
                .and_then(|s| s.link.clone())
                .as_deref(),
            Some("http://www.tracedraw.org")
        );
        assert!(app.edit_text().ends_with("www.tracedraw.org "));
        for c in "https://example.com ".chars() {
            app.text_insert(&c.to_string());
        }
        assert_eq!(
            app.doc()
                .find_shape(id)
                .and_then(|s| s.link.clone())
                .as_deref(),
            Some("http://www.tracedraw.org"),
            "a link the text has stays"
        );
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

    /// Editing a second text keeps the edit of the first: each text's
    /// typing is a step of its own (#373).
    #[test]
    fn editing_a_second_text_keeps_the_edit_of_the_first() {
        let mut app = App::headless();
        app.settings.autocorrect.enabled = false;
        app.start_text(Point::new(10.0, 10.0), None);
        app.text_insert("First");
        let first = app.text_edit.as_ref().map(|t| t.shape).expect("first");
        app.finish_text();
        app.start_text(Point::new(10.0, 60.0), None);
        app.text_insert("Second");
        let second = app.text_edit.as_ref().map(|t| t.shape).expect("second");
        app.finish_text();
        let text_of = |app: &App, id| match app.doc().find_shape(id).map(|s| s.kind.clone()) {
            Some(ShapeKind::Text { spans, .. }) => spans_text(&spans),
            other => panic!("{other:?}"),
        };
        // Edit the first, finish, then edit the second.
        app.begin_text_edit(first);
        app.text_insert(" edited");
        app.text_insert("!");
        app.finish_text();
        assert_eq!(text_of(&app, first), "First edited!");
        app.begin_text_edit(second);
        app.text_insert(" edited");
        assert_eq!(text_of(&app, first), "First edited!");
        assert_eq!(text_of(&app, second), "Second edited");
        // Straight from one text to the other, without finishing first.
        app.text_insert("?");
        app.begin_text_edit(first);
        app.text_insert("#");
        assert_eq!(text_of(&app, second), "Second edited?");
        assert_eq!(text_of(&app, first), "First edited!#");
        app.finish_text();
        // Undo takes back one text's typing at a time.
        app.undo();
        assert_eq!(text_of(&app, first), "First edited!");
        assert_eq!(text_of(&app, second), "Second edited?");
        app.undo();
        assert_eq!(text_of(&app, second), "Second");
        assert_eq!(text_of(&app, first), "First edited!");
        app.undo();
        assert_eq!(text_of(&app, first), "First");
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

    /// The Properties docker shows the selected characters' own fill and
    /// outline (the object's where they have none) and its edits go to
    /// them alone (#304).
    #[test]
    fn the_selected_characters_show_their_own_fill_and_outline() {
        let mut app = text_with_cd_selected();
        let object_fill = edited_shape(&app).fill.clone();
        // Unstyled characters show the object's.
        assert_eq!(app.text_selection_fill(), Some(object_fill.clone()));
        assert_eq!(
            app.text_selection_stroke(),
            Some(edited_shape(&app).stroke.clone())
        );
        let red = Fill::Solid(Color::rgb8(255, 0, 0));
        assert!(app.text_apply_fill(red.clone()));
        let stroke = Stroke::new(Color::rgb8(0, 0, 200), 0.5);
        assert!(app.text_apply_outline(Some(stroke.clone())));
        // Selected again later: their own fill and outline show.
        app.text_move(CaretMove::LineStart, false);
        app.text_move(CaretMove::Right, false);
        app.text_move(CaretMove::Right, false);
        app.text_move(CaretMove::Right, true);
        assert_eq!(app.text_selected(), "c");
        assert_eq!(app.text_selection_fill(), Some(red.clone()));
        assert_eq!(app.text_selection_stroke(), Some(Some(stroke.clone())));
        assert_eq!(
            app.text_selection_span().map(|sp| sp.text),
            Some("cd".to_string())
        );
        // The object itself kept its fill.
        assert_eq!(edited_shape(&app).fill, object_fill);
        // No outline on the characters: shows as none, kept as width 0.
        assert!(app.text_apply_outline(None));
        assert_eq!(app.text_selection_stroke(), Some(None));
        // Without a selection there is nothing to show.
        app.text_move(CaretMove::LineEnd, false);
        assert_eq!(app.text_selection_fill(), None);
        assert_eq!(app.text_selection_span(), None);
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

    /// Vertical text (#257): the caret is a bar across the column, the
    /// selection a box down it, and a click down the column finds the
    /// character there; Text > Vertical Text switches the selected text
    /// in one undo step; the command is offered for CJK languages and
    /// for text in those scripts.
    #[test]
    fn vertical_text_has_carets_across_its_columns_and_a_switch() {
        assert!(vertical_text_language("ja") && vertical_text_language("zh-TW"));
        assert!(!vertical_text_language("en") && !vertical_text_language("pt-BR"));
        let mut app = App::headless();
        let id = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("abcd", "Selawik", 24.0)],
                origin: Point::new(50.0, 100.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            })
            .expect("a layer");
        app.select(vec![id]);
        assert!(
            !app.vertical_text_offered(),
            "Latin text, English interface"
        );
        assert!(!app.vertical_text_on());
        let depth = app.engine.history_labels().0.len();
        app.set_vertical_text(true);
        assert!(app.vertical_text_on());
        assert!(
            app.vertical_text_offered(),
            "once vertical, the switch stays offered"
        );
        assert_eq!(app.engine.history_labels().0.len(), depth + 1);
        let Some((layout, to_page)) = app.text_layout_of(id) else {
            panic!("a layout");
        };
        if layout.glyphs == 0 {
            return;
        }
        assert!(layout.vertical);
        // The caret after "ab": a horizontal bar, below the origin, to
        // the right of the column's axis (x = 50) by the letters' height.
        let (top, bottom, quads) = caret_geometry(&layout, &to_page, 2, (1, 3)).expect("caret");
        assert!((top.y - bottom.y).abs() < 1e-6, "{top:?} {bottom:?}");
        assert!(top.y < 100.0, "{top:?}");
        assert!(
            top.x > 50.0 && bottom.x <= 50.0 + 1e-6,
            "{top:?} {bottom:?}"
        );
        // The selection "bc": one box down the column, as deep as the
        // two letters' advances and as wide as the line's height.
        assert_eq!(quads.len(), 1);
        let q = quads[0];
        let (w, h) = (
            q.iter().map(|p| p.x).fold(f64::MIN, f64::max)
                - q.iter().map(|p| p.x).fold(f64::MAX, f64::min),
            q.iter().map(|p| p.y).fold(f64::MIN, f64::max)
                - q.iter().map(|p| p.y).fold(f64::MAX, f64::min),
        );
        let line = &layout.lines[0];
        assert!((h - (line.edges[3] - line.edges[1])).abs() < 1e-6, "{q:?}");
        assert!((w - (line.ascent + line.descent)).abs() < 1e-6, "{q:?}");
        // A point down the column, just past the second letter, hits the
        // caret place before the third.
        let edges = &layout.lines[0].edges;
        let down = to_page * Point::new(edges[2] * 0.75 + edges[3] * 0.25, 0.0);
        let local = to_page.inverse() * down;
        assert_eq!(layout.hit_char(local), Some(2));
        // Back to lines.
        app.set_vertical_text(false);
        assert!(!app.vertical_text_on());
        // Text in a CJK script offers the switch whatever the language.
        let cjk = app
            .new_shape(ShapeKind::Text {
                spans: vec![TextSpan::new("\u{65E5}\u{672C}", "Selawik", 24.0)],
                origin: Point::new(10.0, 10.0),
                frame: None,
                align: Default::default(),
                para: Default::default(),
                on_path: None,
            })
            .expect("a layer");
        app.select(vec![cjk]);
        assert!(app.vertical_text_offered());
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
