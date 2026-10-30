# Snapping

Snapping pulls the pointer, and the objects you move, onto exact points:
nodes, the middles of segments, the centers of shapes, guidelines, the grid
or the page. It makes it easy to join lines and line objects up without
zooming in.

## Turning snapping on and off {#on-off}

- [[menu.view > menu.view.snap_to]], or the Snap To list on the standard
  toolbar, turns snapping to [[menu.view.snap_pixels]],
  [[menu.view.snap_document_grid]] (<kbd>Ctrl+Y</kbd>),
  [[menu.view.snap_baseline_grid]], [[menu.view.snap_guidelines]],
  [[menu.view.snap_objects]] (<kbd>Alt+Z</kbd>) and [[menu.view.snap_page]] on
  and off. At first objects, guidelines and the page are on.
- [[menu.view.snap_off]] (<kbd>Alt+Q</kbd>) turns all snapping off and on
  again.

## What objects snap to {#modes}

When snapping to objects is on, these points attract the pointer within the
snapping radius (10 screen pixels at first). Each shows its mark and, with
screen tips on, its name.

| Mode | Points |
|---|---|
| Node | the nodes of outlines, the corners of rectangles, the points of polygons |
| Intersection | where two outlines cross |
| Midpoint | halfway along each segment |
| Quadrant | the top, bottom, left and right of ellipses |
| Tangent | where a line you draw touches an ellipse or curve |
| Perpendicular | where a line you draw meets a segment at a right angle |
| Edge | the nearest point of an outline |
| Center | the center of shapes, bitmaps and tables |
| Text baseline | the baseline of a line of text |

Points win over guidelines, guidelines over the grids, and edges come last.
The modes, the radius and the marks are set on the [[options.snapping]] page
of the options; see [TraceDraw options](../customize/options-app.md#snapping).

## Snapping while moving {#moving}

When you start moving a selection with the [[tool.pick]] tool on one of its
points (a node, a midpoint or the center), that point is carried onto the
other object's point or edge. Otherwise the edges and center of the
selection's box snap to guidelines, grids, the page and other objects. The
handles snap while you resize.

## Dynamic and alignment guides {#dynamic-guides}

- [[menu.view > menu.view.dynamic_guides]] (<kbd>Alt+Shift+D</kbd>) shows
  temporary guides at 0, 45, 90 and 135 degrees through the points you pass
  over, so you can draw along them.
- [[menu.view.alignment_guides]] (<kbd>Alt+Shift+A</kbd>) shows guides when
  the object you move lines up with others.
