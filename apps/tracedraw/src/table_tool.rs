//! The Table tool's pointer work and the cell-selection edits: dragging
//! across cells, Ctrl+click and Shift+click, the row and column pointers
//! at the table's left and top edges, dragging the dividing lines to
//! resize, dragging selected rows or columns to move them, and the edits
//! that act on the selected cells (size, fill, borders, text style,
//! inserting several rows or columns, splitting into N). Each edit is one
//! `SetShapeKind` command, so it undoes in one step.

use crate::app::App;
use crate::table::TableEdit;
use crate::tools::Tool;
use egui::{PointerButton, Response};
use serde::{Deserialize, Serialize};
use tracedraw_core::{
    document::ShapeKind,
    geometry::{Affine, Point},
    BorderSet, Command, EdgeBorder, Fill, ShapeId, Stroke, Table, TableHit, TextSpan,
};

/// Reach of the table's lines and edge pointers, screen points.
const REACH_PX: f64 = 4.0;

/// Options > Tools > Table: what Tab does in a cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct TablePrefs {
    /// Tab types a tab character into the cell (Ctrl+Tab then moves to
    /// the next cell); off: Tab moves to the next cell.
    pub tab_inserts_tab: bool,
    /// Tab goes down the columns (top to bottom, then the next column)
    /// rather than along the rows.
    pub tab_by_columns: bool,
}

/// What a Table tool drag is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TableDragKind {
    /// Selecting the block of cells from this cell to the pointer.
    Select { start: (u32, u32) },
    /// Moving the vertical line `k` (1..=cols).
    ColLine(u32),
    /// Moving the horizontal line `k` (1..=rows).
    RowLine(u32),
    /// Moving the selected rows `first..first + count`.
    MoveRows { first: u32, count: u32 },
    /// Moving the selected columns.
    MoveCols { first: u32, count: u32 },
}

/// A Table tool drag in table-local millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableDrag {
    pub shape: ShapeId,
    pub kind: TableDragKind,
    pub start: Point,
    pub current: Point,
}

/// The Table tool's state besides the cell selection.
#[derive(Debug, Clone, Default)]
pub struct TableUi {
    pub drag: Option<TableDrag>,
    /// Which borders of the selected cells the bar's border fields set.
    pub border_set: BorderSet,
    /// The bar's margins are given per side (else one value for all).
    pub margins_separate: bool,
}

/// Border styles offered on the bar: solid, dashed, dotted.
pub const BORDER_DASHES: [(&str, &[f64]); 3] = [
    ("table.border_solid", &[]),
    ("table.border_dashed", &[4.0, 2.0]),
    ("table.border_dotted", &[1.0, 1.5]),
];

impl App {
    /// The table being edited with the Table tool, else the selected one,
    /// with its transform.
    pub fn table_target(&self) -> Option<(ShapeId, Affine, Table)> {
        let id = match &self.table_edit {
            Some(e) => e.shape,
            None => self.selected_table()?.0,
        };
        let (_, s) = self.doc().shape(id).ok()?;
        match &s.kind {
            ShapeKind::Table(t) => Some((id, s.transform, t.clone())),
            _ => None,
        }
    }

    /// The selected cells of table `id`, or every cell when none are.
    pub fn table_selected_cells(&self, id: ShapeId, t: &Table) -> Vec<(u32, u32)> {
        match &self.table_edit {
            Some(e) if e.shape == id && !e.cells.is_empty() => e.cells.clone(),
            _ => t.all_cells(),
        }
    }

    /// Store an edited table as one labelled undo step.
    pub fn set_table_as(&mut self, id: ShapeId, mut t: Table, label: &'static str) {
        t.refit();
        if let Err(e) = self.engine.run_with_label(
            &Command::SetShapeKind {
                shape: id,
                kind: ShapeKind::Table(t),
            },
            label,
        ) {
            self.status = e.to_string();
        }
    }

    fn table_of(&self, id: ShapeId) -> Option<Table> {
        let (_, s) = self.doc().shape(id).ok()?;
        match &s.kind {
            ShapeKind::Table(t) => Some(t.clone()),
            _ => None,
        }
    }

    /// Select cells of table `id`; the first one is the active cell.
    fn set_table_cells(&mut self, id: ShapeId, cells: Vec<(u32, u32)>) {
        let len = self
            .table_of(id)
            .and_then(|t| {
                let (r, c) = cells.first().copied()?;
                let i = t.cell_index(r, c)?;
                Some(tracedraw_core::spans_char_count(&t.cells.get(i)?.text))
            })
            .unwrap_or(0);
        self.table_edit = Some(TableEdit::new(id, cells, len));
    }

    /// Anchor of the cell under a local point, or of the cell nearest to it
    /// inside the table (a drag beyond an edge takes the edge cells).
    fn cell_near(t: &Table, local: Point) -> Option<(u32, u32)> {
        let r = t.rect;
        let x = local.x.clamp(r.x0 + 1e-6, r.x1 - 1e-6);
        let y = local.y.clamp(r.y0 + 1e-6, r.y1 - 1e-6);
        let i = t.cell_at(Point::new(x, y))?;
        t.cells.get(i).map(|c| (c.row, c.col))
    }

    /// Ctrl+click: add the cell under the point to the selection, or take
    /// it out (the last cell stays).
    pub fn table_toggle_cell(&mut self, id: ShapeId, local: Point) {
        let Some(t) = self.table_of(id) else {
            return;
        };
        let Some(rc) = t
            .cell_at(local)
            .and_then(|i| t.cells.get(i))
            .map(|c| (c.row, c.col))
        else {
            return;
        };
        let mut cells = match &self.table_edit {
            Some(e) if e.shape == id => e.cells.clone(),
            _ => Vec::new(),
        };
        if let Some(pos) = cells.iter().position(|c| *c == rc) {
            if cells.len() > 1 {
                cells.remove(pos);
            }
        } else {
            cells.push(rc);
        }
        self.set_table_cells(id, cells);
    }

