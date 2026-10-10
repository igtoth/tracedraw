# Menu bar, standard toolbar and property bar

Interface text is 13 px, the size the target design's menus, labels,
lists and status bar measure in the bundled interface font. Toolbars,
dockers, palettes, the status bar and dialog bodies are `#F4F4F4`; light
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

The View menu follows the target design's: Zoom In, Zoom Out and Zoom
To Fit (with their pictures) and no other zoom commands.

## Dockers

The open docker has a 27 px `#EAEAEA` title bar (its name at 12 px, then
collapse and close in grey at the right) over a white page. The tab
column to its right is 27 px wide: `#EAEAEA` under the title bar and
below the tabs, a `#B2B2B2` line along the docker, grey `#D8D8D8` tabs
with an icon and the name running downwards, the open docker's tab
`#B2B2B2`; the plus button under the tabs adds a docker.

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

Buttons that need a drawing are disabled on the Welcome Screen; hovered
buttons are light blue with a light blue frame.

## Property bar

A 52 px band (and its line); fields stack two to a column, 19 px each
and sharing their middle border, 8 px from the top; fields and lists are
white with a grey border and 13 px text, small pictures before fields
name them (hover for a tooltip), and some fields have spin arrows.

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
  button; polygon points or sides and star sharpness (1 to 99).
- Then the outline: width (Hairline, None, preset widths in the ruler
  unit), line style, start and end arrowheads; then To Front, To Back and
  Convert to Curves.
- Zoom and Pan tools: the zoom box, Zoom In, Zoom Out, Zoom to Selected,
  to All Objects, to Page, to Page Width, to Page Height.

## Checks

- `ui::propbar::tests::the_object_origin_picks_corners_edges_and_centre`
- `ui::propbar::tests::page_size_changes_one_page_or_all_in_one_step`
- `ui::propbar::tests::outline_widths_read_in_the_ruler_unit`
- `ui::propbar::tests::corner_edits_change_the_selection_in_one_step_or_the_defaults`
- `ui::mod::tests::window_draws_with_every_docker_tool_and_selection`
  draws every tool's bar.
