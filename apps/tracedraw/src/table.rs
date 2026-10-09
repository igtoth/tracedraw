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
/// text being typed into the active cell.
#[derive(Debug, Clone, PartialEq)]
pub struct TableEdit {
    pub shape: ShapeId,
    /// Selected (row, col) pairs; the first one is the active cell.
    pub cells: Vec<(u32, u32)>,
    pub text: String,
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
        let text: String = t.cells[ci].text.iter().map(|s| s.text.as_str()).collect();
        match &mut self.table_edit {
            Some(e) if e.shape == id && extend => {
                if !e.cells.contains(&rc) {
                    e.cells.push(rc);
                }
            }
            _ => {
                self.table_edit = Some(TableEdit {
                    shape: id,
                    cells: vec![rc],
                    text,
                });
            }
        }
    }

    /// Tab inside a table: commit and move to the next cell (wrapping rows).
    pub fn table_next_cell(&mut self, backwards: bool) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        self.table_commit_text();
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
        let text = cell_text(t, nr, nc);
        self.table_edit = Some(TableEdit {
            shape: e.shape,
            cells: vec![(nr, nc)],
            text,
        });
    }

    /// Commit typed text into the active cell.
    pub fn table_commit_text(&mut self) {
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
            let existing = cell.text.first().cloned();
            let mut span = existing.unwrap_or_else(|| {
                TextSpan::new(
                    "",
                    self.text_font.clone(),
                    (self.text_size_pt / 2.0).max(8.0),
                )
            });
            span.text = e.text.clone();
            cell.text = if e.text.is_empty() {
                Vec::new()
            } else {
                vec![span]
            };
        }
        self.set_table(e.shape, t);
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
                self.table_edit = Some(TableEdit {
                    shape: id,
                    cells: vec![(ar, ac)],
                    text: cell_text(&t, ar, ac),
                });
                return;
            }
            TableOp::SelectRow => {
                let cells: Vec<(u32, u32)> = (0..t.cols()).map(|c| (ar, c)).collect();
                self.table_edit = Some(TableEdit {
                    shape: id,
                    cells,
                    text: cell_text(&t, ar, ac),
                });
                return;
            }
            TableOp::SelectCol => {
                let cells: Vec<(u32, u32)> = (0..t.rows()).map(|r| (r, ac)).collect();
                self.table_edit = Some(TableEdit {
                    shape: id,
                    cells,
                    text: cell_text(&t, ar, ac),
                });
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
                    } else if c.row == ar && c.row_span > 1 {
                        c.row_span -= 1;
                    } else if c.row < ar && c.row + c.row_span > ar {
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
                    } else if c.col == ac && c.col_span > 1 {
                        c.col_span -= 1;
                    } else if c.col < ac && c.col + c.col_span > ac {
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
                self.table_edit = Some(TableEdit {
                    shape: id,
                    cells: vec![(r0, c0)],
                    text: String::new(),
                });
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

fn cell_text(t: &Table, r: u32, c: u32) -> String {
    t.cells
        .iter()
        .find(|cell| cell.row == r && cell.col == c)
        .map(|cell| cell.text.iter().map(|s| s.text.as_str()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    #[test]
    fn click_edits_cell_and_tab_moves_on() {
        let mut app = App::headless();
        let t = Table::new(Rect::new(10.0, 10.0, 50.0, 40.0), 3, 4, None);
        let id = app.new_shape(ShapeKind::Table(t)).unwrap();
        app.select(vec![id]);
        // Top-left cell: x in [10,20), y in [30,40].
        app.table_click(id, Point::new(12.0, 38.0), false);
        assert_eq!(app.table_edit.as_ref().unwrap().cells, vec![(0, 0)]);
        app.table_edit.as_mut().unwrap().text = "Hello".into();
        app.table_commit_text();
        app.table_next_cell(false);
        assert_eq!(app.table_edit.as_ref().unwrap().cells, vec![(0, 1)]);
        app.table_edit.as_mut().unwrap().text = "World".into();
        app.table_commit_text();
        let (_, t) = app.selected_table().unwrap();
        assert_eq!(cell_text(&t, 0, 0), "Hello");
        assert_eq!(cell_text(&t, 0, 1), "World");
        let c00 = t.cells.iter().find(|c| c.row == 0 && c.col == 0).unwrap();
        let r = t.cell_rect(c00);
        assert!((r.x0 - 10.0).abs() < 1e-9 && (r.x1 - 20.0).abs() < 1e-9);
        assert!((r.y0 - 30.0).abs() < 1e-9 && (r.y1 - 40.0).abs() < 1e-9);
        assert_eq!(t.rect, Rect::new(10.0, 10.0, 50.0, 40.0));
    }
}