    /// Shift+click: the block from the active cell to the cell under the
    /// point.
    pub fn table_range_to(&mut self, id: ShapeId, local: Point) {
        let Some(t) = self.table_of(id) else {
            return;
        };
        let Some(to) = Self::cell_near(&t, local) else {
            return;
        };
        let from = match &self.table_edit {
            Some(e) if e.shape == id => e.cells.first().copied().unwrap_or(to),
            _ => to,
        };
        let mut cells = t.block_cells(from, to);
        // The active cell stays first.
        if let Some(pos) = cells.iter().position(|c| *c == from) {
            cells.swap(0, pos);
        }
        self.set_table_cells(id, cells);
    }

    /// A click on the corner or on a row or column pointer: every cell, the
    /// row or the column (with `extend`, the rows or columns from the
    /// active one to it).
    pub fn table_header_select(&mut self, id: ShapeId, hit: TableHit, extend: bool) {
        let Some(t) = self.table_of(id) else {
            return;
        };
        let (rows, cols) = (t.rows(), t.cols());
        if rows == 0 || cols == 0 {
            return;
        }
        let active = match &self.table_edit {
            Some(e) if e.shape == id && extend => e.cells.first().copied(),
            _ => None,
        };
        let cells = match hit {
            TableHit::Corner => t.all_cells(),
            TableHit::RowHeader(r) => {
                let from = active.map_or(r, |a| a.0);
                t.block_cells((from.min(r), 0), (from.max(r), cols - 1))
            }
            TableHit::ColHeader(c) => {
                let from = active.map_or(c, |a| a.1);
                t.block_cells((0, from.min(c)), (rows - 1, from.max(c)))
            }
            TableHit::ColLine(_) | TableHit::RowLine(_) => return,
        };
        self.set_table_cells(id, cells);
    }

    /// The selected cells make whole rows: their first row and count.
    pub fn table_selected_rows(&self, id: ShapeId, t: &Table) -> Option<(u32, u32)> {
        let e = self.table_edit.as_ref().filter(|e| e.shape == id)?;
        let (r0, r1, c0, c1) = t.span_of(&e.cells)?;
        if c0 != 0 || c1 + 1 != t.cols() || (r0 == 0 && r1 + 1 == t.rows()) {
            return None;
        }
        let mut want = t.block_cells((r0, 0), (r1, t.cols() - 1));
        let mut have = e.cells.clone();
        want.sort_unstable();
        have.sort_unstable();
        (want == have).then_some((r0, r1 - r0 + 1))
    }

    /// The selected cells make whole columns: their first column and count.
    pub fn table_selected_cols(&self, id: ShapeId, t: &Table) -> Option<(u32, u32)> {
        let e = self.table_edit.as_ref().filter(|e| e.shape == id)?;
        let (r0, r1, c0, c1) = t.span_of(&e.cells)?;
        if r0 != 0 || r1 + 1 != t.rows() || (c0 == 0 && c1 + 1 == t.cols()) {
            return None;
        }
        let mut want = t.block_cells((0, c0), (t.rows() - 1, c1));
        let mut have = e.cells.clone();
        want.sort_unstable();
        have.sort_unstable();
        (want == have).then_some((c0, c1 - c0 + 1))
    }

    /// Start a Table tool drag at a local point of table `id`: on a
    /// dividing line it resizes, inside selected whole rows or columns it
    /// moves them, elsewhere in the table it selects cells. `tol` is the
    /// lines' reach in mm. Returns false when the drag is not the table's.
    pub fn table_drag_begin(&mut self, id: ShapeId, local: Point, tol: f64) -> bool {
        let Some(t) = self.table_of(id) else {
            return false;
        };
        let kind = match t.hit(local, tol) {
            Some(TableHit::ColLine(k)) => TableDragKind::ColLine(k),
            Some(TableHit::RowLine(k)) => TableDragKind::RowLine(k),
            Some(_) => return false,
            None => {
                let Some(rc) = t
                    .cell_at(local)
                    .and_then(|i| t.cells.get(i))
                    .map(|c| (c.row, c.col))
                else {
                    return false;
                };
                let in_selection = self
                    .table_edit
                    .as_ref()
                    .is_some_and(|e| e.shape == id && e.cells.contains(&rc));
                match (
                    in_selection,
                    self.table_selected_rows(id, &t),
                    self.table_selected_cols(id, &t),
                ) {
                    (true, Some((first, count)), _) => TableDragKind::MoveRows { first, count },
                    (true, _, Some((first, count))) => TableDragKind::MoveCols { first, count },
                    _ => {
                        self.set_table_cells(id, vec![rc]);
                        TableDragKind::Select { start: rc }
                    }
                }
            }
        };
        self.table_ui.drag = Some(TableDrag {
            shape: id,
            kind,
            start: local,
            current: local,
        });
        true
    }

    /// The drag went on to a local point.
    pub fn table_drag_to(&mut self, local: Point) {
        let Some(d) = self.table_ui.drag.as_mut() else {
            return;
        };
        d.current = local;
        let d = *d;
        if let TableDragKind::Select { start } = d.kind {
            let Some(t) = self.table_of(d.shape) else {
                return;
            };
            if let Some(to) = Self::cell_near(&t, local) {
                let mut cells = t.block_cells(start, to);
                if let Some(pos) = cells.iter().position(|c| *c == start) {
                    cells.swap(0, pos);
                }
                if let Some(e) = self.table_edit.as_mut().filter(|e| e.shape == d.shape) {
                    e.cells = cells;
                } else {
                    self.set_table_cells(d.shape, cells);
                }
            }
        }
    }

    /// Where moved rows or columns would go: the line nearest the pointer.
    pub fn table_drop_line(t: &Table, kind: TableDragKind, p: Point) -> Option<u32> {
        match kind {
            TableDragKind::MoveRows { .. } => (0..=t.rows()).min_by(|a, b| {
                let da = (t.row_top(*a) - p.y).abs();
                let db = (t.row_top(*b) - p.y).abs();
                da.total_cmp(&db)
            }),
            TableDragKind::MoveCols { .. } => (0..=t.cols()).min_by(|a, b| {
                let da = (t.col_x(*a) - p.x).abs();
                let db = (t.col_x(*b) - p.x).abs();
                da.total_cmp(&db)
            }),
            _ => None,
        }
    }

