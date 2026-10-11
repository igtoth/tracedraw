# Corners and Join Curves panels

## Corners (Window > Panels > Corners)

Fillets, scallops or chamfers the corners of the selected objects.

| Control | Default | Notes |
|---|---|---|
| Operation | Fillet | Fillet, Scallop or Chamfer |
| Radius | 2 mm | Fillet: the arc tangent to both sides, its centre equidistant from them. Scallop: measured from the corner, the notch is a quarter of that circle |
| Chamfer distance A, B | 2 mm, locked | A back along the side the path comes in on, B along the side it leaves on (the order the path is drawn in); the lock makes B equal to A |
| Apply | | disabled when nothing selected has a corner the values fit |

- A corner is a node where two segments meet at an angle; smooth and
  symmetrical nodes are not corners.
- Every corner is cut, unless the Shape tool is active and nodes of a
  curve are selected: then only those. Rectangles, ellipses, polygons and
  text become curves (the panel never selects their nodes).
- Corners are cut in the order the path is drawn. A corner whose sides
  are not long enough for the cut, counting the cuts already made on
  them, is skipped; the next corner may still fit.
- For a corner of angle `theta` a fillet of radius `r` starts
  `r / tan(theta / 2)` from the corner on each side; on curved sides the
  distance is measured along the curve and the arc leaves along the
  curve's direction. Arcs are cubic Beziers, one per quarter turn.
- While the panel is open, the result is previewed on the drawing as a
  dashed outline in the selection colour.
- One undo step ("Fillet", "Scallop" or "Chamfer").

Rectangles keep their own corner settings when rounded from the property
bar or with the Shape tool (`shape-tools.md`); the panel is for curves.

## Join Curves (Object > Join Curves)

Joins the ends of the selected curves, and the open subpaths inside them.

| Control | Default | Notes |
|---|---|---|
| Mode | Extend | Extend, Chamfer, Fillet, Bezier Curve |
| Gap tolerance | 5 mm | ends further apart are not joined |
| Radius | 2 mm | Fillet only |

- The nearest pair of free ends within the gap is joined first, then the
  next, until no pair is left. The two ends of one subpath close it.
- Extend: both ends carry on along their directions to where they meet;
  a straight end moves there, a curved one gets a straight piece. When the
  directions do not meet ahead of both ends, a straight line joins them.
- Chamfer: a straight line between the ends.
- Fillet: as Extend, then the corner is rounded with the radius (skipped
  when the sides are too short).
- Bezier Curve: a curve leaving and arriving along the ends' directions,
  handles a third of the gap long.
- The joined curve replaces the curve selected last and keeps its fill
  and outline; the other curves are removed. One undo step.
- Nothing within the gap: nothing changes and the status bar says so.

## Checks

- `tracedraw_core` `corner_cut::tests` (areas of each cut, skipped
  corners, smooth nodes, chosen nodes, curved sides, bad values)
- `tracedraw_core` `join::tests` (each mode, gap, closing, several
  pieces into one closed outline)
- `corners::tests::the_docker_cuts_rectangles_as_curves_in_one_step`
- `corners::tests::the_shape_tool_limits_the_cut_to_chosen_nodes`
- `corners::tests::join_curves_keeps_the_last_selected_curve`
