# Open drawings and document tabs

Several drawings can be open at once, one tab each in the strip above the
rulers:

- The Welcome Screen tab comes first (a house and its title). With no
  drawing open it is the only tab, and the menu bar shows only File,
  Tools, Window and Help; File commands that need a drawing are disabled.
- Each drawing's tab shows its file name, or its document name when it
  has never been saved, followed by `*` while it has unsaved changes. The
  active tab is highlighted. A close button shows on the active tab and
  on the hovered one; a middle click also closes.
- The New button after the last tab runs File > New.

Each drawing keeps its own pages, objects, undo history, selection, view
(zoom and scroll), file, page number settings and drawing units.
Switching tabs ends the interaction in progress (a drag, text or table
cell editing, a curve being drawn, ClipFrame editing).

New and opened drawings get a tab after the last one. Closing a tab
(File > Close, Ctrl+F4, the tab's close button) asks to save unsaved
changes, then shows the tab to its right, else the one to its left, else
the Welcome Screen. Window > Close All closes every drawing, asking about
each one with unsaved changes; Cancel stops. File > Exit, Alt+F4 and the
window's close button do the same and quit when every drawing is closed.

The Window menu lists the open drawings, numbered, with a check on the
active one.

## Checks

- `documents::tests::each_drawing_keeps_its_objects_history_and_selection`
- `documents::tests::closing_moves_to_a_neighbour_then_to_the_welcome_screen`
- `documents::tests::exit_asks_about_each_unsaved_drawing_then_quits`
