# Table tool and Table menu

## Drawing

- Table > Create New Table (and Object > Create > New Table): Number of
  Rows, Number of Columns, Height and Width in the document's units
  (100 x 75 mm at first; the last values are offered again), "?", OK
  and Cancel. OK places the table at the page centre and selects it
  (`ui::dialogs::tests::create_new_table_takes_rows_columns_and_size`).
- Property bar: rows (default 3), columns (default 4), cell fill (off by
  default; a colour when on), border width (hairline by default),
  border colour and style (solid, dashed, dotted), then the hint. With a
  table selected, the fill and border fields edit that table; otherwise
  they set the defaults for the next one. With cells selected the bar
  acts on them (see "Selected cells" below).
- Drag on empty canvas: a table with equal row heights and column
  widths filling the rectangle (a drag smaller than 1 mm either way
  draws nothing). Cells have a 1 mm margin on every side.
- A drag that starts over a table (or on its lines and edge pointers)
  never draws a second table on top.

## Selecting cells

The Table tool works on the table being edited, else the selected one.
The reach of its lines and edge pointers is 4 screen points.

- Click in a cell: it becomes the active cell (the caret goes to the
  character under the pointer) and the only selected cell.
- Drag across cells: the block from the cell where the drag began to
  the cell under the pointer; past the table's edge the edge cells are
  taken. A block always grows to take in whole merged cells.
- Ctrl+click adds a cell to the selection, or takes a selected one out
  (the last one stays). Shift+click selects the block from the active
  cell to the clicked one.
- Just left of the table's left edge the pointer is an arrow pointing
  right: a click selects that row (Shift+click: the rows from the active
  one to it). Just above the top edge the arrow points down and a click
  selects that column. At the top-left corner a click selects every
  cell. The zones reach 3 times the line reach outside the edge and
  half of it inside.
- Ctrl+A in a cell selects its text; pressed again (or in an empty
  cell) it selects every cell of the table.
- Tab and Shift+Tab move to the next and previous cell, skipping cells
  covered by a merged one. Options > Tools > Table sets the order
  (along the rows, the default, wrapping to the next row; or down the
  columns, wrapping to the next column) and what Tab does: move to the
  next cell (default) or type a tab character into the cell, where
  Ctrl+Tab still moves on and Shift+Tab moves back.
  (`table_tool::tests::tab_options_insert_a_tab_or_go_down_the_columns`)
- Table > Select: cell, row, column, table.

## Moving rows and columns

When the selection is one or more whole rows (or whole columns), but
not the whole table, a drag that starts inside it moves them: a line
shows where they would go (the row or column line nearest the
pointer) and the release puts them there in one "Move Rows" or "Move
Columns" undo step; they stay selected. A move that would cut a merged
cell does nothing and says so on the status bar.
(`table_tool::tests::selected_rows_and_columns_move_by_dragging`)

## Resizing

- Over a column line the pointer shows the column resize arrows, over a
  row line the row resize arrows. Dragging an inner column line moves
  width from one of its columns to the other (the table keeps its
  width); dragging the right edge resizes the last column. A row line
  works the same way down the rows; the bottom edge resizes the last
  row, and the table's top stays put. No row or column gets smaller
  than 1 mm. A line shows the new place while dragging; the release is
  one "Resize Table" step.
- Property bar W and H (document units, with cells selected): the total
  width of the columns and the total height of the rows the selected
  cells cover; a new value is shared out over them in proportion. The
  table grows or shrinks to the right and downwards.
- Table > Distribute: Rows Evenly and Columns Evenly share the total of
  the selected rows (columns) equally when the selection covers two or
  more; otherwise every row (column) of the table. The table size is
  kept.
  (`table_tool::tests::dragging_lines_resizes_and_undoes_in_one_step`,
  `table_tool::tests::bar_size_insert_n_and_split_n_follow_the_selection`)

## Table menu

- Insert: Row Above, Row Below, Column Left, Column Right insert as many
  rows (columns) as the selected cells cover, above the first or below
  the last of them (left of the first, right of the last); without a
  selection one. Rows..., Columns...: a dialog with the number (1 to
  100, the selection's count at first) and the side (above or below
  the selection; left or right of it). New rows and columns take the
  size of the row or column next to them; merged cells spanning the
  insertion line grow; the selected cells keep their place in the
  table.
- Select: cell, row, column, table.
- Delete: row, column, table (a table keeps at least one row and one
  column).
