# Blend

Intermediate objects between a start and an end object.

| Control | Default | Range |
|---|---|---|
| Steps | 20 | 1..999 |
| Spacing (on a path) | computed | 0.01..mm |
| Rotation | 0 degrees | -360..360, loop option |
| Colour direction | direct | direct, clockwise, counter-clockwise |
| Acceleration (objects, colours) | 0 (linear) | -1..1 |
| Path | none | any curve, "blend along full path", "rotate all objects" |
| Map nodes, split, fuse start/end | | |

Interactive: drag from the start object to the end object; the start
and end are the two selected objects' order on drop.

## Formulas

For step `k` of `n` (`t = (k) / (n + 1)` with acceleration `t' = t ^ (1 + 2*acc)`):
- Geometry: both paths are resampled to the same node count
  (the larger of the two, nodes mapped by arc length, or by the user's
  node map), then interpolated node by node, including control points.
  Before that the end outline is wound the way the start outline is,
  and its first node is the one whose direction from its own centre is
  closest to the start outline's first node's direction from its centre,
  so equal shapes blend without twisting through the middle whatever
  their relative position.
- Transform: interpolated as translation + rotation + scale (not matrix
  entries), so blends rotate the short way unless "rotation" adds turns.
- Colour: RGB linear for direct; HSB with hue going the chosen way.
- Along a path: step `k` is placed at arc length `t' * L` on the path,
  oriented to the tangent when "rotate all objects" is on.

## Checks

- Given two 10 mm circles 100 mm apart, 9 steps, then the 5th step is
  centred at 50 mm and has a 10 mm diameter.
- Given two equal ellipses, the second down and to the right, every step
  keeps the ellipse's width and height within 1 mm; reversing the second
  outline changes nothing.
- Given red to blue direct with 1 step, then the step is (128, 0, 128).
