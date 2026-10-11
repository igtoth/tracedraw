//! Table editing: cell selection, text in cells, insert and delete rows and
//! columns, merge and split, distribute, conversions to and from text.

use crate::app::App;
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Point, Rect},
    Command, ShapeId, Table, TableCell, TextSpan,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableOp {
    RowAbove,
    RowBelow,
    ColLeft,
    ColRight,
    SelectCell,
    SelectRow,
    SelectCol,
    SelectTable,
    DeleteRow,
    DeleteCol,
    DeleteTable,
    DistributeRows,
    DistributeCols,
    Merge,
    Unmerge,
    SplitRows,
    SplitCols,
}

/// Cell editing state: which table and which cells are selected, plus the
/// caret inside the active cell's text.
#[derive(Debug, Clone, PartialEq)]
pub struct TableEdit {
    pub shape: ShapeId,
    /// Selected (row, col) pairs; the first one is the active cell.
    pub cells: Vec<(u32, u32)>,
    /// Caret as a character index of the active cell's spans.
    pub caret: usize,
    /// Other end of the selection inside the cell.
    pub anchor: usize,
}

impl TableEdit {
    pub fn new(shape: ShapeId, cells: Vec<(u32, u32)>, caret: usize) -> Self {
        TableEdit {
            shape,
            cells,
            caret,
            anchor: caret,
        }
    }

    pub fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    pub fn has_selection(&self) -> bool {
        self.caret != self.anchor
    }
}

impl App {
    /// The selected table shape, if the selection is exactly one table.
    pub fn selected_table(&self) -> Option<(ShapeId, Table)> {
        if self.selection.len() != 1 {
            return None;
        }
        let id = self.selection[0];
        let (_, s) = self.doc().shape(id).ok()?;
        match &s.kind {
            ShapeKind::Table(t) => Some((id, t.clone())),
            _ => None,
        }
    }

