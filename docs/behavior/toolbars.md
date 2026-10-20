# Menu bar, standard toolbar and property bar

Interface text is 13 px for menus, labels, lists and the status bar, in
the bundled interface font. Toolbars,
panels, palettes, the status bar and dialog bodies are `#F4F4F4`; light
lines between bands are `#D8D8D8`.

## Menu bar

A 24 px `#D8D8D8` band. Titles are 13 px, the first 11 px from the edge
and about 25 px apart; the open or hovered title is a light blue box
(`#E0F0FF`) with a blue frame (`#00ADFE`). Menus are white with a 1 px
`#B2B2B2` border and square corners: a 30 px `#F4F4F4` gutter holds the
icons, the bold check marks and the dots of exclusive choices (View >
Wireframe, Normal, Enhanced, Pixels); rows are 26 px with labels from
38 px and shortcuts right-aligned 28 px from the edge, all black (light
grey when unavailable); separators are 7 px with a `#D8D8D8` line from
the gutter; a submenu row ends in a grey triangle and stays highlighted
while its submenu is open; a hovered row is the same blue box as the
titles. The canvas, palette and New Document menus are drawn the same
way.

The View menu has Zoom In, Zoom Out and Zoom
To Fit (with their pictures) and no other zoom commands.

## Panels

The open panel has a 27 px `#EAEAEA` title bar (its name at 12 px, then
collapse and close in grey at the right) over a white page. The tab
column to its right is 27 px wide: `#EAEAEA` under the title bar and
below the tabs, a `#B2B2B2` line along the panel, grey `#D8D8D8` tabs
with an icon and the name running downwards, the open panel's tab
`#B2B2B2`; the plus button under the tabs adds a panel.

## Standard toolbar

A 35 px band (and its line): 32 px buttons with 20 px icons in groups
split by thin lines:

1. New (a page with a green plus), Open with the recent drawings behind
   its arrow, Save, Print.
2. Cut, Copy, Paste.
3. Undo and Redo, each with its list of steps behind the arrow (choosing
   a step undoes or redoes up to it).
4. Import, Export, Publish to PDF.
5. The zoom box: the zoom level as text (type a value, Enter applies it)
   and, behind its arrow, Zoom to Selected, to All Objects, to Page, to
   Page Width, to Page Height and levels from 10% to 1600%. Full-screen
   preview.
6. Show Rulers, Show Grid, Show Guidelines: toggles drawn pressed (white
   with a grey frame) while on.
7. Snap Off (Alt+Q), pressed while snapping is off; Snap To with the
   snapping list.
8. Options (the TraceDraw options).
9. Launch: Font Manager, Welcome Screen, Color Management.

Buttons that need a drawing, and Snap To, are disabled on the Welcome
Screen, also when drawings are open behind it; the property bar and the
toolbox are pale there too, so the page fields cannot be changed
(`pick_drag_tests::the_page_bar_does_nothing_on_the_welcome_screen`).
New, Open, Options, Launch and AI Generate stay on. Hovered buttons are
light blue with a light blue frame.

## Property bar

A 52 px band (and its line); fields stack two to a column, 19 px each
and sharing their middle border, 8 px from the top; fields and lists are
white with a grey border and 13 px text, small pictures before fields
name them (hover for a tooltip), and some fields have spin arrows.

Number fields show their value at the left and are typed into: a click
selects the value, Enter or leaving the field applies it, Esc puts it
back. A comma or a point marks decimals; sums work (`10+5`, `2*3,5`,
brackets); a distance typed with another unit is converted (`1 in` in a
millimetre field is 25.4 mm). Values outside a field's range are brought
into it (`ui::field::tests`).

- Pick tool, nothing selected (page bar): page size list; page width and
  height; portrait, landscape; all pages or current page (which pages a
  size change applies to, one undo step); Units; nudge distance;
  duplicate distances; treat all objects as filled.
