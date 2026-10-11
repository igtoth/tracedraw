# Shape tools

Rectangle, 3-point rectangle, Ellipse, 3-point ellipse, Polygon, Star,
Complex star, Spiral, Graph paper, Common shapes and Action Lines. All of them
draw by dragging a box; the object takes the toolbox defaults (no fill,
a 0.2 mm black outline) unless the Properties panel set other defaults.

## Live preview

While the button is held the object itself follows the pointer, built
with the current property bar settings (corner radius, arc angles,
points and sharpness, revolutions, rows and columns, the chosen common
shape, action lines): the same geometry the release creates. It is drawn
in the colour and width of the default outline, at least one pixel wide
(the selection colour when new objects get no outline). Crop, zoom and
text frame drags keep a plain rectangle. After the release the new
object is selected and shows the eight handles and the centre marker
whatever tool is active.

## Drag modifiers (every box tool)

- Ctrl constrains the box to a square (circle, regular polygon).
- Shift grows the box from the start point as its centre; it is read
  while dragging, so it can be pressed or released mid-drag.
- Snapping applies to the start and current points (see `snapping.md`).
- A drag shorter than 0.05 mm in either direction creates nothing.

## Rectangle (F6)

- New rectangles take the corners set on the property bar (or in Tools >
  Options > Toolbox > Rectangle tool) while nothing is selected; default
  square, round, relative scaling on.
- Corner style: Round (a quarter circle), Scalloped (a quarter circle
  centred on the corner, cutting inwards) or Chamfered (a straight cut).
  The size is the radius, or for a chamfer how far the cut starts from
  the corner.
- Four sizes: top left, top right, bottom left, bottom right (the bar
  shows them as two columns, left corners then right ones). With Edit
  Corners Together (the lock, on by default) a size typed in one field
  goes to all four. A corner is at most half the shorter side.
- Relative Corner Scaling (on by default): the corners scale, or
  stretch, with the rectangle. Off: they keep their size on the page
  when the rectangle is resized, and stay circular when it is stretched.
  Turning it on or off keeps the corners as they look. The size is
  measured against the rectangle's own scaling: scaling a group scales
  its rectangles' corners, and ungrouping keeps them as they look.
- The sizes on the bar are page sizes in the ruler unit; with relative
  scaling on a stretched rectangle shows the smaller of its two scales.
- Double-clicking the tool button draws a page frame: a rectangle the
  size of the page, selected.

## Ellipse (F7)

- Property bar: ellipse, pie or arc, with start and end angles (degrees,
  counter-clockwise from +x) and a direction toggle. Default: ellipse.

## Polygon (Y) and Star

- Points: default 5, range 3..500 (property bar).
- The first vertex is at the top centre. The dragged box is the
  polygon's own bounding box: the model keeps the ellipse the outer
  vertices lie on, so the ellipse is computed from the unit polygon's
  vertex bounds `(x0, x1, y0, y1)` as `rx = w / (x1 - x0)`,
  `ry = h / (y1 - y0)`, centre `(left - x0 rx, bottom - y0 ry)`.
- Star sharpness: default 0.5 (0 = polygon, 1 = thinnest spikes).
- The Star tool's property bar starts with the Star and Complex Star
  buttons, which pick what it draws. A complex star (default 9 points,
  sharpness 2) joins each vertex to the one `sharpness + 1` further on,
  so its sides cross; points 5..500, sharpness 1..`(points - 1) / 2 - 1`
  (at least 1). When that step shares a factor with the points the star
  is several closed subpaths (6 points: two triangles). Filled even-odd,
  its middle stays empty. Its vertices are a polygon's, so the dragged
  box fits them the same way; the Shape tool shows the vertices only and
  dragging one grows or shrinks the star about its centre. `.cdr` files
  get it as a curve.
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

## Action Lines

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
- `geometry::tests::complex_stars_cross_their_sides` (pentagram winding
  2 in the middle, two triangles for 6 points, sharpness limits) and
  `ui::propbar::tests::the_star_tool_draws_complex_stars_that_fill_the_drag`.
- Given a 40 x 20 mm rectangle with 5 mm corners, then it loses
  (4 - pi) 25 mm2 with round corners, 25 pi with scalloped ones and 50
  with chamfered ones (`document::tests::corner_styles_cut_the_expected_area`).
- Given fixed 4 mm corners on a rectangle scaled 3 x 2, then the corners
  still cut (4 - pi) 16 mm2
  (`document::tests::fixed_corners_keep_their_size_when_scaled`).
- `ui::propbar::tests::corner_edits_change_the_selection_in_one_step_or_the_defaults`,
  `ui::propbar::tests::the_rectangle_tool_draws_with_the_default_corners`.
