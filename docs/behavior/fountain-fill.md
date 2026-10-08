# Fountain fill

## Controls and defaults

| Control | Default | Range |
|---|---|---|
| Type | Linear | Linear, Radial (elliptical), Conical, Square |
| Stops | 2: colour 0% = object colour or black, 100% = white | 2..64 |
| Angle | 0 (left to right, counter-clockwise positive) | -360..360 |
| Edge pad | 0% | 0..49% |
| Centre offset (radial, conical, square) | 0, 0 | -100..100% of half the bounding box |
| Steps (display) | 256 | 2..256 |
| Mirror, repeat, reverse | off | |

Shortcut: F11 opens the fill editor; G is the Interactive Fill tool.
Dragging with Interactive Fill on an object sets a linear fill from the
press point to the release point; Ctrl constrains the angle to 15 degree
steps; dragging a palette colour onto a stop recolours it; dragging a
colour onto the fill line adds a stop.

## Formulas

Bounding box `b` of the object in page space, centre `c`.

- Linear: axis from `c - h*(cos a, sin a)` to `c + h*(cos a, sin a)` with
  `h = (w*|cos a| + h*|sin a|) / 2`; `t = projection of p on the axis,
  normalised 0..1`.
- Radial: `t = |p - (c + offset)| / (max(w, h) / 2 * sqrt 2)`.
- Conical: `t = ((atan2(p - c') - a) mod 2pi) / 2pi`, mirrored so the
  colour at 0 and 1 is the same (`t = 1 - |2t - 1|`).
- Square: `t = max(|dx|, |dy|)` in the frame rotated by `a`, normalised
  by half the diagonal.
- Edge pad `e` remaps stop positions to `e + pos * (1 - 2e)`.
- Colour at `t` is a linear interpolation in RGB between the two stops
  around it; stops are sorted by position; equal positions give a hard
  edge.

## Checks

- Given a 100x10 mm rectangle with stops black 0%, red 50%, white 100%
  and angle 0, when rendered at 1 px/mm, then pixel (50, 5) is red
  within 16/255 and pixel (2, 5) is black, pixel (97, 5) is white.
- Given a conical fill, when sampled at two angles 180 degrees apart,
  then the colours differ.
- Given a PDF export of a 3-stop fill, then the shading uses a
  stitching function with one bound at 0.3.