    fn set_table(&mut self, id: ShapeId, mut t: Table) {
        t.refit();
        self.run(Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Table(t),
        });
    }

    /// Start editing the cell under a local point (Table tool click).
    pub fn table_click(&mut self, id: ShapeId, local: Point, extend: bool) {
        let Some((_, s)) = self.doc().shape(id).ok() else {
            return;
        };
        let ShapeKind::Table(t) = &s.kind else {
            return;
        };
        let Some(ci) = t.cell_at(local) else {
            return;
        };
        let rc = (t.cells[ci].row, t.cells[ci].col);
        let len = tracedraw_core::spans_char_count(&t.cells[ci].text);
        match &mut self.table_edit {
            Some(e) if e.shape == id && extend => {
                if !e.cells.contains(&rc) {
                    e.cells.push(rc);
                }
            }
            _ => {
                self.table_edit = Some(TableEdit::new(id, vec![rc], len));
                // The caret goes to the character under the pointer.
                if let Some(idx) = self.cell_hit_char(local) {
                    if let Some(e) = self.table_edit.as_mut() {
                        e.caret = idx;
                        e.anchor = idx;
                    }
                }
            }
        }
    }

    /// Tab inside a table: commit and move to the next cell (wrapping rows).
    pub fn table_next_cell(&mut self, backwards: bool) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let Some((_, s)) = self.doc().shape(e.shape).ok() else {
            return;
        };
        let ShapeKind::Table(t) = &s.kind else {
            return;
        };
        let Some((r, c)) = e.cells.first().copied() else {
            return;
        };
        let (rows, cols) = (t.rows(), t.cols());
        let (mut nr, mut nc) = (r, c);
        for _ in 0..(rows * cols) {
            if backwards {
                if nc == 0 {
                    nc = cols - 1;
                    nr = if nr == 0 { rows - 1 } else { nr - 1 };
                } else {
                    nc -= 1;
                }
            } else {
                nc += 1;
                if nc >= cols {
                    nc = 0;
                    nr = (nr + 1) % rows;
                }
            }
            if !t.covered(nr, nc) {
                break;
            }
        }
        let len = cell_text(t, nr, nc).chars().count();
        self.table_edit = Some(TableEdit::new(e.shape, vec![(nr, nc)], len));
    }

    /// Move to the cell `dr` rows and `dc` columns away (arrow keys at a
    /// cell edge); the caret goes to the start or the end of its text.
    pub fn table_step_cell(&mut self, dr: i64, dc: i64, caret_at_end: bool) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let Some((_, s)) = self.doc().shape(e.shape).ok() else {
            return;
        };
        let ShapeKind::Table(t) = &s.kind else {
            return;
        };
        let Some((r, c)) = e.cells.first().copied() else {
            return;
        };
        let nr = r as i64 + dr;
        let nc = c as i64 + dc;
        if nr < 0 || nc < 0 || nr >= t.rows() as i64 || nc >= t.cols() as i64 {
            return;
        }
        let (mut nr, mut nc) = (nr as u32, nc as u32);
        // A covered cell belongs to a merged one: go to its anchor.
        if t.covered(nr, nc) {
            if let Some(anchor) = t.cells.iter().find(|cell| {
                cell.row <= nr
                    && nr < cell.row + cell.row_span
                    && cell.col <= nc
                    && nc < cell.col + cell.col_span
            }) {
                nr = anchor.row;
                nc = anchor.col;
            }
        }
        let len = cell_text(t, nr, nc).chars().count();
        let caret = if caret_at_end { len } else { 0 };
        self.table_edit = Some(TableEdit::new(e.shape, vec![(nr, nc)], caret));
    }

    /// Spans of the active cell.
    pub fn cell_spans(&self) -> Option<Vec<TextSpan>> {
        let e = self.table_edit.as_ref()?;
        let (_, s) = self.doc().shape(e.shape).ok()?;
        let ShapeKind::Table(t) = &s.kind else {
            return None;
        };
        let (r, c) = e.cells.first().copied()?;
        t.cells
            .iter()
            .find(|cell| cell.row == r && cell.col == c)
            .map(|cell| cell.text.clone())
    }

    /// Text of the active cell.
    pub fn cell_text(&self) -> String {
        self.cell_spans()
            .map(|s| tracedraw_core::spans_text(&s))
            .unwrap_or_default()
    }

    /// Spans of the active cell with one span guaranteed (the default
    /// style when the cell is empty), for editing.
    fn cell_spans_for_edit(&self) -> Option<Vec<TextSpan>> {
        let mut spans = self.cell_spans()?;
        if spans.is_empty() {
            spans.push(TextSpan::new(
                "",
                self.text_font.clone(),
                (self.text_size_pt / 2.0).max(8.0),
            ));
        }
        Some(spans)
    }

    /// Replace the active cell's spans; keystrokes collapse into one undo
    /// step per cell.
    fn set_cell_spans(&mut self, spans: Vec<TextSpan>) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let Some((_, s)) = self.doc().shape(e.shape).ok() else {
            return;
        };
        let ShapeKind::Table(t) = &s.kind else {
            return;
        };
        let mut t = t.clone();
        let Some((r, c)) = e.cells.first().copied() else {
            return;
        };
        if let Some(cell) = t
            .cells
            .iter_mut()
            .find(|cell| cell.row == r && cell.col == c)
        {
            cell.text = if tracedraw_core::spans_text(&spans).is_empty() {
                Vec::new()
            } else {
                spans
            };
        }
        grow_row_to_fit(&mut t, r, c);
        t.refit();
        if self.engine.undo_label() == Some("Edit Cell") {
            let _ = self.engine.undo();
        }
        let _ = self.engine.run_with_label(
            &Command::SetShapeKind {
                shape: e.shape,
                kind: ShapeKind::Table(t),
            },
            "Edit Cell",
        );
    }

    fn set_cell_caret(&mut self, caret: usize, extend: bool) {
        let len = self.cell_text().chars().count();
        if let Some(e) = self.table_edit.as_mut() {
            e.caret = caret.min(len);
            if !extend {
                e.anchor = e.caret;
            }
        }
    }

    /// Type into the active cell at the caret, replacing the selection.
    pub fn cell_insert(&mut self, text: &str) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let Some(mut spans) = self.cell_spans_for_edit() else {
            return;
        };
        let (a, b) = e.selection();
        tracedraw_core::spans_delete(&mut spans, a, b);
        tracedraw_core::spans_insert(&mut spans, a, text);
        self.set_cell_spans(spans);
        self.set_cell_caret(a + text.chars().count(), false);
    }

    pub fn cell_delete_backward(&mut self) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let Some(mut spans) = self.cell_spans_for_edit() else {
            return;
        };
        let (a, b) = if e.has_selection() {
            e.selection()
        } else if e.caret > 0 {
            (e.caret - 1, e.caret)
        } else {
            return;
        };
        tracedraw_core::spans_delete(&mut spans, a, b);
        self.set_cell_spans(spans);
        self.set_cell_caret(a, false);
    }

    pub fn cell_delete_forward(&mut self) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let Some(mut spans) = self.cell_spans_for_edit() else {
            return;
        };
        let len = tracedraw_core::spans_char_count(&spans);
        let (a, b) = if e.has_selection() {
            e.selection()
        } else if e.caret < len {
            (e.caret, e.caret + 1)
        } else {
            return;
        };
        tracedraw_core::spans_delete(&mut spans, a, b);
        self.set_cell_spans(spans);
        self.set_cell_caret(a, false);
    }

    /// Move the caret inside the cell; at the cell's edges the arrow keys
    /// step to the neighbouring cell.
    pub fn cell_move(&mut self, m: crate::text_editing::CaretMove, extend: bool) {
        use crate::text_editing::CaretMove;
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let text: Vec<char> = self.cell_text().chars().collect();
        let len = text.len();
        let caret = match m {
            CaretMove::Left if e.caret == 0 && !extend => {
                self.table_step_cell(0, -1, true);
                return;
            }
            CaretMove::Right if e.caret >= len && !extend => {
                self.table_step_cell(0, 1, false);
                return;
            }
            CaretMove::Left => e.caret.saturating_sub(1),
            CaretMove::Right => (e.caret + 1).min(len),
            CaretMove::Up | CaretMove::Down => {
                let line_of = |i: usize| text.iter().take(i).filter(|c| **c == '\n').count();
                let lines = line_of(len);
                let cur = line_of(e.caret);
                if (m == CaretMove::Up && cur == 0) || (m == CaretMove::Down && cur == lines) {
                    if !extend {
                        let dr = if m == CaretMove::Up { -1 } else { 1 };
                        self.table_step_cell(dr, 0, m == CaretMove::Up);
                    }
                    return;
                }
                // The same column on the neighbouring line of the cell.
                let line_start = |line: usize| {
                    let mut seen = 0usize;
                    for (i, c) in text.iter().enumerate() {
                        if seen == line {
                            return i;
                        }
                        if *c == '\n' {
                            seen += 1;
                        }
                    }
                    len
                };
                let col = e.caret - line_start(cur);
                let target = if m == CaretMove::Up { cur - 1 } else { cur + 1 };
                let start = line_start(target);
                let end = text[start..]
                    .iter()
                    .position(|c| *c == '\n')
                    .map(|p| start + p)
                    .unwrap_or(len);
                (start + col).min(end)
            }
            CaretMove::LineStart | CaretMove::WordLeft => text[..e.caret.min(len)]
                .iter()
                .rposition(|c| *c == '\n')
                .map(|p| p + 1)
                .unwrap_or(0),
            CaretMove::LineEnd | CaretMove::WordRight => text[e.caret.min(len)..]
                .iter()
                .position(|c| *c == '\n')
                .map(|p| e.caret + p)
                .unwrap_or(len),
            CaretMove::TextStart => 0,
            CaretMove::TextEnd => len,
        };
        self.set_cell_caret(caret, extend);
    }

    pub fn cell_select_all(&mut self) {
        let len = self.cell_text().chars().count();
        if let Some(e) = self.table_edit.as_mut() {
            e.anchor = 0;
            e.caret = len;
        }
    }

    /// Restyle the selected characters of the cell, or the whole cell.
    pub fn cell_apply_style(&mut self, f: impl Fn(&mut TextSpan)) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let Some(mut spans) = self.cell_spans_for_edit() else {
            return;
        };
        if e.has_selection() {
            let (a, b) = e.selection();
            tracedraw_core::spans_apply(&mut spans, a, b, f);
        } else {
            for sp in spans.iter_mut() {
                f(sp);
            }
        }
        self.set_cell_spans(spans);
    }

    /// The selected text of the cell.
    pub fn cell_selected_text(&self) -> String {
        let Some(e) = self.table_edit.as_ref() else {
            return String::new();
        };
        let (a, b) = e.selection();
        self.cell_text().chars().skip(a).take(b - a).collect()
    }

    /// The active cell's text as the shape `Table::expand` draws it, in
    /// table-local space.
    fn cell_text_shape(&self) -> Option<tracedraw_core::Shape> {
        let e = self.table_edit.as_ref()?;
        let (_, s) = self.doc().shape(e.shape).ok()?;
        let ShapeKind::Table(t) = &s.kind else {
            return None;
        };
        let (r, c) = e.cells.first().copied()?;
        let cell = t.cells.iter().find(|cell| cell.row == r && cell.col == c)?;
        let inner = t.cell_rect(cell).inset(-t.padding);
        let spans = if cell.text.is_empty() {
            self.cell_spans_for_edit()?
        } else {
            cell.text.clone()
        };
        Some(tracedraw_core::Shape::new(
            ShapeId(0),
            ShapeKind::Text {
                spans,
                origin: Point::new(inner.x0, inner.y0),
                frame: Some(tracedraw_core::geometry::Size::new(
                    inner.width().max(0.1),
                    inner.height().max(0.1),
                )),
                align: cell.align,
                para: tracedraw_core::ParagraphStyle::default(),
                on_path: None,
            },
        ))
    }

    /// Character index under a table-local point in the active cell.
    pub fn cell_hit_char(&self, local: Point) -> Option<usize> {
        let tx = self.cell_text_shape()?;
        let (layout, to_local) = crate::text_editing::layout_of_text_shape(
            &tx,
            tracedraw_core::geometry::Affine::IDENTITY,
        )?;
        layout.hit_char(to_local.inverse() * local)
    }

    /// Caret segment and selection quads of the active cell, on the page.
    pub fn cell_caret_geometry(&self) -> Option<(Point, Point, Vec<[Point; 4]>)> {
        let e = self.table_edit.as_ref()?;
        let (_, s) = self.doc().shape(e.shape).ok()?;
        let tx = self.cell_text_shape()?;
        let (layout, to_page) = crate::text_editing::layout_of_text_shape(&tx, s.transform)?;
        crate::text_editing::caret_geometry(&layout, &to_page, e.caret, e.selection())
    }

    /// Keyboard input while a cell is active (Table tool). Returns true:
    /// the keys are consumed.
    pub fn table_keyboard(&mut self, ctx: &egui::Context) -> bool {
        use crate::text_editing::CaretMove;
        use egui::{Event, Key};
        let events = ctx.input(|i| i.events.clone());
        // While a cell is active every key belongs to it, so letters never
        // reach the single-key shortcuts (align, tools) of the canvas.
        for ev in &events {
            match ev {
                Event::Text(t) | Event::Paste(t) => self.cell_insert(t),
                Event::Copy => {
                    let sel = self.cell_selected_text();
                    if !sel.is_empty() {
                        ctx.copy_text(sel);
                    }
                }
                Event::Cut => {
                    let sel = self.cell_selected_text();
                    if !sel.is_empty() {
                        ctx.copy_text(sel);
                        self.cell_delete_backward();
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
                        Key::Backspace => self.cell_delete_backward(),
                        Key::Delete => self.cell_delete_forward(),
                        Key::Enter => self.cell_insert("\n"),
                        Key::Tab => self.table_next_cell(shift),
                        Key::ArrowLeft => self.cell_move(CaretMove::Left, shift),
                        Key::ArrowRight => self.cell_move(CaretMove::Right, shift),
                        Key::ArrowUp => self.cell_move(CaretMove::Up, shift),
                        Key::ArrowDown => self.cell_move(CaretMove::Down, shift),
                        Key::Home if ctrl => self.cell_move(CaretMove::TextStart, shift),
                        Key::End if ctrl => self.cell_move(CaretMove::TextEnd, shift),
                        Key::Home => self.cell_move(CaretMove::LineStart, shift),
                        Key::End => self.cell_move(CaretMove::LineEnd, shift),
                        Key::A if ctrl => self.cell_select_all(),
                        Key::B if ctrl => {
                            let on = !self.text_bold;
                            self.text_bold = on;
                            self.cell_apply_style(move |s| s.bold = on);
                        }
                        Key::I if ctrl => {
                            let on = !self.text_italic;
                            self.text_italic = on;
                            self.cell_apply_style(move |s| s.italic = on);
                        }
                        Key::U if ctrl => {
                            let on = !self.text_underline;
                            self.text_underline = on;
                            self.cell_apply_style(move |s| s.underline = on);
                        }
                        Key::Escape => self.table_edit = None,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        true
    }

    pub fn table_op(&mut self, op: TableOp) {
        let Some((id, mut t)) = self.selected_table() else {
            return;
        };
        let sel: Vec<(u32, u32)> = self
            .table_edit
            .as_ref()
            .filter(|e| e.shape == id)
            .map(|e| e.cells.clone())
            .unwrap_or_else(|| vec![(0, 0)]);
        let (ar, ac) = sel.first().copied().unwrap_or((0, 0));
        match op {
            TableOp::RowAbove | TableOp::RowBelow => {
                let at = if op == TableOp::RowAbove { ar } else { ar + 1 };
                let h = t.row_heights.get(ar as usize).copied().unwrap_or(10.0);
                t.row_heights.insert(at as usize, h);
                for c in t.cells.iter_mut() {
                    if c.row >= at {
                        c.row += 1;
                    } else if c.row + c.row_span > at {
                        c.row_span += 1;
                    }
                }
                for col in 0..t.cols() {
                    if !t.covered(at, col) {
                        t.cells.push(blank(at, col));
                    }
                }
            }
            TableOp::ColLeft | TableOp::ColRight => {
                let at = if op == TableOp::ColLeft { ac } else { ac + 1 };
                let w = t.col_widths.get(ac as usize).copied().unwrap_or(20.0);
                t.col_widths.insert(at as usize, w);
                for c in t.cells.iter_mut() {
                    if c.col >= at {
                        c.col += 1;
                    } else if c.col + c.col_span > at {
                        c.col_span += 1;
                    }
                }
                for row in 0..t.rows() {
                    if !t.covered(row, at) {
                        t.cells.push(blank(row, at));
                    }
                }
            }
            TableOp::SelectCell => {
                let len = cell_text(&t, ar, ac).chars().count();
                self.table_edit = Some(TableEdit::new(id, vec![(ar, ac)], len));
                return;
            }
            TableOp::SelectRow => {
                let cells: Vec<(u32, u32)> = (0..t.cols()).map(|c| (ar, c)).collect();
                let len = cell_text(&t, ar, ac).chars().count();
                self.table_edit = Some(TableEdit::new(id, cells, len));
                return;
            }
            TableOp::SelectCol => {
                let cells: Vec<(u32, u32)> = (0..t.rows()).map(|r| (r, ac)).collect();
                let len = cell_text(&t, ar, ac).chars().count();
                self.table_edit = Some(TableEdit::new(id, cells, len));
                return;
            }
            TableOp::SelectTable => {
                self.table_edit = None;
                self.select(vec![id]);
                return;
            }
            TableOp::DeleteRow => {
                if t.rows() <= 1 {
                    return;
                }
                t.row_heights.remove(ar as usize);
                t.cells.retain(|c| c.row != ar || c.row_span > 1);
                for c in t.cells.iter_mut() {
                    if c.row > ar {
                        c.row -= 1;
                    } else if (c.row == ar && c.row_span > 1)
                        || (c.row < ar && c.row + c.row_span > ar)
                    {
                        c.row_span -= 1;
                    }
                }
                self.table_edit = None;
            }
            TableOp::DeleteCol => {
                if t.cols() <= 1 {
                    return;
                }
                t.col_widths.remove(ac as usize);
                t.cells.retain(|c| c.col != ac || c.col_span > 1);
                for c in t.cells.iter_mut() {
                    if c.col > ac {
                        c.col -= 1;
                    } else if (c.col == ac && c.col_span > 1)
                        || (c.col < ac && c.col + c.col_span > ac)
                    {
                        c.col_span -= 1;
                    }
                }
                self.table_edit = None;
            }
            TableOp::DeleteTable => {
                self.table_edit = None;
                self.run(Command::DeleteShapes { shapes: vec![id] });
                self.selection.clear();
                return;
            }
            TableOp::DistributeRows => {
                let h = t.rect.height() / t.rows() as f64;
                for r in t.row_heights.iter_mut() {
                    *r = h;
                }
            }
            TableOp::DistributeCols => {
                let w = t.rect.width() / t.cols() as f64;
                for c in t.col_widths.iter_mut() {
                    *c = w;
                }
            }
            TableOp::Merge => {
                if sel.len() < 2 {
                    return;
                }
                let r0 = sel.iter().map(|s| s.0).min().unwrap_or(0);
                let r1 = sel.iter().map(|s| s.0).max().unwrap_or(0);
                let c0 = sel.iter().map(|s| s.1).min().unwrap_or(0);
                let c1 = sel.iter().map(|s| s.1).max().unwrap_or(0);
                // Text of all merged cells joined by spaces.
                let mut text: Vec<TextSpan> = Vec::new();
                for c in &t.cells {
                    if c.row >= r0 && c.row <= r1 && c.col >= c0 && c.col <= c1 {
                        text.extend(c.text.iter().cloned());
                    }
                }
                t.cells
                    .retain(|c| !(c.row >= r0 && c.row <= r1 && c.col >= c0 && c.col <= c1));
                let mut merged = blank(r0, c0);
                merged.row_span = r1 - r0 + 1;
                merged.col_span = c1 - c0 + 1;
                merged.text = text;
                t.cells.push(merged);
                self.table_edit = Some(TableEdit::new(id, vec![(r0, c0)], 0));
            }
            TableOp::Unmerge => {
                let Some(pos) = t.cells.iter().position(|c| c.row == ar && c.col == ac) else {
                    return;
                };
                let cell = t.cells.remove(pos);
                for r in cell.row..cell.row + cell.row_span {
                    for c in cell.col..cell.col + cell.col_span {
                        let mut b = blank(r, c);
                        if r == cell.row && c == cell.col {
                            b.text = cell.text.clone();
                            b.fill = cell.fill.clone();
                        }
                        t.cells.push(b);
                    }
                }
            }
            TableOp::SplitRows => {
                // Split the active row in two equal rows.
                let h = t.row_heights.get(ar as usize).copied().unwrap_or(10.0) / 2.0;
                if let Some(x) = t.row_heights.get_mut(ar as usize) {
                    *x = h;
                }
                t.row_heights.insert(ar as usize + 1, h);
                for c in t.cells.iter_mut() {
                    if c.row > ar {
                        c.row += 1;
                    } else if c.row + c.row_span > ar + 1 {
                        c.row_span += 1;
                    }
                }
                for col in 0..t.cols() {
                    if !t.covered(ar + 1, col)
                        && !t.cells.iter().any(|c| c.row == ar + 1 && c.col == col)
                    {
                        t.cells.push(blank(ar + 1, col));
                    }
                }
            }
            TableOp::SplitCols => {
                let w = t.col_widths.get(ac as usize).copied().unwrap_or(20.0) / 2.0;
                if let Some(x) = t.col_widths.get_mut(ac as usize) {
                    *x = w;
                }
                t.col_widths.insert(ac as usize + 1, w);
                for c in t.cells.iter_mut() {
                    if c.col > ac {
                        c.col += 1;
                    } else if c.col + c.col_span > ac + 1 {
                        c.col_span += 1;
                    }
                }
                for row in 0..t.rows() {
                    if !t.covered(row, ac + 1)
                        && !t.cells.iter().any(|c| c.row == row && c.col == ac + 1)
                    {
                        t.cells.push(blank(row, ac + 1));
                    }
                }
            }
        }
        self.set_table(id, t);
    }

    /// Table > Convert Text to Table: tabs separate columns, newlines rows.
    pub fn convert_text_to_table(&mut self) {
        let Some(s) = self
            .selected_shapes()
            .into_iter()
            .find(|s| matches!(s.kind, ShapeKind::Text { .. }))
        else {
            return;
        };
        let ShapeKind::Text { spans, .. } = &s.kind else {
            return;
        };
        let text: String = spans.iter().map(|x| x.text.as_str()).collect();
        let style = spans
            .first()
            .cloned()
            .unwrap_or_else(|| TextSpan::new("", "Arial", 12.0));
        let rows: Vec<Vec<&str>> = text.lines().map(|l| l.split('\t').collect()).collect();
        let nrows = rows.len().max(1) as u32;
        let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(1).max(1) as u32;
        let b = s.bounds();
        let rect = Rect::new(
            b.x0,
            b.y1 - nrows as f64 * 10.0,
            b.x0 + ncols as f64 * 40.0,
            b.y1,
        );
        let mut t = Table::new(
            rect,
            nrows,
            ncols,
            Some(tracedraw_core::Stroke::hairline(
                tracedraw_core::Color::BLACK,
            )),
        );
        for (r, row) in rows.iter().enumerate() {
            for (c, txt) in row.iter().enumerate() {
                if let Some(cell) = t
                    .cells
                    .iter_mut()
                    .find(|cell| cell.row == r as u32 && cell.col == c as u32)
                {
                    cell.text = vec![TextSpan {
                        text: txt.to_string(),
                        ..style.clone()
                    }];
                }
            }
        }
        let id = s.id;
        self.run(Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Table(t),
        });
    }

    /// Table > Convert Table to Text: cells joined by tabs, rows by newlines.
    pub fn convert_table_to_text(&mut self) {
        let Some((id, t)) = self.selected_table() else {
            return;
        };
        let mut lines = Vec::new();
        for r in 0..t.rows() {
            let mut cols = Vec::new();
            for c in 0..t.cols() {
                cols.push(cell_text(&t, r, c));
            }
            lines.push(cols.join("\t"));
        }
        let style = t
            .cells
            .iter()
            .find_map(|c| c.text.first().cloned())
            .unwrap_or_else(|| TextSpan::new("", self.text_font.clone(), self.text_size_pt));
        let span = TextSpan {
            text: lines.join("\n"),
            ..style
        };
        self.table_edit = None;
        self.run(Command::SetShapeKind {
            shape: id,
            kind: ShapeKind::Text {
                spans: vec![span],
                origin: Point::new(t.rect.x0, t.rect.y0),
                frame: Some(tracedraw_core::geometry::Size::new(
                    t.rect.width(),
                    t.rect.height(),
                )),
                align: tracedraw_core::TextAlign::Left,
                para: Default::default(),
                on_path: None,
            },
        });
    }
}

fn blank(row: u32, col: u32) -> TableCell {
    TableCell {
        row,
        col,
        row_span: 1,
        col_span: 1,
        text: Vec::new(),
        fill: None,
        align: tracedraw_core::TextAlign::Left,
    }
}

/// Grow the cell's row so its text fits (rows grow downwards and never
/// shrink while typing). Cells
/// spanning several rows grow their last row.
fn grow_row_to_fit(t: &mut Table, r: u32, c: u32) {
    let Some(cell) = t.cells.iter().find(|cell| cell.row == r && cell.col == c) else {
        return;
    };
    if cell.text.is_empty() {
        return;
    }
    let inner = t.cell_rect(cell).inset(-t.padding);
    let para = tracedraw_core::ParagraphStyle::default();
    let layout =
        tracedraw_text::fonts().layout(&tracedraw_core::document::text_outline::TextRequest {
            spans: &cell.text,
            frame: Some(tracedraw_core::geometry::Size::new(
                inner.width().max(0.1),
                1.0e6,
            )),
            align: cell.align,
            para: &para,
            on_path: None,
        });
    let needed = -layout.bounds.y0 + 2.0 * t.padding + 0.01;
    let have = t.cell_rect(cell).height();
    let last = (cell.row + cell.row_span).saturating_sub(1) as usize;
    if needed.is_finite() && needed > have + 1e-6 {
        if let Some(h) = t.row_heights.get_mut(last) {
            *h += needed - have;
        }
    }
}

fn cell_text(t: &Table, r: u32, c: u32) -> String {
    t.cells
        .iter()
        .find(|cell| cell.row == r && cell.col == c)
        .map(|cell| cell.text.iter().map(|s| s.text.as_str()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn new_tables_take_the_bar_defaults() {
        use tracedraw_core::{Color, Fill, Stroke};
        let mut app = App::headless();
        app.table_rows = 2;
        app.table_cols = 5;
        app.table_fill = Fill::Solid(Color::rgb8(0, 10, 20));
        let mut border = Stroke::hairline(Color::rgb8(9, 9, 9));
        border.width = 0.5;
        app.table_border = Some(border.clone());
        app.create_table(Rect::new(0.0, 0.0, 100.0, 40.0));
        let (_, t) = app.selected_table().expect("table selected");
        assert_eq!((t.rows(), t.cols()), (2, 5));
        assert_eq!(t.cell_fill, Fill::Solid(Color::rgb8(0, 10, 20)));
        assert_eq!(t.border, Some(border));
        assert!((t.rect.width() - 100.0).abs() < 1e-9);
    }

    use super::*;
    use tracedraw_core::geometry::Rect;

    #[test]
    fn click_edits_cell_and_tab_moves_on() {
        let mut app = App::headless();
        let t = Table::new(Rect::new(10.0, 10.0, 130.0, 40.0), 3, 4, None);
        let id = app.new_shape(ShapeKind::Table(t)).unwrap();
        app.select(vec![id]);
        // Top-left cell: x in [10,40), y in [30,40].
        app.table_click(id, Point::new(12.0, 38.0), false);
        assert_eq!(app.table_edit.as_ref().unwrap().cells, vec![(0, 0)]);
        app.cell_insert("Hello");
        app.table_next_cell(false);
        assert_eq!(app.table_edit.as_ref().unwrap().cells, vec![(0, 1)]);
        app.cell_insert("World");
        let (_, t) = app.selected_table().unwrap();
        assert_eq!(cell_text(&t, 0, 0), "Hello");
        assert_eq!(cell_text(&t, 0, 1), "World");
        // Caret editing inside the cell: Home, Delete, End, Backspace,
        // Shift+Left selection replaced by typing, Enter for a new line.
        use crate::text_editing::CaretMove;
        app.cell_move(CaretMove::LineStart, false);
        app.cell_delete_forward();
        app.cell_move(CaretMove::LineEnd, false);
        app.cell_delete_backward();
        assert_eq!(app.cell_text(), "orl");
        app.cell_move(CaretMove::Left, true);
        app.cell_move(CaretMove::Left, true);
        assert_eq!(app.cell_selected_text(), "rl");
        app.cell_insert("k\nsecond");
        assert_eq!(app.cell_text(), "ok\nsecond");
        app.cell_move(CaretMove::Up, false);
        assert_eq!(app.table_edit.as_ref().unwrap().caret, 2);
        // Left at the start of the cell steps to the previous cell's end.
        app.cell_move(CaretMove::LineStart, false);
        app.cell_move(CaretMove::Left, false);
        assert_eq!(app.table_edit.as_ref().unwrap().cells, vec![(0, 0)]);
        assert_eq!(app.table_edit.as_ref().unwrap().caret, 5);
        // Bold on a selection splits the cell's spans.
        app.cell_select_all();
        app.cell_move(CaretMove::Left, true);
        app.cell_apply_style(|s| s.bold = true);
        let spans = app.cell_spans().unwrap();
        assert_eq!(spans.len(), 2);
        assert!(spans[0].bold && !spans[1].bold);
        assert_eq!(spans[0].text, "Hell");
        // Down from the last line goes to the cell below.
        app.cell_move(CaretMove::Down, false);
        assert_eq!(app.table_edit.as_ref().unwrap().cells, vec![(1, 0)]);
        // The caret has geometry even in an empty cell.
        assert!(app.cell_caret_geometry().is_some());
        // Several lines make the row taller so nothing overlaps.
        let (_, before) = app.selected_table().unwrap();
        let h0 = before.row_heights[1];
        app.cell_insert("a\nb\nc\nd\ne\nf");
        let (_, after) = app.selected_table().unwrap();
        if !tracedraw_text::fonts().families().is_empty() {
            assert!(after.row_heights[1] > h0, "{h0} -> {:?}", after.row_heights);
            assert!(
                (after.rect.y1 - before.rect.y1).abs() < 1e-9,
                "the top stays"
            );
        }
        let c00 = t.cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        let r = t.cell_rect(c00);
        assert!((r.x0 - 10.0).abs() < 1e-9 && (r.x1 - 40.0).abs() < 1e-9);
        assert!((r.y0 - 30.0).abs() < 1e-9 && (r.y1 - 40.0).abs() < 1e-9);
        assert_eq!(t.rect, Rect::new(10.0, 10.0, 130.0, 40.0));
    }
}