- Distribute: rows evenly, columns evenly (see Resizing).
- Merge Cells: the block the selected cells span (grown over merged
  cells) becomes one cell with the upper-left cell's background,
  borders and alignment, whose text is the texts of the merged cells in
  order. Unmerge Cells restores one cell per grid position; the text
  and fill stay in the top-left one.
- Split into Rows... / Split into Columns...: a dialog with the number
  of parts (2 to 100). The selected cells in the active cell's band
  (same first row and row span; for columns, same first column and
  column span) are split; for a single grid row (column) it becomes N
  equal ones, and the other cells of that row (column) span them; a
  merged cell of K rows (columns) splits without new grid lines when N
  divides K; otherwise the split is refused with a status message. The
  first part keeps the text; every part keeps the background,
  alignment and outer borders.
  (`table_bar::tests::the_count_dialog_inserts_and_splits`,
  `tracedraw_core::table::tests::split_a_cell_into_three_rows_and_columns`)
- Convert Text to Table (a dialog, #1276): paragraphs become rows; the
  cells are cut at commas, tabs, a user-defined separator (an empty one
  keeps each paragraph whole) or not at all (Paragraphs: one column, a
  row each). The dialog opens on tabs when the text has one, else on
  commas when it has one, else on Paragraphs, and shows the rows and
  columns the table will have. Cells are trimmed of spaces; closing
  empty paragraphs make no rows. The table takes the text's top left
  corner, its first style and at least its width (columns at least
  20 mm and as wide as their widest text on one line with the cell
  margins, rows 1.6 lines of the text's size).
  (`table::tests::text_becomes_a_table_by_its_separator`,
  `ui::dialogs::tests::convert_text_to_table_takes_the_chosen_separator`)
  Convert Table to Text: cells joined by tabs, rows by newlines.
- Every change is one `SetShapeKind` command after `Table::refit`, so the
  table rectangle always equals the sum of its rows and columns.

## Selected cells

With cells selected (the active cell counts), the property bar shows:

- W and H (see Resizing).
- Cell background: the active cell's fill (its own, else the table's);
  ticking it gives the selected cells white, unticking makes them
  transparent, the colour button sets their colour. A click on a
  palette colour with the Table tool fills the selected cells, not the
  table ("Cell Fill" undo step).
- Border: which borders of the selected cells to set (All, Outside,
  Inside, Top, Bottom, Left, Right, Inner horizontal, Inner vertical),
  then the width (0 = no border), colour and style. Outside, Top,
  Bottom, Left and Right are the sides on the outline of the selection;
  Inside and the inner choices are the lines between two selected
  cells. Without selected cells the choice applies to every cell, and
  All sets the table's own border (cells lose their own settings).
- Options: cell margins (one value for every side, or Top, Bottom,
  Left and Right with "A margin for each side"), and Separated borders
  with a horizontal and a vertical spacing (1 mm each when switched on).
- Font, size, bold, italic, underline and alignment of the selected
  cells' text.

### Borders per edge

Every side of a cell has a border of its own: the table's border (the
default), none, or its own outline. A line shared by two cells takes the
setting of the cell that has one; when both do, the cell below (or to
the right) wins. Setting a side on the selection's outline also sets
the facing side of the cell across it when that one had its own
setting, so the new border shows. Lines inside a merged cell are not
drawn. With separated borders every cell draws its own box, smaller by
half the spacing on each side, and the table's border draws the table's
outline.
(`tracedraw_core::table::tests::outside_borders_of_a_selection_take_their_own_stroke`,
`tracedraw_core::table::tests::each_border_choice_picks_its_sides`,
`tracedraw_core::table::tests::separated_borders_shrink_cells_and_draw_the_outline`)

### Text of several cells

The Text docker shows the active cell's text. Its character settings
(font, style, size, kerning, underline, fill) and the property bar's
text fields change the selected characters of the cell being typed in;
with several cells selected they change every character of every
selected cell (one "Cell Text" step), and the alignment sets the cells'
alignment. Empty cells keep no style.
(`table_tool::tests::text_style_reaches_every_selected_cell`)

## Checks

- Given a 3 x 4 table, when cells are dragged from (0, 0) to (1, 2),
  then six cells are selected and (0, 0) is active
  (`table_tool::tests::drag_ctrl_and_shift_select_cells`).
- Given a selected row, when it is dragged to the bottom edge, then it
  becomes the last row and stays selected.
- Given a 3 x 4 table, when the first column line is dragged 9.8 mm to
  the right, then the first two columns are 39.8 and 20.2 mm wide and
  the table stays 120 mm wide; one undo restores them.

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
