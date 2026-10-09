# Dimension and callout tools

Parallel, Horizontal or Vertical, Angular and Segment dimensions, and
the 3-point callout. Each is placed with clicks (the property bar says
which point is next) and produces a group of the dimension lines and a
text object with the measurement.

## Parallel, horizontal/vertical and segment dimensions

- Three clicks: first point, second point, then where the dimension
  line sits (the offset from the measured segment along its normal).
- Horizontal/Vertical measures the horizontal or vertical distance
  (whichever is larger between the two points); Segment measures the
  segment of the curve under the first click between its two nodes.
- Drawing: extension lines from the points to the dimension line, the
  dimension line, filled arrowheads at both ends; the text is the length
  in the document units with two decimals (`12.70 mm`), 10 pt in the
  current font, placed 1.5 mm above the middle of the line and rotated
  with it.
- The text is a normal text object: it can be moved or restyled; it
  does not update when the drawing changes (dimensions are static, as
  "Convert to curves" leaves them in the target design).

## Angular dimension

- Three clicks: the vertex, a point on the first leg, a point on the
  second leg; the arc radius is the distance to the last click (minimum
  1 mm). The text is the sweep in degrees with one decimal (`45.0°`),
  measured counter-clockwise from the first leg.

## 3-point callout

- Three clicks: the tip (what the callout points at), the elbow, and
  where the text starts. The callout is a two-segment line with an
  arrowhead at the tip, and a text object that opens for typing right
  away.

## Checks

- Given points (0,0) and (50.8,0) and an offset click 10 mm above, then
  the dimension line is at y = 10 and the label reads `50.80 mm` (or
  `2.00 in` with inch units).
- Given a vertex at the origin and legs along +x and +y, then the
  angular label reads `90.0°`.