    /// The table as the drag in progress would leave it (resizes only).
    pub fn table_drag_result(&self, d: &TableDrag) -> Option<Table> {
        let mut t = self.table_of(d.shape)?;
        match d.kind {
            TableDragKind::ColLine(k) => t.drag_col_line(k, d.current.x - d.start.x),
            TableDragKind::RowLine(k) => t.drag_row_line(k, d.start.y - d.current.y),
            _ => return None,
        }
        t.refit();
        Some(t)
    }

    /// Release: resize, move the rows or columns, or keep the selection.
    pub fn table_drag_end(&mut self) {
        let Some(d) = self.table_ui.drag.take() else {
            return;
        };
        match d.kind {
            TableDragKind::Select { .. } => {}
            TableDragKind::ColLine(_) | TableDragKind::RowLine(_) => {
                if (d.current - d.start).hypot() < 1e-9 {
                    return;
                }
                if let Some(t) = self.table_drag_result(&d) {
                    self.set_table_as(d.shape, t, "Resize Table");
                }
            }
            TableDragKind::MoveRows { first, count } | TableDragKind::MoveCols { first, count } => {
                let Some(mut t) = self.table_of(d.shape) else {
                    return;
                };
                let Some(to) = Self::table_drop_line(&t, d.kind, d.current) else {
                    return;
                };
                if to >= first && to <= first + count {
                    return;
                }
                let rows = matches!(d.kind, TableDragKind::MoveRows { .. });
                let moved = if rows {
                    t.move_rows(first, count, to)
                } else {
                    t.move_cols(first, count, to)
                };
                if !moved {
                    self.status = crate::i18n::tr("table.move_blocked");
                    return;
                }
                let new_first = if to > first { to - count } else { to };
                let cells = if rows {
                    t.block_cells((new_first, 0), (new_first + count - 1, t.cols() - 1))
                } else {
                    t.block_cells((0, new_first), (t.rows() - 1, new_first + count - 1))
                };
                self.set_table_as(d.shape, t, if rows { "Move Rows" } else { "Move Columns" });
                self.set_table_cells(d.shape, cells);
            }
        }
    }

    /// The Table tool on the canvas (`p` in page space).
    pub fn table_tool_input(&mut self, response: &Response, p: Point, mods: egui::Modifiers) {
        // A release the canvas did not see ends the drag.
        let primary_down = response.ctx.input(|i| i.pointer.primary_down());
        if self.table_ui.drag.is_some()
            && !primary_down
            && !response.drag_stopped_by(PointerButton::Primary)
        {
            self.table_drag_end();
        }
        let tol = REACH_PX / self.view.zoom.max(1e-3) as f64;
        let target = self.table_target();
        let hit = target.as_ref().and_then(|(id, tf, t)| {
            let local = tf.inverse() * p;
            t.hit(local, tol / scale_of(tf)).map(|h| (*id, h))
        });
        let over_table = self.hit_test(p).and_then(|id| {
            let (_, s) = self.doc().shape(id).ok()?;
            matches!(s.kind, ShapeKind::Table(_)).then_some((id, s.transform))
        });
        if response.clicked_by(PointerButton::Primary) {
            if let Some((id, h)) = hit {
                if !matches!(h, TableHit::ColLine(_) | TableHit::RowLine(_)) {
                    self.table_header_select(id, h, mods.shift);
                    self.select(vec![id]);
                    return;
                }
            }
            if let Some((id, tf)) = over_table {
                let local = tf.inverse() * p;
                if mods.command || mods.ctrl {
                    self.table_toggle_cell(id, local);
                } else if mods.shift && self.table_edit.as_ref().is_some_and(|e| e.shape == id) {
                    self.table_range_to(id, local);
                } else {
                    self.table_click(id, local, false);
                }
                self.select(vec![id]);
                self.status = crate::i18n::tr("table.type_hint");
                return;
            }
            // Elsewhere a click selects, as with the other drawing tools.
            self.table_edit = None;
            self.click_select(p, mods);
            return;
        }
        if response.drag_started_by(PointerButton::Primary) {
            let start = hit.map(|(id, _)| id).or(over_table.map(|(id, _)| id));
            if let Some(id) = start {
                let tf = self
                    .doc()
                    .shape(id)
                    .map(|(_, s)| s.transform)
                    .unwrap_or(Affine::IDENTITY);
                if self.table_drag_begin(id, tf.inverse() * p, tol / scale_of(&tf)) {
                    self.select(vec![id]);
                }
                // A drag that starts on a table never draws a new one.
                return;
            }
        }
        if let Some(d) = self.table_ui.drag {
            let tf = self
                .doc()
                .shape(d.shape)
                .map(|(_, s)| s.transform)
                .unwrap_or(Affine::IDENTITY);
            if response.dragged_by(PointerButton::Primary) {
                self.table_drag_to(tf.inverse() * p);
            }
            if response.drag_stopped_by(PointerButton::Primary) {
                self.table_drag_to(tf.inverse() * p);
                self.table_drag_end();
            }
            return;
        }
        self.box_like_input(response, p, mods);
    }

