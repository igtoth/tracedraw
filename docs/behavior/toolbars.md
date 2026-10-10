# Standard toolbar and property bar

## Standard toolbar

32 px buttons with 20 px icons in groups split by thin lines:

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

34 px high, so fields stack two to a column; fields and lists are white
with a grey border, small pictures before fields name them (hover for a
tooltip), and some fields have spin arrows.

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
- Then, by the kind of the selected objects: rectangle corner radius;
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
- `ui::mod::tests::window_draws_with_every_docker_tool_and_selection`
  draws every tool's bar.
