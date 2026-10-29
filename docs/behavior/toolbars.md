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
`#B2B2B2`; the three-dot button under the tabs lists every panel in the
groups of Window > Panels with a box ticked at each one in the column
(ticking opens the panel in front, unticking closes it). The pointer
over a tab shows a close cross in a slot at the tab's top; a click on
it closes that panel (as the title bar's close and the tab's menu do),
a click lower on the tab brings the panel to the front
(`ui::dockers::tests::a_docker_tab_closes_with_the_cross_at_its_top`).

### Panels anywhere

- The button at the end of each tab column is three small dots in a row
  (22 px tall, an 18 px grey box behind it while the pointer is over it;
  tip `docker.add_docker`, "Show or hide panels"). Its menu lists the
  panels in the groups of Window > Panels, with separators between the
  groups and the effect panels after the text group: each row is the
  check box (ticked while the panel is open anywhere), then the panel's
  picture (the one its tab shows; a blank slot of the same width where it
  has none, so the names line up), then its name
  (`ui::menus::check_pic`). Ticking a row opens the panel in front of
  that column (`App::move_docker`), unticking closes it
  (`App::close_docker`). Taller than the window, the list scrolls, as
  every menu body does
  (`pick_drag_tests::the_three_dot_button_lists_the_panels_with_their_pictures`).
- Panels live in one of three places (`App::docker_place`): the default
  column at the right of the window (`open_dockers`, `docker_tab` in
  front, `show_dockers`; at the left in a right-to-left language), the
  column at the other side (`left_dockers`, `left_tab`,
  `show_left_dockers`; it exists only while it holds a panel) and
  floating windows (`floating_dockers`, one panel each). Every panel
  starts in the default column.