- A selection (object bar): the object origin selector (a 3 x 3 grid,
  the centre by default: X and Y show and move that point of the
  selection's box, counted from the ruler origin), X and Y; width and
  height (scaled about the origin point); scale percentages and the lock
  that keeps proportions; the rotation angle of a single object (read
  from its transform, changed about the origin point); mirror
  horizontally and vertically.
- Then, by the kind of the selected objects: rectangle corners (Round,
  Scalloped and Chamfered Corner buttons; the four corner sizes in two
  stacked columns, top left over bottom left, then top right over bottom
  right, with spin arrows; the Edit Corners Together lock; Relative
  Corner Scaling);
  ellipse, pie or arc with start and end angles and the change direction
  button; polygon points or sides and star sharpness (1 to 99); with
  the Star tool the Star and Complex Star buttons come first, and a
  complex star's sharpness counts steps (1 up to what its points allow).
- Then the outline: width (Hairline, None, preset widths in the ruler
  unit), line style, start and end arrowheads; then To Front, To Back and
  Convert to Curves.
- Zoom and Pan tools: the zoom box, Zoom In, Zoom Out, Zoom to Selected,
  to All Objects, to Page, to Page Width, to Page Height.
- Shape tool on curves: the Selection mode list, then picture buttons in
  groups (node edits, node types, curve direction and closing, node
  transforms and Align Nodes, Reflect and Elastic toggles, Select All
  Nodes, Reduce Nodes with the Curve smoothness slider); toggles are drawn
  pressed while on. The full list is in `shape-tool.md`.

## Bars longer than the window

The standard toolbar, the property bar, the text, zoom, transform and
Time Tracker bars and the toolbox show what fits between a "«" button at
their start and a "»" button at their end (up and down arrows for the
toolbox on a side). "»" slides the bar so the first item not wholly in
view comes first; "«" slides it back, cut between items, to the start;
the mouse wheel over the bar slides it too. The items stay as they are on
the bar and work as there (the bar is drawn once). A bar that fits shows
no buttons; the slide is worked out again when the window changes size
(`ui::overflow::tests`).

Dialogs taller (or wider) than the window keep their size inside a
window that fits the screen, and their body scrolls; dockers scroll as
before.

## Moving the bars

The toolbox, the colour palette, the standard toolbar, the property bar
and the status bar move (`ui/bars.rs`).

| Bar | Places | First place |
|---|---|---|
| Toolbox | top, bottom, left, right, floating | left |
| Colour palette | top, bottom, left, right, floating | bottom (on a side: the outermost column, beyond the dockers) |
| Standard toolbar | top, bottom, floating | top |
| Property bar | top, bottom, floating | top |
| Status bar | top, bottom | bottom |

- A docked bar starts with a grip: a column of grey dots 9 px wide (a
  row of dots 9 px high on a bar that runs down a side). The pointer
  shows the move cursor over it.
- Dragging a grip shows where the bar will go as a light blue band with a
  blue frame: within 56 px of an edge the bar can take, the band along
  that edge; elsewhere, the window the bar floats in, with its title
  under the pointer. Releasing puts the bar there, also when the pointer
  has left the window (the last place it was seen counts). A bar that
  cannot float, the status bar, goes to the nearer of its two edges.
- The status bar keeps the outer band: other bars dock inside it, and the
  band shown while dragging leaves it out. Along the top the bars come
  below the menu bar.
- A floating bar is a small window with a grey border and a shadow: an
  18 px title with the bar's name at 11 px and a close button, then the
  bar. Dragging the title moves it (near an edge it docks), a double
  click puts the bar back at its first place, and the close button hides
  it (Window > Toolbars shows it again). The floating toolbox is a grid
  two buttons wide whose flyouts open to its right; the floating palette
  is one row 460 px wide.
- The palette on a side is a column: the swatches run downwards and the
  arrows sit at its ends; the expanded palette opens beside the column
  (below a palette along the top). Wherever the palette is, the document
  palette is a row under the drawing window (over it when the palette is
  along the top). The toolbox along the top or the bottom is a row with
  its flyouts opening below or above it.
- Right-clicking a grip, an empty part of a toolbar or of the status bar,
  or a floating bar's title opens the bar's menu: the places the bar can
  take (a dot at the current one), the five bars with a check at each
  one shown, Lock Toolbars and Reset Bar Positions. Window > Toolbars
  has the colour palette, Lock Toolbars and Reset Bar Positions too.
- Lock Toolbars hides the grips, stops every drag and greys the places in
  the menu. Reset Bar Positions puts every bar at its first place and
  shows it; the lock stays as it was.
- The layout and the lock are kept in the preferences (`bars`; in the
  browser, its local storage) as soon as they change; older preferences
  without it start with the first places. Layouts of version 1
  (`bars.version`), whose palette stood at the right edge by default,
  put a palette found there back at the bottom once; a palette anywhere
  else stays, and wherever the user puts it after that is kept
  (`ui::bars::tests::the_palette_starts_at_the_bottom_and_stays_where_it_is_put`).

## Checks

- `ui::propbar::tests::the_object_origin_picks_corners_edges_and_centre`
- `ui::propbar::tests::page_size_changes_one_page_or_all_in_one_step`
- `ui::propbar::tests::outline_widths_read_in_the_ruler_unit`
- `ui::propbar::tests::corner_edits_change_the_selection_in_one_step_or_the_defaults`
- `ui::mod::tests::window_draws_with_every_docker_tool_and_selection`
  draws every tool's bar.
- `ui::overflow::tests::forward_brings_the_first_hidden_item_to_the_start`
- `ui::overflow::tests::back_returns_between_items_and_to_the_start`
- `ui::overflow::tests::areas_holding_items_are_not_cut_points`
- `ui::overflow::tests::a_bar_that_fits_does_not_slide`
- `ui::bars::tests::a_dropped_bar_docks_at_the_nearest_edge_or_floats`
- `ui::bars::tests::bars_only_go_where_they_can`
- `ui::bars::tests::the_drop_zone_leaves_the_status_bar_outside`
- `ui::bars::tests::reset_brings_every_bar_back_and_keeps_the_lock`
- `ui::bars::tests::the_layout_survives_the_preferences`
- `raster::tests::a_canvas_that_moves_renders_again`: moving a bar moves
  the drawing window; the drawing is rendered again for its new place.
