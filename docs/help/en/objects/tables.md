# Tables

A table is an object made of rows and columns of cells, each of which can
hold text and a fill. Draw tables with the [[tool.table]] tool and change them
with the [[menu.table]] menu.

## Creating a table {#create-table}

- Choose [[menu.table > menu.table.create_new_table]] (or
  [[menu.object > menu.object.create > menu.object.create_table]]). In the
  [[dialog.create_table]] dialog box, set the [[dialog.table_rows]], the
  [[dialog.table_columns]] and the height and width (100 by 75 mm at first),
  and click [[dialog.ok]]. The table is placed in the middle of the page and
  selected.
- Or choose the [[tool.table]] tool and drag a rectangle: the table fills it,
  with equal rows and columns.

![The Create New Table dialog box](../../img/objects-tables-create-table.png)

On the property bar of the [[tool.table]] tool, set the number of rows (3 at
first) and columns (4 at first), the cell fill (none at first), and the border
width and color. With a table selected, the fill and border change that table;
otherwise they apply to the next table you draw.

## Typing in cells {#typing}

With the [[tool.table]] tool, click a cell and type. Text editing works as in
any text: <kbd>Ctrl+B</kbd>, <kbd>Ctrl+I</kbd> and <kbd>Ctrl+U</kbd> format the
selected characters, or the whole cell. <kbd>Enter</kbd> adds a line,
<kbd>Tab</kbd> and <kbd>Shift+Tab</kbd> go to the next and previous cell, and
the arrow keys step to the next cell at the edge of the text. A row grows when
its text needs more height. <kbd>Shift</kbd>+click adds cells to the
selection.

## Changing rows and columns {#table-menu}

The [[menu.table]] menu acts on the active cell and the selected cells:

- [[menu.table.insert]]: [[menu.table.insert_row_above]],
  [[menu.table.insert_row_below]], [[menu.table.insert_column_left]] and
  [[menu.table.insert_column_right]]. The new row or column takes the size
  of its neighbour.
- [[menu.table.select]]: [[menu.table.select_cell]],
  [[menu.table.select_row]], [[menu.table.select_column]] or
  [[menu.table.select_table]].
- [[menu.table.delete]]: [[menu.table.delete_row]],
  [[menu.table.delete_column]] or [[menu.table.delete_table]]. A table keeps
  at least one row and one column.
- [[menu.table.distribute]]: [[menu.table.distribute_rows]] or
  [[menu.table.distribute_columns]], keeping the size of the table.
- [[menu.table.merge_cells]] (<kbd>Ctrl+M</kbd>) makes the selected cells
  one cell, with their texts in order; [[menu.table.unmerge_cells]] splits it
  again, keeping the text and fill in the top left cell.
- [[menu.table.split_into_rows]] and [[menu.table.split_into_columns]] split
  the active row or column into two halves.

## Converting text and tables {#convert}

- [[menu.table.convert_text_to_table]]: each paragraph of the selected
  text becomes a row. The dialog box asks what separates the cells:
  commas, tabs, nothing (each paragraph is one cell, in one column) or
  characters of your own under User defined, and tells how many rows
  and columns the table will have. It starts with tabs when the text
  has any, else commas when it has any. The spaces around each cell's
  text are left out, so "Anna, Ben, Carl" with commas makes three
  cells.
- [[menu.table.convert_table_to_text]]: the cells are joined by tabs and the
  rows by line breaks.

The defaults of the tool are on the [[tool.table]] page of the tool options;
see [Tool options](../customize/tool-options.md#table).