    /// The cursor of the Table tool at a screen point: resize arrows over a
    /// dividing line, an arrow into the table over the row and column
    /// pointers, the I-beam over a cell, else the tool's cross hair.
    pub fn table_cursor(&self, pos: Option<egui::Pos2>) -> crate::ui::cursors::ToolCursor {
        use crate::ui::cursors::ToolCursor;
        if let Some(d) = &self.table_ui.drag {
            return match d.kind {
                TableDragKind::ColLine(_) => ToolCursor::System(egui::CursorIcon::ResizeColumn),
                TableDragKind::RowLine(_) => ToolCursor::System(egui::CursorIcon::ResizeRow),
                TableDragKind::MoveRows { .. } | TableDragKind::MoveCols { .. } => {
                    ToolCursor::System(egui::CursorIcon::Grabbing)
                }
                TableDragKind::Select { .. } => ToolCursor::System(egui::CursorIcon::Cell),
            };
        }
        let Some(pos) = pos else {
            return ToolCursor::Badge(Tool::Table);
        };
        let p = self.view.to_page(pos);
        let tol = REACH_PX / self.view.zoom.max(1e-3) as f64;
        if let Some((_, tf, t)) = self.table_target() {
            match t.hit(tf.inverse() * p, tol / scale_of(&tf)) {
                Some(TableHit::ColLine(_)) => {
                    return ToolCursor::System(egui::CursorIcon::ResizeColumn)
                }
                Some(TableHit::RowLine(_)) => {
                    return ToolCursor::System(egui::CursorIcon::ResizeRow)
                }
                Some(TableHit::RowHeader(_)) => return ToolCursor::TableArrow { down: false },
                Some(TableHit::ColHeader(_)) => return ToolCursor::TableArrow { down: true },
                Some(TableHit::Corner) => return ToolCursor::TableArrow { down: true },
                None => {}
            }
        }
        let over = self
            .hit_test(p)
            .and_then(|id| self.doc().find_shape(id))
            .is_some_and(|s| matches!(s.kind, ShapeKind::Table(_)));
        if over {
            ToolCursor::IBeam
        } else {
            ToolCursor::Badge(Tool::Table)
        }
    }

    // ----- Edits on the selected cells -----

    /// Insert `count` rows above (or below) the selected cells' rows.
    pub fn table_insert_rows(&mut self, count: u32, below: bool) {
        let Some((id, _, mut t)) = self.table_target() else {
            return;
        };
        let cells = self.table_selected_cells(id, &t);
        let Some((r0, r1, _, _)) = t.span_of(&cells) else {
            return;
        };
        let count = count.clamp(1, 1000);
        if below {
            t.insert_rows(r1 + 1, count, r1);
        } else {
            t.insert_rows(r0, count, r0);
            self.shift_table_selection(count as i64, 0);
        }
        self.set_table_as(id, t, "Insert Rows");
    }

    /// Insert `count` columns left (or right) of the selected cells.
    pub fn table_insert_cols(&mut self, count: u32, right: bool) {
        let Some((id, _, mut t)) = self.table_target() else {
            return;
        };
        let cells = self.table_selected_cells(id, &t);
        let Some((_, _, c0, c1)) = t.span_of(&cells) else {
            return;
        };
        let count = count.clamp(1, 1000);
        if right {
            t.insert_cols(c1 + 1, count, c1);
        } else {
            t.insert_cols(c0, count, c0);
            self.shift_table_selection(0, count as i64);
        }
        self.set_table_as(id, t, "Insert Columns");
    }

    /// How many rows and columns the selected cells cover (one each
    /// without a selection): what Insert takes by default.
    pub fn table_selection_extent(&self) -> (u32, u32) {
        let Some((id, _, t)) = self.table_target() else {
            return (1, 1);
        };
        if !self.table_edit.as_ref().is_some_and(|e| e.shape == id) {
            return (1, 1);
        }
        let cells = self.table_selected_cells(id, &t);
        t.span_of(&cells)
            .map_or((1, 1), |(r0, r1, c0, c1)| (r1 - r0 + 1, c1 - c0 + 1))
    }

    fn shift_table_selection(&mut self, dr: i64, dc: i64) {
        if let Some(e) = self.table_edit.as_mut() {
            for c in e.cells.iter_mut() {
                c.0 = (c.0 as i64 + dr).max(0) as u32;
                c.1 = (c.1 as i64 + dc).max(0) as u32;
            }
        }
    }

    /// Split the selected cells (those in the active cell's band) into
    /// `n` rows or columns.
    pub fn table_split(&mut self, rows: bool, n: u32) {
        let Some((id, _, mut t)) = self.table_target() else {
            return;
        };
        let cells = match &self.table_edit {
            Some(e) if e.shape == id && !e.cells.is_empty() => e.cells.clone(),
            _ => vec![(0, 0)],
        };
        let ok = if rows {
            t.split_rows(&cells, n)
        } else {
            t.split_cols(&cells, n)
        };
        if !ok {
            self.status = crate::i18n::tr("table.split_blocked");
            return;
        }
        self.set_table_as(id, t, if rows { "Split Rows" } else { "Split Columns" });
        if let Some(first) = cells.first().copied() {
            self.set_table_cells(id, vec![first]);
        }
    }

    /// Width of the columns and height of the rows the selected cells cover
    /// (the bar's fields): the first one's.
    pub fn table_cell_size(&self) -> Option<(f64, f64)> {
        let (id, _, t) = self.table_target()?;
        let cells = self.table_selected_cells(id, &t);
        let (r0, r1, c0, c1) = t.span_of(&cells)?;
        let w: f64 = t
            .col_widths
            .iter()
            .skip(c0 as usize)
            .take((c1 - c0 + 1) as usize)
            .sum();
        let h: f64 = t
            .row_heights
            .iter()
            .skip(r0 as usize)
            .take((r1 - r0 + 1) as usize)
            .sum();
        Some((w, h))
    }

    /// Give the selected cells' columns a total width of `width` and their
    /// rows a total height of `height` (shared out in proportion); the
    /// table grows or shrinks to the right and downwards.
    pub fn table_set_cell_size(&mut self, width: Option<f64>, height: Option<f64>) {
        let Some((id, _, mut t)) = self.table_target() else {
            return;
        };
        let cells = self.table_selected_cells(id, &t);
        let Some((r0, r1, c0, c1)) = t.span_of(&cells) else {
            return;
        };
        let scale = |sizes: &mut Vec<f64>, a: u32, b: u32, total: f64| {
            if !total.is_finite() || total <= 0.0 {
                return;
            }
            let n = (b - a + 1) as f64;
            let total = total.max(tracedraw_core::table::MIN_CELL * n);
            let old: f64 = sizes
                .iter()
                .skip(a as usize)
                .take((b - a + 1) as usize)
                .sum();
            for s in sizes.iter_mut().skip(a as usize).take((b - a + 1) as usize) {
                *s = if old > 0.0 {
                    *s * total / old
                } else {
                    total / n
                };
            }
        };
        if let Some(w) = width {
            scale(&mut t.col_widths, c0, c1, w);
        }
        if let Some(h) = height {
            scale(&mut t.row_heights, r0, r1, h);
        }
        self.set_table_as(id, t, "Cell Size");
    }

