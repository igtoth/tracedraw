# Table tool and Table menu

## Drawing

- Property bar: rows (default 3), columns (default 4), cell fill (off by
  default; a colour when on), border width (hairline by default) and
  border colour, then the hint. With a table selected, the fill and
  border fields edit that table; otherwise they set the defaults for
  the next one.
- Drag on empty canvas: a table with equal row heights and column
  widths filling the rectangle (a drag smaller than 1 mm either way
  draws nothing). Cells have a 1 mm padding.
- Click inside an existing table: that cell becomes the active cell
  and typing goes into it; Shift+click adds cells to the selection.
  Clicking elsewhere commits the text. A drag that starts over a table
  does nothing (it does not draw a second table on top).
- Tab commits and moves to the next cell, wrapping rows; Shift+Tab
  moves back. Cells covered by a merged cell are skipped.

## Table menu

- Insert: row above, row below, column left, column right (relative to
  the active cell; the new row or column copies its neighbour's size;
  merged cells spanning the insertion point grow).
- Select: cell, row, column, table.
- Delete: row, column, table (a table keeps at least one row and one
  column).
- Distribute: rows evenly, columns evenly (the table size is kept).
- Merge Cells: the bounding block of the selected cells becomes one
  cell whose text is the texts of the merged cells in order.
  Unmerge Cells restores one cell per grid position; the text and fill
  stay in the top-left one.
- Split to Rows / Split to Columns: the active row or column is split in
  two equal halves.
- Convert Text to Table: lines become rows, tabs separate cells.
  Convert Table to Text: cells joined by tabs, rows by newlines.
- Every change is one `SetShapeKind` command after `Table::refit`, so the
  table rectangle always equals the sum of its rows and columns.

## Checks

- Given a 3 x 4 table, when a cell is clicked and "abc" typed, then Tab,
  then the first cell holds "abc" and the second is active.
- Given rows 2, columns 5, a white cell fill and a 0.5 mm border on the
  bar, when a 100 x 40 mm rectangle is dragged, then the table has 2 x 5
  cells with that fill and border and is 100 mm wide.

## Typing in cells

With the Table tool, a click in a cell makes it active and puts the caret
at the character under the pointer. Typing, Backspace, Delete, Home, End,
Ctrl+Home/End, Ctrl+A and Ctrl+B/I/U work as in text editing (formatting
applies to the selected characters, or to the whole cell without a
selection); Enter adds a line, Tab and Shift+Tab move to the next and
previous cell, and the arrow keys step to the neighbouring cell at the
edges of the text. While a cell is active every key goes to it, so
letters never trigger canvas shortcuts. A row grows downwards (the table
top stays put) when its text needs more height than the row has; rows do
not shrink while typing. All keystrokes in one cell are one "Edit Cell"
undo step.
