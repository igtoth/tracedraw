# Snapping and guides

| Setting | Default |
|---|---|
| Snap to objects | on (Alt+Z), radius 10 px (screen) |
| Snap to guides | on |
| Snap to grid | off (Ctrl+Y); document grid 10 mm both ways through the ruler origin (Document Options > Grid) |
| Snap to baseline grid | off; baselines from the page top (see `rulers-grid-guidelines.md`) |
| Snap to pixels | off; whole pixels of the document resolution from the page corner |
| Snap to page | on: the page's corners, edge midpoints, centre and edges |
| Dynamic guides | off (Alt+Shift+D); angles 0, 45, 90, 135 |
| Alignment guides | off (Alt+Shift+A) |
| Snap modes | all on: node, intersection, midpoint, quadrant, tangent, perpendicular, edge, centre, text baseline |
| Snap location marks, screen tip | on |

Alt+Q turns all snapping off and on.

## Modes

| Mode | Points | Mark |
|---|---|---|
| Node | the nodes of outlines (a rectangle's corners or the ends of its corner cuts, a polygon's vertices); an ellipse's own node (its top, or a pie's or arc's two ends) | square |
| Intersection | where two outlines cross (segments joined end to end meet at a node instead) | cross |
| Midpoint | halfway along each segment (by length) | triangle |
| Quadrant | 0, 90, 180 and 270 degrees on an ellipse, or those an arc covers | diamond |
| Tangent | from where the line being drawn starts: where it touches an ellipse or a curve | circle under a line |
| Perpendicular | from the same start: the foot of the perpendicular on a segment | right angle |
| Edge | the nearest point of an outline | hourglass |
| Center | the centre of rectangles, ellipses, polygons, bitmaps and tables; the area centroid of closed curves | circle with a dot |
| Text baseline | the nearest point of the baseline of a line of artistic or paragraph text | a T on a line |

Groups and clip frames offer their contents' points. The line being drawn
starts at the last node of the curve in progress (Bezier, Pen, Polyline,
2-point line, B-spline) or at the last point a dimension or callout tool
placed.

## Which point wins

Within the radius, a point (node, intersection, midpoint, quadrant,
tangent, perpendicular, centre) wins, the nearest one, an equal distance
going to the mode listed first; then guidelines (horizontal, vertical,
angled), the baseline grid, the document grid and pixels; then the
nearest edge or text baseline. The mark of the point snapped to is drawn
in the selection colour with the mode's name beside it.

Drawing tools snap the pointer. A move with the Pick tool that starts on
one of the selection's points (a node, a midpoint, a quadrant or the
centre within the radius) carries that point onto another object's point
or edge; otherwise the selection's box edges and centre snap to
guidelines, the grids, the page and the other objects' boxes.

A guideline being dragged (or dragged out of a ruler) and the ruler origin
snap to everything except guidelines.

## Checks

- Given a guide at x = 50 mm and a 10 mm radius, when an object's right
  edge is dragged to x = 51 mm, then it lands at 50 mm
  (`snap::tests::a_guideline_catches_an_object_moved_near_it`).
- `snap::tests::drawing_tools_snap_to_object_points_and_show_the_mark`
- `snap::tests::snapping_to_objects_turns_off_but_the_page_stays`
- `snap::tests::a_move_carries_its_snap_point_onto_another_object`
- `snap::tests::lines_snap_to_tangent_and_perpendicular_points_from_their_start`
- `snap_points::tests` (each mode, the page, priorities)