- Dragging a tab, or the title bar of a panel (a column's or a floating
  one's), moves the panel: a light blue band shows where it goes, within
  56 px of a side the column there (the strip and a 340 px panel),
  elsewhere a window the size a panel floats at (340 by 480) with its
  top left corner where the grab point puts it. Releasing puts the panel
  there, in front of the column it joins (the column it leaves shows the
  tab next to it, or closes when none is left); a release outside the
  window counts at the last place the pointer was seen. The tab's menu
  and the title's menu (right click) have Dock at Right, Dock at Left,
  Floating and Close panel.
- A floating panel is a window with a grey border and a shadow: the
  panel's title bar (its name and close; no collapse) over its page,
  resizable at its edges; where it is and how large are kept.
- A panel brought to the front from a menu, a shortcut or a tool while it
  lies in the other column or floats is put in front where it is, and
  the default column stays as it was; a panel open nowhere joins the
  default column (`App::open_docker`, `App::reconcile_dockers`). Window >
  Panels and the Help > Hints check show a panel as on when it is in
  front of an open column or floating (`App::docker_shown`); the plus
  button of a column opens a ticked panel in front of that column.
- The places are kept in the preferences (`dockers`: `open`, `front`,
  `left`, `left_front`, `left_shown`, `floating` with each window's
  corner and size); older preferences with the column alone read as
  before.
- Checks: `app::tests::dockers_move_between_the_columns_and_float`,
  `ui::dockers::tests::a_docker_dragged_to_a_side_docks_there_and_the_window_draws_them_all`.

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
into it (`ui::field::tests`). Every number field has a pair of spin
arrows at its right (11 px): a click on one steps the value by a unit
(or the step the field was given), as does the mouse wheel over the
field; Shift steps ten times as much, Ctrl a tenth; a whole-number field
stays whole and the range holds (`ui::field::tests::arrows_and_the_wheel_step_the_value`).
The wheel steps once a notch; parts of a notch from a touchpad or a fine
wheel add up to one step. The wheel over a field (or the outline width
box, the font list, the page tabs) belongs to it: the bar or panel
around it does not slide or scroll with that notch, on the frames the
smoothed scroll of the notch lasts as well, so the field stays under the
pointer and notch after notch steps it (#783,
`ui::field::tests::the_wheel_steps_a_field_in_a_long_bar_without_sliding_the_bar`,
`ui::field::tests::the_wheel_steps_a_field_in_a_scrolling_panel_without_scrolling_it`,
`ui::propbar::tests::the_wheel_over_the_outline_width_in_a_long_bar_keeps_the_bar_still`).
Every step applies at once, one undo step each, whether or not the box
holds the focus: the rotation angle, the scale percentages, the arc
angles, the dimension text size and line width, the corner radii and the
Transform toolbar's fields took only the first click after the box was
clicked into (the one that took the focus from it) and then none
(`ui::propbar::tests::every_click_on_the_rotation_arrows_turns_the_selection`).

- Pick tool, nothing selected (page bar): page size list; page width and
  height; fit page to drawing (Layout's command, pale on an empty page);
  portrait, landscape; all pages or current page (which pages a size
  change applies to, one undo step); Units; nudge distance; duplicate
  distances; treat all objects as filled.
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
  Convert to Curves. The mouse wheel over the closed width box steps the
  selection's outline to the next width of the list up (wheel up) or
  down, stopping at None and at the widest; a width between two of the
  list goes to the one above or below. One step per notch: a wheel event
  of a notch or more counts as one notch whatever its size (browsers send
  a notch as 100 points or 3 lines), parts of a notch from a touchpad or
  a fine wheel add up until they make one (50 points make a notch), and
  what has added up is dropped when the wheel turns the other way or the
  pointer leaves the box. One undo step a notch. With nothing selected
  the wheel does nothing (#663).
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
the mouse wheel over the bar (away from a number field) slides it too. The items stay as they are on
the bar and work as there (the bar is drawn once). A bar that fits shows
no buttons; the slide is worked out again when the window changes size
(`ui::overflow::tests`).

Dialogs taller (or wider) than the window keep their size inside a
window that fits the screen, and their body scrolls; dockers scroll as
before.

## Moving the bars

The toolbox, the colour palette, the standard toolbar, the property bar,
the Text, Zoom and Transform toolbars, the time tracker and the status
bar move (`ui/bars.rs`).

| Bar | Places | First place |
|---|---|---|
| Toolbox | top, bottom, left, right, floating | left |
| Colour palette | top, bottom, left, right, floating | bottom (on a side: the outermost column, beyond the dockers) |
| Standard toolbar | top, bottom, floating | top, first row |
| Property bar | top, bottom, floating | top, second row |
| Text, Zoom, Transform toolbars, time tracker | top, bottom, floating | top, a row each under the property bar (hidden at first) |
| Status bar | top, bottom | bottom |

- Toolbars in rows (#837, #855): the toolbars docked along the top or
  the bottom lie in rows, top to bottom; at first each has a row of its
  own. A row is a band as high as its highest toolbar (36 px, 53 px with
  the property bar), the toolbars side by side in it, each centred in
  the band with its grip first. Each toolbar takes the length its
  buttons need while the row has room, the last one the rest; when they
  do not all fit, the first ones keep their length and the later ones
  get what is left (at least 90 px each while the row holds that), the
  part they cannot show sliding behind their arrows as a bar longer than
  its room does (`ui::bars::share_row`).
- Dragging a toolbar's grip over a row shows a blue line where it would
  go: between the toolbars whose middles lie either side of the pointer
  (or at the row's end). Releasing puts it there, beside the others.
  Over the upper or lower 7 px of a row the mark is a band across the
  window on that line, and the toolbar gets a row of its own there.
  Away from the rows a toolbar docks at the nearest edge or floats as
  below; one coming from elsewhere takes a row of its own at the edge
  of the window (the topmost row along the top, the lowest along the
  bottom). Hidden toolbars keep their places in their rows and come
  back to them. The rows are kept in the preferences (`bars.rows`: each
  toolbar's row and place) and Reset Bar Positions gives each toolbar
  its first row again
  (`ui::bars::tests::toolbars_share_rows_and_keep_their_places`,
  `ui::bars::tests::a_toolbar_dropped_on_a_row_joins_it_or_starts_one`,
  `ui::bars::tests::a_row_shares_its_length`,
  `pick_drag_tests::a_toolbar_dragged_onto_another_row_shares_it`).

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
  take (a dot at the current one); every bar with a check at each one
  shown, as Window > Toolbars lists them (`menus::toolbars_list`: the
  menu bar, always on; the standard toolbar, the property bar, the
  toolbox, the colour palette, the AI prompt bar, the status bar, the
  Text, Zoom and Transform toolbars, the time tracker); Customize; Lock
  Toolbars and Reset Bar Positions. Window > Toolbars has the same list,
  Lock Toolbars and Reset Bar Positions. A right click on the buttons and
  boxes of a toolbar, docked or floating, opens the same menu (#835). The
  menu stays open after the button comes up, until an entry is picked, a
  click lands outside it or Esc is pressed; each click opens one menu,
  shown once a frame (`pick_drag_tests::a_right_click_on_any_toolbar_keeps_the_bar_menu_open`, #994).
- Customize holds: Default Button Size (Small, Medium, Large, Small at
  first: the buttons of every bar are 24, 32 or 40 px square, the
  property bar's 21, 28 or 35; `settings.button_size`, `theme::set_button_sizes`); Toolbox
  Toolbar (Reset to Default: every flyout shown as at first, each button
  on its first tool, the default size and icons only; Button Size:
  Default, which follows the default button size, or its own Small,
  Medium or Large, `settings.toolbox_button_size`; Image Only, Caption
  Only or Caption to Right, `settings.toolbox_captions`: with captions
  every toolbox button is as wide as the widest tool name of the flyouts
  shown, plus 6 px each side, so the column keeps one width whichever
  tools the buttons show, and the name is drawn at 12 px to the right of
  the icon or in its place); Workspace (Export Workspace, Import
  Workspace, the same as Window > Workspace); and Customize..., which
  opens Options > Customization > Command Bars.
- The buttons of the Standard and Zoom toolbars are lists the user can
  change (`ui/bar_items.rs`, `settings.bar_items`, kept only when changed;
  in an exported workspace too). In customize mode, with Tools > Options >
  Customization open (its dialog leaves those two bars open to the
  pointer) or with Alt held, a press on a button no longer runs it: the
  button is outlined under the pointer, a drag carries its picture, a
  line marks the gap it would go to, and on release it moves there (along
  its bar or to the other one) or, released off both bars, goes away.
  Commands and tools dragged from the Commands page join the bar at the
  gap; a command already on that bar only moves. Added buttons show the
  command's menu picture (its name when it has none) and run it as its
  menu item would; a tool's button selects the tool and stays pushed in
  while it is the tool. Reset (Command Bars page) and Reset Bar Positions
  bring the first buttons back. The Zoom toolbar shows the zoom box and
  picture buttons, as on the Zoom tool's property bar: Zoom In, Zoom Out,
  Zoom to Selected, to All Objects, to Page, to Page Width, to Page
  Height (#563). Checked by
  `pick_drag_tests::toolbar_buttons_move_and_leave_in_customize_mode` and
  `bar_items::tests`.
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
- `ui::propbar::tests::the_wheel_steps_through_the_outline_widths`,
  `ui::propbar::tests::the_wheel_over_the_outline_width_changes_it` (#663)
- `ui::propbar::tests::corner_edits_change_the_selection_in_one_step_or_the_defaults`
- `ui::mod::tests::window_draws_with_every_docker_tool_and_selection`
- `ui::toolbox::tests::buttons_follow_the_sizes_and_the_captions`
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