    /// Distribute the selected rows (or columns) evenly, or all of them
    /// when the selection covers only one.
    pub fn table_distribute(&mut self, rows: bool) {
        let Some((id, _, mut t)) = self.table_target() else {
            return;
        };
        let cells = self.table_selected_cells(id, &t);
        let (a, b) = match t.span_of(&cells) {
            Some((r0, r1, _, _)) if rows && r1 > r0 => (r0, r1),
            Some((_, _, c0, c1)) if !rows && c1 > c0 => (c0, c1),
            _ if rows => (0, t.rows().saturating_sub(1)),
            _ => (0, t.cols().saturating_sub(1)),
        };
        let sizes = if rows {
            &mut t.row_heights
        } else {
            &mut t.col_widths
        };
        let n = (b - a + 1) as usize;
        let sum: f64 = sizes.iter().skip(a as usize).take(n).sum();
        for s in sizes.iter_mut().skip(a as usize).take(n) {
            *s = sum / n as f64;
        }
        self.set_table_as(
            id,
            t,
            if rows {
                "Distribute Rows"
            } else {
                "Distribute Columns"
            },
        );
    }

    /// Background of the selected cells (`None`: the table's cell fill).
    /// Returns false when no cells are selected.
    pub fn table_set_cell_fill(&mut self, fill: Option<Fill>) -> bool {
        let Some(e) = self.table_edit.clone() else {
            return false;
        };
        let Some(mut t) = self.table_of(e.shape) else {
            return false;
        };
        let mut any = false;
        for (r, c) in &e.cells {
            if let Some(cell) = t.cell_index(*r, *c).and_then(|i| t.cells.get_mut(i)) {
                cell.fill = fill.clone();
                any = true;
            }
        }
        if any {
            self.set_table_as(e.shape, t, "Cell Fill");
        }
        any
    }

    /// The fill of the active cell (its own, else the table's).
    pub fn table_active_fill(&self) -> Option<Fill> {
        let e = self.table_edit.as_ref()?;
        let t = self.table_of(e.shape)?;
        let (r, c) = e.cells.first().copied()?;
        let cell = t.cells.get(t.cell_index(r, c)?)?;
        Some(cell.fill.clone().unwrap_or_else(|| t.cell_fill.clone()))
    }

    /// Set the chosen borders (`table_ui.border_set`) of the selected
    /// cells, or of every cell, to `border` (`None`: no border).
    pub fn table_apply_borders(&mut self, border: Option<Stroke>) {
        let Some((id, _, mut t)) = self.table_target() else {
            return;
        };
        let whole = !self.table_edit.as_ref().is_some_and(|e| e.shape == id);
        let set = self.table_ui.border_set;
        if whole && set == BorderSet::All {
            // The whole table: its border, and no cell keeps its own.
            t.border = border;
            for c in t.cells.iter_mut() {
                c.borders = Default::default();
            }
            self.set_table_as(id, t, "Cell Borders");
            return;
        }
        let cells = self.table_selected_cells(id, &t);
        let edge = match border {
            Some(s) => EdgeBorder::Stroke(Box::new(s)),
            None => EdgeBorder::None,
        };
        t.apply_borders(&cells, set, edge);
        self.set_table_as(id, t, "Cell Borders");
    }

    /// The border the bar shows: the active cell's first chosen side with a
    /// setting of its own, else the table's.
    pub fn table_shown_border(&self) -> Option<Stroke> {
        let (id, _, t) = self.table_target()?;
        let own = self
            .table_edit
            .as_ref()
            .filter(|e| e.shape == id)
            .and_then(|e| {
                let (r, c) = e.cells.first().copied()?;
                let cell = t.cells.get(t.cell_index(r, c)?)?;
                let b = &cell.borders;
                [&b.top, &b.left, &b.bottom, &b.right]
                    .into_iter()
                    .find(|s| !matches!(s, EdgeBorder::Table))
                    .cloned()
            });
        match own {
            Some(EdgeBorder::Stroke(s)) => Some(*s),
            Some(_) => None,
            None => t.border.clone(),
        }
    }

    /// Margins of the table's cells.
    pub fn table_set_margins(&mut self, m: tracedraw_core::CellMargins) {
        let Some((id, _, mut t)) = self.table_target() else {
            return;
        };
        let clean = |v: f64| {
            if v.is_finite() {
                v.clamp(0.0, 100.0)
            } else {
                0.0
            }
        };
        let m = tracedraw_core::CellMargins {
            left: clean(m.left),
            right: clean(m.right),
            top: clean(m.top),
            bottom: clean(m.bottom),
        };
        t.margins = Some(m);
        self.set_table_as(id, t, "Cell Margins");
    }

    /// Separated borders with this horizontal and vertical spacing, or
    /// shared borders (`None`).
    pub fn table_set_spacing(&mut self, spacing: Option<[f64; 2]>) {
        let Some((id, _, mut t)) = self.table_target() else {
            return;
        };
        t.spacing = spacing.map(|[h, v]| {
            let clean = |v: f64| {
                if v.is_finite() {
                    v.clamp(0.0, 100.0)
                } else {
                    0.0
                }
            };
            [clean(h), clean(v)]
        });
        self.set_table_as(id, t, "Cell Spacing");
    }

