# Snapping and guides

| Setting | Default |
|---|---|
| Snap to objects | on (Alt+Z), radius 10 px (screen) |
| Snap to guides | on |
| Snap to grid | off (Ctrl+Y); document grid 10 mm both ways through the ruler origin (Document Options > Grid) |
| Snap to baseline grid | off; baselines from the page top (see `rulers-grid-guidelines.md`) |
| Snap to pixels | off; whole pixels of the document resolution from the page corner |
| Snap to page | on: the page's corners, edge midpoints, centre and edges |
| Dynamic guides | off (Alt+Shift+D); angles 0, 45, 90, 135; see `live-guides.md` |
| Alignment guides | off (Alt+Shift+A); edges, centres, the page, equal spacing; see `live-guides.md` |
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
| Center | the centre of rectangles, ellipses, polygons, bitmaps and tables; the area centroid of closed curves | circle with a cross |
| Text baseline | the nearest point of the baseline of a line of artistic or paragraph text | a T on a line |

Groups and clip frames offer their contents' points. The line being drawn
starts at the last node of the curve in progress (Bezier, Pen, Polyline,
2-point line, B-spline) or at the last point a dimension or callout tool
placed.

## Which point wins

Within the radius, a point (node, intersection, midpoint, quadrant,
tangent, perpendicular, centre) wins, the nearest one, an equal distance
going to the mode listed first; then alignment guides and dynamic guides
when they are on (`live-guides.md`); then guidelines (horizontal, vertical,
angled), the baseline grid, the document grid and pixels; then the
nearest edge or text baseline. The mark of the point snapped to is drawn
in black on a white halo, with the mode's name beside it in the
selection colour, also haloed in white (no box), so both read on any
drawing.

Only objects whose bounds come within the radius of the pointer are
looked at. An intersection is found on chords an eighth of the radius
long, cut from the segments near the pointer, and then refined onto both
curves, so it is exact while a page of thousands of curves still snaps
at the screen's rate (#345, `curves_snap_to_their_exact_crossing`).
Within one outline, only the nodes within the radius are gathered and
told apart, and a segment is measured only when the box of its control
points comes within the radius, so a single curve of tens of thousands
of nodes (a combined ornament) costs a few milliseconds a frame, not the
seconds it took when every node was compared with every other (#764,
`snap_points::tests::nodes_near_the_pointer_are_offered_once`,
`pick_drag_tests::a_combined_curve_of_many_nodes_moves_and_snaps_to_its_nodes`;
`frame_times_over_a_combined_curve` measures it).

Drawing tools snap the pointer. A move with the Pick tool that starts on
one of the selection's points (a node, a midpoint, a quadrant or the
centre within the radius) carries that point onto another object's point
or edge; otherwise the selection's box edges and centre snap to
guidelines, the grids, the page and the other objects' boxes. A scale
handle being dragged snaps like the pointer of a drawing tool, leaving
out the selection's own points.

A node dragged with the Shape tool (or by node tracking with the Pick
tool) snaps to other objects and also to what stays put of its own
curve: its other nodes and the segments between two of them, with every
mode (the nodes being dragged, where they started and the segments they
bend are left out; with Ctrl the point kept on its axis snaps, #673).
That geometry is found once a drag, in pieces of at most 16 segments
each with its bounds, and a move looks only at the pieces near the
pointer, so a curve of thousands of nodes drags as fast as without it
(`frame_times_of_a_node_drag_on_a_long_curve`).

A guideline being dragged (or dragged out of a ruler) and the ruler origin
snap to everything except guidelines.

## Checks

- Given a guide at x = 50 mm and a 10 mm radius, when an object's right
  edge is dragged to x = 51 mm, then it lands at 50 mm
  (`snap::tests::a_guideline_catches_an_object_moved_near_it`).
- `snap::tests::drawing_tools_snap_to_object_points_and_show_the_mark`
- `snap::tests::snapping_to_objects_turns_off_but_the_page_stays`
- `snap::tests::a_move_carries_its_snap_point_onto_another_object`
- Given a rectangle converted to curves, when its top right node is
  dragged with the Shape tool to 3 px off its bottom left node, then it
  lands on that node
  (`pick_drag_tests::a_dragged_node_snaps_to_another_node_of_its_own_curve`).
- Given an open curve through (20, 60), (60, 60) and (100, 60), when the
  middle node is dragged right with Ctrl, the pointer 15 px below the
  line 1 mm short of the end, then it lands on the end node
  (`pick_drag_tests::a_ctrl_dragged_node_snaps_to_its_own_curve_on_its_axis`).
- A node dragged 6 px off where it started, or off the middle of a side
  it bends, stays where the pointer puts it
  (`pick_drag_tests::a_dragged_node_does_not_snap_to_itself_or_its_moving_segments`).
- `snap::tests::lines_snap_to_tangent_and_perpendicular_points_from_their_start`
- `snap_points::tests` (each mode, the page, priorities)

## Snap To list

The standard bar's Snap To opens a list of Pixels, Document Grid,
Baseline Grid, Guidelines, Objects and Page, each a check box that turns
that snapping on or off, then Alignment Guides and Dynamic Guides. The list stays open while boxes are ticked; its
X, a click outside or Esc closes it.
