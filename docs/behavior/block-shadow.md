# Block Shadow

Live effect (`Effect::BlockShadow { offset, color, gap }`) drawn below
the object: a solid extrusion of the outline along `offset`.

## Geometry

- The object's outline is flattened (0.05 mm) and its vertices kept
  exactly, so corners stay sharp; outlines with more than 400 vertices
  are resampled to 400.
- The shadow is the union of the back face (the outline moved by
  `offset`) and one quadrilateral per outline segment swept along
  `offset`. The union is one filled path with no outline, in the shadow
  colour.
- Gap: when greater than 0, the outline grown by `gap` mm is trimmed
  from the shadow, which leaves a clear band between object and shadow.

## Property bar

- Depth: `|offset|` in mm. Direction: angle of `offset` in degrees
  (0 = right, counter-clockwise positive). Colour (default 60 % black,
  CMYK 0 0 0 60). Gap in mm (default 0). Clear Block Shadow removes the
  effect.
- Editing a field with a shadowed object selected rewrites its effect;
  with nothing shadowed the values become the defaults for the next
  drag.

## Interaction

- Drag from an object: the object is selected and the shadow offset
  follows the pointer from the press point; every move replaces the
  previous block shadow.

## Checks

- Given a 20 mm square at the origin and offset (5, -5), then the shadow
  bounds are (0, -5) to (25, 20) and the point 0.2 mm inside the far
  corner is covered.