    /// Restyle the text of the selected cells: the selected characters of
    /// the one cell being typed in, else every character of every
    /// selected cell (one undo step).
    pub fn cells_apply_style(&mut self, f: impl Fn(&mut TextSpan)) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        if e.cells.len() <= 1 {
            self.cell_apply_style(f);
            return;
        }
        let Some(mut t) = self.table_of(e.shape) else {
            return;
        };
        for (r, c) in &e.cells {
            if let Some(cell) = t.cell_index(*r, *c).and_then(|i| t.cells.get_mut(i)) {
                cell.text.iter_mut().for_each(&f);
            }
        }
        self.set_table_as(e.shape, t, "Cell Text");
    }

    /// The bar's text style (font, size, bold, italic, underline) and
    /// alignment on the selected cells.
    pub fn cells_apply_text_style(&mut self) {
        let (font, size, bold, italic, underline, align) = (
            self.text_font.clone(),
            self.text_size_pt,
            self.text_bold,
            self.text_italic,
            self.text_underline,
            self.text_align,
        );
        self.cells_apply_style(move |sp| {
            sp.font_family = font.clone();
            sp.size_pt = size;
            sp.bold = bold;
            sp.italic = italic;
            sp.underline = underline;
        });
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let Some(mut t) = self.table_of(e.shape) else {
            return;
        };
        let mut changed = false;
        for (r, c) in &e.cells {
            if let Some(cell) = t.cell_index(*r, *c).and_then(|i| t.cells.get_mut(i)) {
                if cell.align != align {
                    cell.align = align;
                    changed = true;
                }
            }
        }
        if changed {
            self.set_table_as(e.shape, t, "Cell Text");
        }
    }

    /// The active cell's text as a text object, for the Text docker.
    pub fn active_cell_text_shape(&self) -> Option<tracedraw_core::Shape> {
        self.table_edit.as_ref()?;
        self.cell_text_shape()
    }

    /// Tab in a cell, by the Table tool options.
    pub fn table_tab(&mut self, backwards: bool, ctrl: bool) {
        let prefs = self.settings.table.clone();
        if prefs.tab_inserts_tab && !ctrl && !backwards {
            self.cell_insert("\t");
            return;
        }
        if prefs.tab_by_columns {
            self.table_next_cell_by_columns(backwards);
        } else {
            self.table_next_cell(backwards);
        }
    }

    /// Tab down the columns: the cell below, then the top of the next
    /// column (wrapping); covered cells are skipped.
    pub fn table_next_cell_by_columns(&mut self, backwards: bool) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let Some(t) = self.table_of(e.shape) else {
            return;
        };
        let Some((mut r, mut c)) = e.cells.first().copied() else {
            return;
        };
        let (rows, cols) = (t.rows(), t.cols());
        if rows == 0 || cols == 0 {
            return;
        }
        for _ in 0..(rows * cols) {
            if backwards {
                if r == 0 {
                    r = rows - 1;
                    c = if c == 0 { cols - 1 } else { c - 1 };
                } else {
                    r -= 1;
                }
            } else {
                r += 1;
                if r >= rows {
                    r = 0;
                    c = (c + 1) % cols;
                }
            }
            if !t.covered(r, c) {
                break;
            }
        }
        self.set_table_cells(e.shape, vec![(r, c)]);
    }

    /// Ctrl+A in a cell: its text; again (or in an empty cell), every cell
    /// of the table.
    pub fn table_select_all(&mut self) {
        let Some(e) = self.table_edit.clone() else {
            return;
        };
        let len = self.cell_text().chars().count();
        let (a, b) = e.selection();
        if len > 0 && !(a == 0 && b == len) {
            self.cell_select_all();
            return;
        }
        if let Some(t) = self.table_of(e.shape) {
            let mut cells = t.all_cells();
            if let Some(first) = e.cells.first() {
                if let Some(pos) = cells.iter().position(|c| c == first) {
                    cells.swap(0, pos);
                }
            }
            if let Some(ed) = self.table_edit.as_mut() {
                ed.cells = cells;
            }
        }
    }
}

/// Mean scale of a transform, so reaches in page millimetres become local
/// ones.
fn scale_of(t: &Affine) -> f64 {
    let c = t.as_coeffs();
    let det = (c[0] * c[3] - c[1] * c[2]).abs().sqrt();
    if det.is_finite() && det > 1e-9 {
        det
    } else {
        1.0
    }
}

