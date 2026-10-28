# Open drawings and document tabs

Several drawings can be open at once, one tab each in the strip above the
rulers:

- The strip is 26 px high over a 1 px `#00ADFE` line that runs the width
  of the window. Tabs are `#D8D8D8` with 1 px gaps of the strip colour
  `#EAEAEA`; the active tab is `#CEE3FF`; labels are 13 px black.
- The Welcome Screen tab comes first (a filled dark house and its title).
  With no drawing open it is the only tab, and the menu bar shows only
  File, Tools, Window and Help; File commands that need a drawing are
  disabled. While it is shown (with drawings open too) the toolbox and the
  property bar show pale, the colour palette shows pale and takes no
  clicks, and the dockers that work on a drawing say "Open or create a
  drawing to use this docker." instead of showing the hidden drawing's
  selection; Hints still shows (#482,
  `ui::palette::tests::the_palette_rests_on_the_welcome_screen`).
- The Welcome Screen tab has a close button on hover (a middle click
  also closes it): the tab shrinks to its house alone (a 26 px square,
  the title as its tip) and the active drawing shows. With no drawing
  open the workspace stays empty and grey, the toolbox and property bar
  stay pale, and the Hints docker comes to the front at its Home topics
  (#1081). A click on the house, Window > Welcome Screen or Help >
  Welcome Screen shows the Welcome Screen again with its whole tab. The
  closed tab is remembered with the settings (#677).
- Each drawing's tab (at least 116 px wide) shows its file name, or its
  document name when it has never been saved, followed by `*` while it has
  unsaved changes. A close button shows on the hovered tab; a middle click
  also closes.
- The New tab after the last one (a grey plus on a 26 px square) runs
  File > New.

Each drawing keeps its own pages, objects, undo history, selection, view
(zoom and scroll), file, page number settings and drawing units.
Switching tabs ends the interaction in progress (a drag, text or table
cell editing, a curve being drawn, clip frame editing, symmetry editing).

New and opened drawings get a tab after the last one. A file that is
open already is not read into a second tab (#645): opening it again (the
Welcome Screen's recent drawings, File > Open and Open Recent, the
toolbar's Open, a file dropped on the Welcome Screen, the command line)
shows the tab of the drawing read from it or saved to it, its unsaved
changes kept, and the status bar says "<name> is already open". The same
file is found by another spelling of its path (links, `.` and `..`, the
letter case on Windows); a drawing read from a `.cdr`, `.svg`, `.pdf` or
other format counts as that file until it is saved under a name of its
own. Templates (`.tdt`, `.cdt`, `.cdrt`) always make a new untitled
drawing, File > Revert reads the file again, Import is not an open, and
the browser build does not look (uploads of one name are different
files). Window > New Window gives a second window on an open drawing
(`documents::tests::opening_an_open_file_again_shows_its_tab`,
`templates_always_open_a_new_drawing`).

Closing a tab
(File > Close, Ctrl+F4, the tab's close button) asks to save unsaved
changes, then shows the tab to its right, else the one to its left, else
the Welcome Screen (the empty workspace when its tab is closed). Window > Close All closes every drawing, asking about
each one with unsaved changes; Cancel stops. File > Exit, Alt+F4 and the
window's close button do the same and quit when every drawing is closed.

The Window menu lists the open drawings, numbered, with a check on the
active one.

## Checks

- `documents::tests::each_drawing_keeps_its_objects_history_and_selection`
- `documents::tests::closing_moves_to_a_neighbour_then_to_the_welcome_screen`
- `documents::tests::exit_asks_about_each_unsaved_drawing_then_quits`
- Given a drawing open and the Welcome Screen shown, when its tab's
  close button is clicked, then the drawing shows and the tab is a house
  half as wide or less; a click on the house shows the Welcome Screen
  with its whole tab; with no drawing open it closes to the empty
  workspace with Hints at Home
  (`pick_drag_tests::the_welcome_tab_closes_to_its_house`).
- Given the Welcome Screen tab closed, when the last drawing closes, then
  the workspace stays empty
  (`pick_drag_tests::closing_the_last_drawing_keeps_the_welcome_tab_choice`).
