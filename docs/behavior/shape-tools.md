# Shape tools

Rectangle, 3-point rectangle, Ellipse, 3-point ellipse, Polygon, Star,
Complex star, Spiral, Graph paper, Common shapes and Impact. All of them
draw by dragging a box; the object takes the toolbox defaults (no fill,
black hairline outline) unless the Properties docker set other defaults.

## Drag modifiers (every box tool)

- Ctrl constrains the box to a square (circle, regular polygon).
- Shift grows the box from the start point as its centre; it is read
  while dragging, so it can be pressed or released mid-drag.
- Snapping applies to the start and current points (see `snapping.md`).
- A drag shorter than 0.05 mm in either direction creates nothing.

## Rectangle (F6)

- Corner radius from the property bar, default 0 (shared by all four
  corners; the Shape tool drags corners individually).
- Double-clicking the tool button draws a page frame: a rectangle the
  size of the page, selected.

## Ellipse (F7)

- Property bar: ellipse, pie or arc, with start and end angles (degrees,
  counter-clockwise from +x) and a direction toggle. Default: ellipse.

## Polygon (Y) and Star

- Points: default 5, range 3..500 (property bar).
- Star sharpness: default 0.5 (0 = polygon, 1 = thinnest spikes);
  Complex star uses the same outline with the even-odd rule, so the
  centre is hollow.
- Polygons are symmetric: moving one node with the Shape tool moves its
  mirrored counterparts (see `shape-tool.md`).

## Spiral (A)

- Revolutions: default 4 (1..100). Symmetric (constant pitch) or
  logarithmic (radius grows as e^(4t), normalised to the box) spirals.
- Sampled at 48 points per revolution and smoothed; an open curve.

## Graph paper (D)

- Rows x columns: default 4 x 3. The result is a group of rows x columns
  rectangles that fill the box exactly.

## Common shapes

- Right arrow, heart, diamond, banner, callout, cross, lightning,
  triangle; picked on the property bar; the glyph scales to the box.

## Impact

- Lines: default 12 (2..500), parallel (default) or radial. Lengths are
  pseudo-random from a fixed seed, so the same box gives the same
  object; the lines are grouped.

## Checks

- Given a drag from (10,10) to (30,20) with Ctrl, then the rectangle is
  20 x 20 mm.
- Given the same drag with Shift, then the rectangle spans (-10,0) to
  (30,20) (centre at the start point).
- Given a 5-point star with sharpness 0.5 in a 40 mm square, then its
  bounds are 40 x 40 mm and it has 10 nodes.