/// The resize line or the drop line of a Table tool drag, on the canvas.
pub fn draw_table_drag(app: &App, painter: &egui::Painter) {
    let Some(d) = &app.table_ui.drag else {
        return;
    };
    let Ok((_, s)) = app.doc().shape(d.shape) else {
        return;
    };
    let ShapeKind::Table(t) = &s.kind else {
        return;
    };
    let view = &app.view;
    let stroke = egui::Stroke::new(2.0, crate::theme::Tokens::SELECTION);
    let line = |a: Point, b: Point| {
        painter.line_segment(
            [
                view.to_screen(s.transform * a),
                view.to_screen(s.transform * b),
            ],
            stroke,
        );
    };
    let r = t.rect;
    match d.kind {
        TableDragKind::ColLine(k) => {
            if let Some(nt) = app.table_drag_result(d) {
                let x = nt.col_x(k);
                line(Point::new(x, nt.rect.y0), Point::new(x, nt.rect.y1));
            }
        }
        TableDragKind::RowLine(k) => {
            if let Some(nt) = app.table_drag_result(d) {
                let y = nt.row_top(k);
                line(Point::new(nt.rect.x0, y), Point::new(nt.rect.x1, y));
            }
        }
        TableDragKind::MoveRows { .. } => {
            if let Some(k) = App::table_drop_line(t, d.kind, d.current) {
                let y = t.row_top(k);
                line(Point::new(r.x0, y), Point::new(r.x1, y));
            }
        }
        TableDragKind::MoveCols { .. } => {
            if let Some(k) = App::table_drop_line(t, d.kind, d.current) {
                let x = t.col_x(k);
                line(Point::new(x, r.y0), Point::new(x, r.y1));
            }
        }
        TableDragKind::Select { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracedraw_core::geometry::Rect;

    /// A 3 x 4 table of 30 x 10 mm cells from (0, 0) to (120, 30).
    fn app_with_table() -> (App, ShapeId) {
        let mut app = App::headless();
        app.tool = Tool::Table;
        let t = Table::new(
            Rect::new(0.0, 0.0, 120.0, 30.0),
            3,
            4,
            Some(Stroke::hairline(tracedraw_core::Color::BLACK)),
        );
        let id = app.new_shape(ShapeKind::Table(t)).unwrap();
        app.select(vec![id]);
        (app, id)
    }

    /// Centre of cell (r, c).
    fn at(r: u32, c: u32) -> Point {
        Point::new(c as f64 * 30.0 + 15.0, 30.0 - r as f64 * 10.0 - 5.0)
    }

    fn table(app: &App) -> Table {
        app.selected_table().unwrap().1
    }

    fn cells(app: &App) -> Vec<(u32, u32)> {
        let mut c = app.table_edit.as_ref().unwrap().cells.clone();
        c.sort_unstable();
        c
    }

    #[test]
    fn drag_ctrl_and_shift_select_cells() {
        let (mut app, id) = app_with_table();
        assert!(app.table_drag_begin(id, at(0, 0), 0.5));
        app.table_drag_to(at(1, 2));
        app.table_drag_end();
        assert_eq!(
            cells(&app),
            vec![(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)]
        );
        assert_eq!(app.table_edit.as_ref().unwrap().cells[0], (0, 0));
        // Dragging beyond the table takes the edge cells.
        assert!(app.table_drag_begin(id, at(2, 3), 0.5));
        app.table_drag_to(Point::new(500.0, 500.0));
        app.table_drag_end();
        assert_eq!(cells(&app), vec![(0, 3), (1, 3), (2, 3)]);
        // Ctrl+click adds and takes away a cell.
        app.table_click(id, at(0, 0), false);
        app.table_toggle_cell(id, at(2, 2));
        assert_eq!(cells(&app), vec![(0, 0), (2, 2)]);
        app.table_toggle_cell(id, at(0, 0));
        assert_eq!(cells(&app), vec![(2, 2)]);
        app.table_toggle_cell(id, at(2, 2));
        assert_eq!(cells(&app), vec![(2, 2)], "the last cell stays");
        // Shift+click: the block from the active cell.
        app.table_range_to(id, at(1, 0));
        assert_eq!(
            cells(&app),
            vec![(1, 0), (1, 1), (1, 2), (2, 0), (2, 1), (2, 2)]
        );
    }

    #[test]
    fn edge_pointers_select_rows_columns_and_everything() {
        let (mut app, id) = app_with_table();
        app.table_header_select(id, TableHit::RowHeader(1), false);
        assert_eq!(cells(&app), vec![(1, 0), (1, 1), (1, 2), (1, 3)]);
        app.table_header_select(id, TableHit::RowHeader(2), true);
        assert_eq!(cells(&app).len(), 8);
        app.table_header_select(id, TableHit::ColHeader(3), false);
        assert_eq!(cells(&app), vec![(0, 3), (1, 3), (2, 3)]);
        app.table_header_select(id, TableHit::Corner, false);
        assert_eq!(cells(&app).len(), 12);
        // Ctrl+A: the cell's text first, then every cell.
        app.table_click(id, at(0, 0), false);
        app.cell_insert("ab");
        app.table_select_all();
        assert_eq!(app.cell_selected_text(), "ab");
        assert_eq!(cells(&app), vec![(0, 0)]);
        app.table_select_all();
        assert_eq!(cells(&app).len(), 12);
    }

    #[test]
    fn tab_options_insert_a_tab_or_go_down_the_columns() {
        let (mut app, id) = app_with_table();
        app.table_click(id, at(0, 0), false);
        app.table_tab(false, false);
        assert_eq!(cells(&app), vec![(0, 1)]);
        app.settings.table.tab_by_columns = true;
        app.table_tab(false, false);
        assert_eq!(cells(&app), vec![(1, 1)]);
        app.table_tab(true, false);
        assert_eq!(cells(&app), vec![(0, 1)]);
        app.settings.table.tab_inserts_tab = true;
        app.table_tab(false, false);
        assert_eq!(app.cell_text(), "\t");
        app.table_tab(false, true);
        assert_eq!(cells(&app), vec![(1, 1)], "Ctrl+Tab still moves on");
    }

    #[test]
    fn dragging_lines_resizes_and_undoes_in_one_step() {
        let (mut app, id) = app_with_table();
        // The line between columns 0 and 1 at x = 30.
        assert!(app.table_drag_begin(id, Point::new(30.2, 15.0), 0.5));
        app.table_drag_to(Point::new(40.0, 15.0));
        app.table_drag_end();
        let t = table(&app);
        assert!((t.col_widths[0] - 39.8).abs() < 1e-9, "{:?}", t.col_widths);
        assert!((t.col_widths[1] - 20.2).abs() < 1e-9);
        assert!((t.rect.width() - 120.0).abs() < 1e-9);
        // The bottom edge grows the last row downwards; the top stays.
        assert!(app.table_drag_begin(id, Point::new(50.0, 0.1), 0.5));
        app.table_drag_to(Point::new(50.0, -4.9));
        app.table_drag_end();
        let t = table(&app);
        assert!(
            (t.row_heights[2] - 15.0).abs() < 1e-9,
            "{:?}",
            t.row_heights
        );
        assert!((t.rect.y1 - 30.0).abs() < 1e-9);
        app.engine.undo().unwrap();
        app.engine.undo().unwrap();
        assert_eq!(table(&app).col_widths, vec![30.0; 4]);
    }

    #[test]
    fn selected_rows_and_columns_move_by_dragging() {
        let (mut app, id) = app_with_table();
        app.table_click(id, at(0, 0), false);
        app.cell_insert("first");
        app.table_header_select(id, TableHit::RowHeader(0), false);
        assert_eq!(app.table_selected_rows(id, &table(&app)), Some((0, 1)));
        assert!(app.table_drag_begin(id, at(0, 1), 0.5));
        // Drop on the bottom edge.
        app.table_drag_to(Point::new(45.0, 0.5));
        app.table_drag_end();
        let t = table(&app);
        let moved = t.cells.iter().find(|c| !c.text.is_empty()).unwrap();
        assert_eq!((moved.row, moved.col), (2, 0));
        assert_eq!(cells(&app), vec![(2, 0), (2, 1), (2, 2), (2, 3)]);
        // A column goes to the far left.
        app.table_header_select(id, TableHit::ColHeader(0), false);
        assert!(app.table_drag_begin(id, at(1, 0), 0.5));
        app.table_drag_to(Point::new(119.0, 15.0));
        app.table_drag_end();
        let t = table(&app);
        let moved = t.cells.iter().find(|c| !c.text.is_empty()).unwrap();
        assert_eq!((moved.row, moved.col), (2, 3));
    }

    #[test]
    fn bar_size_insert_n_and_split_n_follow_the_selection() {
        let (mut app, id) = app_with_table();
        app.table_header_select(id, TableHit::ColHeader(1), false);
        app.table_header_select(id, TableHit::ColHeader(2), true);
        assert_eq!(app.table_cell_size(), Some((60.0, 30.0)));
        app.table_set_cell_size(Some(80.0), None);
        let t = table(&app);
        assert_eq!(t.col_widths, vec![30.0, 40.0, 40.0, 30.0]);
        assert!((t.rect.width() - 140.0).abs() < 1e-9);
        // Two columns selected: Insert Column Right adds two.
        assert_eq!(app.table_selection_extent(), (3, 2));
        app.table_insert_cols(2, true);
        assert_eq!(table(&app).cols(), 6);
        // Above the selection: the selection moves down with its cells.
        app.table_click(id, at(1, 0), false);
        app.table_insert_rows(3, false);
        assert_eq!(table(&app).rows(), 6);
        assert_eq!(cells(&app), vec![(4, 0)]);
        // Split the active cell into three columns.
        app.table_split(false, 3);
        let t = table(&app);
        assert_eq!(t.cols(), 8);
        assert!(t.cell_index(4, 2).is_some());
        // Distribute just the selected rows.
        app.table_header_select(id, TableHit::RowHeader(0), false);
        app.table_header_select(id, TableHit::RowHeader(1), true);
        app.table_set_cell_size(None, Some(30.0));
        app.table_distribute(true);
        let t = table(&app);
        assert!((t.row_heights[0] - 15.0).abs() < 1e-9 && (t.row_heights[1] - 15.0).abs() < 1e-9);
    }

    #[test]
    fn fill_borders_margins_and_spacing_of_selected_cells() {
        let (mut app, id) = app_with_table();
        assert!(!{
            app.table_edit = None;
            app.table_set_cell_fill(Some(Fill::Solid(tracedraw_core::Color::WHITE)))
        });
        app.table_click(id, at(0, 0), false);
        app.table_toggle_cell(id, at(1, 1));
        let red = Fill::Solid(tracedraw_core::Color::rgb8(200, 0, 0));
        assert!(app.table_set_cell_fill(Some(red.clone())));
        let t = table(&app);
        let fills: Vec<_> = t.cells.iter().filter(|c| c.fill.is_some()).collect();
        assert_eq!(fills.len(), 2);
        assert_eq!(app.table_active_fill(), Some(red));
        // A palette click with the Table tool fills the selected cells.
        app.apply_fill(Fill::Solid(tracedraw_core::Color::rgb8(0, 0, 200)));
        let t = table(&app);
        assert_eq!(t.cell_fill, Fill::None, "the table's fill is untouched");
        assert!(t
            .cells
            .iter()
            .filter(|c| c.fill.is_some())
            .all(|c| c.fill == Some(Fill::Solid(tracedraw_core::Color::rgb8(0, 0, 200)))));
        // Top borders of the selection only.
        app.table_ui.border_set = BorderSet::Top;
        let mut s = Stroke::hairline(tracedraw_core::Color::rgb8(0, 120, 0));
        s.width = 0.7;
        app.table_apply_borders(Some(s.clone()));
        let t = table(&app);
        let c = &t.cells[t.cell_index(0, 0).unwrap()];
        assert_eq!(c.borders.top, EdgeBorder::Stroke(Box::new(s.clone())));
        assert_eq!(c.borders.bottom, EdgeBorder::Table);
        assert_eq!(app.table_shown_border(), Some(s));
        app.table_set_margins(tracedraw_core::CellMargins {
            left: 2.0,
            right: f64::NAN,
            top: 1.0,
            bottom: 0.5,
        });
        let t = table(&app);
        assert_eq!(t.margins.unwrap().right, 0.0);
        app.table_set_spacing(Some([1.0, 2.0]));
        assert_eq!(table(&app).spacing, Some([1.0, 2.0]));
        // Every one of these is a single undo step.
        for _ in 0..5 {
            app.engine.undo().unwrap();
        }
        let t = table(&app);
        assert!(t
            .cells
            .iter()
            .all(|c| c.fill.is_none() && c.borders.is_default()));
    }

    #[test]
    fn text_style_reaches_every_selected_cell() {
        let (mut app, id) = app_with_table();
        app.table_click(id, at(0, 0), false);
        app.cell_insert("one");
        app.table_next_cell(false);
        app.cell_insert("two");
        app.table_click(id, at(0, 0), false);
        app.table_range_to(id, at(0, 1));
        app.text_bold = true;
        app.text_size_pt = 30.0;
        app.text_align = tracedraw_core::TextAlign::Center;
        app.apply_text_style();
        let t = table(&app);
        for c in [(0, 0), (0, 1)] {
            let cell = &t.cells[t.cell_index(c.0, c.1).unwrap()];
            assert!(cell
                .text
                .iter()
                .all(|s| s.bold && (s.size_pt - 30.0).abs() < 1e-9));
            assert_eq!(cell.align, tracedraw_core::TextAlign::Center);
        }
        // The Text docker's span edits go to the cells as well.
        crate::ui::text_docker::edit_spans(&mut app, |s| s.italic = true);
        let t = table(&app);
        assert!(t
            .cells
            .iter()
            .filter(|c| !c.text.is_empty())
            .all(|c| c.text.iter().all(|s| s.italic)));
        assert!(app.active_cell_text_shape().is_some());
    }
}
