# Toolbox

- One button per flyout, top to bottom, with
  separator lines after the Shape, Zoom, Polygon, Text, Connector,
  Transparency and Fill groups (before the "+"). A button shows the tool last used from its flyout
  (kept in the settings); the active tool's button is tinted.
- Buttons with a flyout have a small arrow at the bottom-right. Clicking
  the arrow, right-clicking, dragging, or pressing and holding the button
  for 0.4 s opens the flyout; a click elsewhere picks the tool the button
  shows. While a flyout is open, hovering another flyout button opens that
  one instead; moving away from the toolbox or clicking elsewhere closes
  it.
- A flyout lists its tools vertically: a check mark beside the tool the
  button shows, the icon, the name and the shortcut at the right.
  Separators split Pick and Freehand Pick from Free Transform; Crop, Knife
  and Segment Delete from Eraser; Polygon, Star, Spiral and Common
  Shapes from Action Lines and Graph Paper; the dimension tools from the 3-point
  callout; the connectors from Anchor Editing; Interactive and Area Fill
  from Mesh Fill.
- Tooltips: "<Name> tool (<shortcut>)" in bold, then what the tool does.
  No tooltip shows while a flyout is open.
- Double-clicking a button: Pick selects all objects, Zoom fits all
  objects, Rectangle adds a page frame.
- The "+" at the end lists every flyout with a check box to show or hide
  it (the Outline flyout is hidden by default) and Reset toolbox.
- The toolbox starts docked on the left. Its grip (the dots above the
  first button) drags it to another edge, where it is a column or a row,
  or into a floating window two buttons wide (see "Moving the bars" in
  `toolbars.md`). Flyouts open away from the edge the toolbox is on.

## Checks

- `ui::toolbox::tests::flyout_groups_can_be_hidden_and_reset`
- `ui::toolbox::tests::separators_split_the_reference_subgroups`
- `ops::smoke_tests::toolbox_groups_remember_the_last_tool_used`

## The pointer over the drawing

Each tool's pointer says which tool is active (code: `ui/cursors.rs`):

| Tools | Pointer |
|---|---|
| Pick, Freehand Pick | the arrow, with resize, move, turning and slant arrows over handles and the selection (see `pick-tool.md`) |
| Shape | a solid arrow marking nodes, control points and segments (see `shape-tool.md`) |
| Zoom | a magnifier with a plus (a minus with Shift) |
| Pan, and any tool while the middle button pans | an open hand, closed while dragging |
| Colour and Attributes Eyedropper, palette sampling | a pipette |
| Area Fill | a paint bucket |
| Text | the text I-beam over text (a click places the caret there), elsewhere the cross hair with the tool's icon |
| Eraser | the cross hair inside the nib's outline at its size on screen (a circle, a square for the square nib) |
| Smooth, Smear, Twirl, Attract and Repel, Smudge, Roughen | the cross hair inside a circle of the brush size |
| Freehand, Bezier, Pen, B-spline, Polyline, 2-point line | the cross hair with the tool's icon; above and right of it a node with a line going on over an end node of the selected open curve (a line from there continues it), a ring over the first node of the curve being drawn once it has three nodes (a click closes it) |
| Interactive Fill | the move arrows over a handle of the selected object's fountain fill (and while one is dragged), else the cross hair with the tool's icon |
| Every other tool (shapes, dimensions, connectors, effects, fills, Crop, Knife, Table, ...) | the cross hair with the tool's icon at its lower right on a light patch |

The cross hair, the arrows and the I-beam are the system's own; the
icons, nibs and drawn pointers are painted by TraceDraw above the
drawing. Nib outlines stay between 2 and 400 points on screen.

## Clicks with drawing tools

With the drawing and shaping tools whose click has no job of its own
(Rectangle, Ellipse, Polygon, Star, Graph Paper, Action Lines, Spiral,
Common Shapes, Table outside a table, Knife, Brush Strokes, the connectors, Blend, Block Shadow, Shape Recognition, Sketch
and the shape brushes Smooth, Smear, Twirl, Attract and Repel, Smudge and
Roughen), a click selects the same way the Pick tool does: the object
under the pointer becomes the selection, Shift+click adds or removes it,
Ctrl+click reaches one group level further in, and a click on empty
space lets go of the selection (Shift+click there keeps it). The tool
stays active. A drag still does the tool's work, even when it starts on
top of an object.

Tools whose click already does something keep it: the line tools
(Freehand, Bezier, Pen, B-spline, Polyline, 2-point line, the 3-point
tools) place points, Text places the caret, the fill tools, eyedroppers,
Zoom, Crop, Eraser, Segment Delete, Area Fill, the dimension and callout
tools and Variable Outline keep their own clicks, and the effect tools
that already selected on a click (Free Transform, Drop Shadow,
Transparency, Envelope, Extrude, Distort, Contour, Mesh Fill, Anchor
Editing, Painterly Brush) go on doing so.

Check: `pick_drag_tests::every_click_selecting_tool_selects_on_click`,
`pick_drag_tests::a_click_with_a_drawing_tool_selects_and_a_drag_draws`,
`pick_drag_tests::shift_click_with_a_drawing_tool_adds_and_takes_away`,
`pick_drag_tests::line_tools_still_place_points_on_objects`,
`pick_drag_tests::a_click_outside_a_table_selects_with_the_table_tool`.
