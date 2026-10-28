# Drawing window

## Desktop and page

- The desktop (the area around the page) is white by default; the
  colour is a setting (`desktop_rgb`, Options > Customization).
- The page is white with a 1 px border `#AAAAAA` and a flat shadow
  `#BFBFBF`: the bordered page rectangle moved 6 px right and 4 px down,
  drawn behind the page, so a 6 px band shows on the right and a 4 px
  band at the bottom. The page rectangle is snapped to whole pixels.
  View > Page > Page Border hides the border and the shadow.
- View > Mask Outside Page, right after the view modes (Pixels), as in
  the reference program (off by default, kept with the other view
  settings): an opaque colour covers everything outside the page
  (outside every page in Multipage View) over the objects, and the page
  border is drawn over the objects, so the page shows as it will print
  or export. The colour is the desktop colour, or a near-black `#222222`
  when the desktop is as light as the paper (every channel at least
  `#E0`, as the default white desktop), so the page stands out. The
  selected objects' outlines are drawn over the mask as a 1 px line in
  the selection blue, so a part off the page can still be found and
  picked (flattened once per change of the selection or the zoom, at
  most 200,000 points); every tool works as usual, and the selection
  handles, guidelines and previews stay on top (#615,
  `pick_drag_tests::mask_outside_page_covers_the_desktop_and_keeps_objects_editable`,
  `canvas::tests::the_mask_covers_everything_outside_the_pages`).
- Double-clicking the page border (within 3 px) or its shadow with the
  Pick tool, where no object is hit, opens the page size options
  (Layout > Document Options at Page Size).

## Scrollbars

- The vertical scrollbar runs down the right edge between the ruler and
  the bottom row; the horizontal one fills the right part of the bottom
  row. Both are 17 px: arrow buttons at the ends, a light track and a flat
  grey thumb that darkens when hovered or dragged.
- They cover the page plus one page size on every side, grown to include
  whatever is visible. Dragging the thumb pans; a click on the track pages
  by 90 % of the window towards the click; an arrow moves 40 px, then
  repeats every 50 ms after 0.4 s while held.

## Bottom row

From the left, after the ruler's width:

1. The document navigator: insert page before, first page, previous
   page, "n of m", next page, last page, insert page after. The insert
   buttons add a page of the current size next to the current page and
   make it current.
2. The page tabs: one slanted tab per page, the current one white; a click
   goes to the page, a double click renames it. The tabs that fit in the
   row are shown; when the current page changes (a tab, the navigator,
   PgUp and PgDn, Go To Page) they slide just enough for its tab to show
   whole, and the mouse wheel over them slides them one tab a notch
   without changing the page. Before, the tabs past the row's end never
   showed, so in a long drawing (a merged document) their pages could be
   reached only with Go To Page (#794,
   `pick_drag_tests::page_tabs_slide_to_the_current_page_of_a_long_drawing`). A right click makes the
   page current and opens its menu, six rows with a picture each and no
   separators: Rename Page... (a page with a text cursor), Insert Page
   After and Insert Page Before (two pages side by side, the new one with
   a green plus, on the right or on the left, under an arrow toward it),
   Duplicate Page... (two pages with a green plus; opens the Duplicate
   Page dialog, see [pages.md](pages.md)), Delete Page (a page with a red
   cross; greyed when it is the only page) and Switch Page Orientation.
   Only the rows that open a dialog end in an ellipsis. Page size and
   background are in the Layout menu
   (`ui::window_bars::tests::the_page_tab_menu_has_six_rows_in_order_each_with_a_picture`,
   `the_page_tab_menu_has_no_separators`,
   `the_page_tab_menu_inserts_duplicates_turns_and_deletes_pages`).
3. A dotted splitter: dragging it shares the row between the page tabs and
   the horizontal scrollbar (45 % for the tabs by default).
4. The horizontal scrollbar.

The Navigator button (a magnifier over a cross) sits in the corner under
the vertical scrollbar; see `zoom-and-pan.md`.

## Windows

With drawings open, Window > Cascade, Tile Horizontally and Tile
Vertically put every drawing in a window of its own inside the drawing
area (`ui/windows.rs`); Combine Windows and Dock Window bring the tabs
back. The tabs stay above the area and still choose the active drawing.

- Tile Horizontally: the windows one above the other, each as wide as
  the area and an equal share of its height. Tile Vertically: side by
  side, each as tall as the area. Cascade: windows of 72 % of the area
  (at least 320 x 240 px, at most the area), each 28 px down and to the
  right of the one before, starting again at the top left when the next
  one would leave the area; the area behind them is `#BEBEBE`.
- A window has a 24 px title bar: the drawing's name (an asterisk while
  it has unsaved changes; bold on `#CEE3FF` for the active drawing, on
  `#E6E6E6` for the others), then a button that brings the tabs back with
  this drawing active (as a double click on the bar does) and one that
  closes the drawing (asking to save first when it has changes). The
  active window has a 1 px accent frame, the others a grey one.
- The active window is drawn on top with its rulers, scrollbars, page
  tabs and tools. The others show their page, grids, objects and
  guidelines as their own view left them (each drawing keeps its view
  and its rendered page), without the editing going on in the active
  window; a click on one makes its drawing the active one.
- A new arrangement, another number of windows, or the tabs again fit
  every page into its window. Otherwise a drawing stays where it was in
  its window when the window moves or the canvas begins elsewhere (the
  rulers shown or hidden, a bar docked on another side): its view moves
  with the canvas.

## New Window

Window > New Window (greyed with only the Welcome Screen) opens another
window of the active drawing: a tab right after the drawing's last
window, made active, showing the page, view and selection the active
window shows.

