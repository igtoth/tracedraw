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
Dragging with Interactive Fill on an object (anywhere inside it, filled
or not) sets a linear fill from the press point to the release point; a
click gives the default fill; Ctrl constrains the angle to 15 degree
steps; dragging a palette colour onto a stop recolours it; dragging a
colour onto the fill line adds a stop.

Property bar (Interactive Fill): the fill type (None, Uniform, Fountain,
Pattern, Texture, Mesh), then the type's fields: the colour for a
uniform fill; for a fountain the kind (Linear, Radial, Conical, Square),
the start and end colours, the angle and the edge pad. With a selection
the fields edit it; with nothing selected they set the default fill for
the next object.

Handles on the canvas (Interactive Fill, fountain-filled selection): a
linear fill shows its axis as a dashed line between a start node and an
end node (squares in the two colours) at `c -/+ h (cos a, sin a)` with
`h = (w |cos a| + h |sin a|) / 2`; dragging either node turns the axis
about the centre and sets the angle. Radial, conical and square fills
show one node at `c + offset * (w, h) / 2`; dragging it sets the centre
offset, clamped to the bounds.

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
- Given a 40 x 20 mm rectangle with a 0 degree linear fill, then the
  handles are at (0, 10) and (40, 10); dragging the end node to (20, 30)
  makes the angle 90 degrees.
- Given a radial fill, dragging the centre node to (30, 15) on that
  rectangle sets the offset to (0.5, 0.5); beyond the edge it clamps to 1.