- All windows of a drawing show one document with one undo history: an
  edit, undo or redo in one is what every other shows. Each window keeps
  its own page, view (zoom and scroll) and selection; objects removed in
  another window drop out of a window's selection when it becomes active
  again.
- While a drawing has more than one window, its tabs, window title bars,
  Window menu entries and the main title read "name:1", "name:2" ... in
  the order the windows were opened (the asterisk for unsaved changes
  follows the number). Closing a window numbers the rest again from 1; a
  drawing left with one window shows its plain name.
- Save and Save As from any window save the drawing; every window then
  shows it saved, under the new name.
- Closing a window while the drawing has another open closes only that
  window, without asking. Closing the last window closes the drawing as
  before, asking to save when it has changes. Close All and quitting ask
  once per changed drawing; auto-backup writes one file per drawing.
- Tiled and cascaded windows (above) draw each window of a drawing with
  its own view.

## Drawing the page

- The objects are rasterised with the CPU renderer into one texture the
  size of the canvas (`raster.rs`), rendered anew only when the drawing,
  the view, the canvas size or the drag preview changes.
- Objects wholly outside the part of the page on screen are not drawn
  (`render::Renderer::out_of_sight`): an object's bounds, grown by what
  its outline may reach (twelve widths, scaled with the object), must
  touch the visible page rectangle. Objects whose extent is not their
  bounds (effects, drop shadows, symbols, the objects of a drag preview)
  are always drawn. Zoomed into a page of ten thousand rectangles the
  render takes less than half the time it did.
- While the view moves (a pan, a scroll, a zoom) the last render is
  shown moved or scaled into place at once, and the page is rendered
  anew once the view has rested for 0.12 s (`raster::SETTLE_SECONDS`);
  the part the last render did not cover stays blank until then. So a
  page of thousands of objects pans at the screen's rate whatever its
  size; a zoom looks coarse for that moment. The Pixels view never
  shows a moved render.
- The page is rendered in horizontal bands, one per processor core
  (up to 16), each on its own thread, when the canvas has a quarter of
  a million pixels or more; a band reaches past its edges by twice the
  widest drop shadow blur so shadows join without seams. A page with a
  lens is rendered whole, since a lens reads what is under it. The web
  build renders on one thread.
- When the last render took longer than 0.03 s
  (`raster::SLOW_RENDER`), the next one runs on a thread of its own
  with a copy of the drawing, and the last render stays on screen
  until it is ready; the window keeps answering the pointer and the
  keyboard meanwhile. A render made stale by a newer change is
  dropped. One such render runs at a time in the whole program (each
  has its own band threads and picture buffers): while one runs, other
  pages show their last picture, and the render repaints when it ends.
  The page being edited goes first: Multipage View draws the other
  pages each frame too, so when the current page had to wait, the freed
  turn is kept for it for a second and the other pages wait.
- Undo keeps one copy of the drawing per step (the state the step
  replaced) and swaps it in and out, instead of a copy before and one
  after; a command costs one copy of the drawing instead of two.
- Undo also keeps to a memory limit (`app::UNDO_MEMORY`: 1 GB, 256 MB
  in the browser; no Options field, Undo levels keeps its step count).
  Each step records about how much memory its copy holds of its own:
  every object and its path points, and the pictures the drawing does
  not share with it (a step that replaced or deleted a picture holds
  the old one). Past the limit the oldest steps go; the newest always
  stays. Nothing is measured per pointer move: a step recorded right
  after an Undo (a live colour drag) takes the undone step's size, and
  steps recorded during a preview are measured only if it is kept.
  `Engine::begin_group` and `end_group` keep a group of steps (a right
  click copy during a drag and its move) from being cut by the limit.

## Checks

- `tracedraw_render::culling_tests::objects_out_of_sight_are_skipped`
- `raster::tests::a_moving_view_shows_the_last_render_until_it_rests`
- `raster::tests::a_canvas_that_moves_renders_again`
- `raster::tests::a_slow_page_is_drawn_on_its_own_thread`
- `raster::tests::one_render_job_runs_at_a_time_and_the_current_page_goes_first`
- `raster::tests::the_current_page_gets_the_next_turn`
- `tracedraw_core::engine::tests::the_history_keeps_to_its_memory_limit`
- `tracedraw_core::engine::tests::a_group_stays_one_step_at_the_memory_limit`
- `tracedraw_core::engine::tests::live_steps_are_not_measured_again`

- `ui::windows::tests::windows_tile_and_cascade_inside_the_area`
- `ui::windows::tests::tiled_windows_draw_every_drawing_and_switch`
- `ui::windows::tests::the_view_stays_put_in_a_window_that_moves`
- `documents::tests::a_new_window_shares_the_drawing_and_its_undo_history`
- `documents::tests::each_window_keeps_its_own_view_page_and_selection`
- `documents::tests::window_titles_number_the_windows_of_a_drawing`
- `documents::tests::closing_one_window_keeps_the_drawing_in_the_other`
- `documents::tests::closing_a_parked_drawing_window_and_close_all`
- `documents::tests::saving_from_either_window_saves_the_one_drawing`
- `documents::tests::tiled_windows_draw_both_windows_of_a_drawing`
- `ui::menus::tests::window_new_window_opens_a_second_window`
- `ui::window_bars::tests::insert_page_before_and_after_the_current_one`
- `ui::window_bars::tests::the_page_border_and_shadow_are_hit_but_not_the_page_inside`
- `pick_drag_tests::mask_outside_page_covers_the_desktop_and_keeps_objects_editable`
- `canvas::tests::the_mask_covers_everything_outside_the_pages`
