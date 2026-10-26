# Shape tools

Rectangle, 3-point rectangle, Ellipse, 3-point ellipse, Polygon, Star,
Complex star, Spiral, Graph paper, Common shapes and Action Lines. All of them
draw by dragging a box; the object takes the toolbox defaults (no fill,
a 0.2 mm black outline) unless the Properties panel set other defaults.

## Live preview

Pressing to draw lets go of whatever was selected at once. While the
button is held the object itself follows the pointer, built
with the current property bar settings (corner radius, arc angles,
points and sharpness, revolutions, rows and columns, the chosen common
shape, action lines): the same geometry the release creates. It is drawn
in the colour and width of the default outline, at least one pixel wide
(the selection colour when new objects get no outline), and over it a
black hairline on a white line twice as wide, so its edge shows on any
outline colour and any drawing under it (#670; the 3-point tools'
preview and their base line too, while it is dragged and while it waits
for the third click). The zoom and text frame drags, and a
box tool's rubber band, are a hairline (blue, black for a text frame) on
a white line twice as wide, so they show over a black fill too (#672).
Crop keeps a plain rectangle. After the release the new
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
  square, round, relative scaling off (the corners keep their size).
- Corner style: Round (a quarter circle), Scalloped (a quarter circle
  centred on the corner, cutting inwards) or Chamfered (a straight cut).
  The size is the radius, or for a chamfer how far the cut starts from
  the corner.
- Four sizes: top left, top right, bottom left, bottom right (the bar
  shows them as two columns, left corners then right ones). With Edit
  Corners Together (the lock, on by default) a size typed in one field
  goes to all four. A corner is at most half the shorter side.
- Relative Corner Scaling (off by default for new rectangles): off, the
  corners keep their size on the page when the rectangle is resized, and
  stay circular when it is stretched in one direction only. On, they
  scale with the rectangle; a stretch in one direction stretches them
  too, so round corners become elliptical.
  Turning it on or off keeps the corners as they look. The size is
  measured against the rectangle's own scaling: scaling a group evenly
  grows its rectangles' corners (they still keep that size afterwards),
  stretching a group stretches them, and ungrouping keeps them as they
  look.
  Drawings saved before this default changed keep what they store: a
  rectangle without the setting in the file scales its corners as it
  did. Rounded rectangles opened from `.cdr` files keep their corner
  size, at the size the corners measure on the page (a corner larger
  than half the shorter side stays so); one the file stretches gets
  round corners of its shorter scale, the stretch going into its size
  unless its outline scales with the object (a skewed one keeps
  corners that follow it)
  (`document::tests::keep_corner_size_leaves_the_outline_and_takes_the_stretch`)
  (`parse::tests::rounded_rectangles_keep_their_corner_size` in the
  .cdr reader). Preferences saved by older versions, which could not store the
  choice, start with it off once.
  Checked by `pick_drag_tests::rounded_corners_keep_their_size_when_relative_scaling_is_off`,
  `pick_drag_tests::new_rounded_rectangles_keep_circular_corners_when_stretched`
  and `settings::tests::rectangle_corners_keep_their_size_by_default_and_after_upgrading`.
- Saving: corners that keep their size are written as one plain radius
  where the format has one. `.cdr` versions with rectangle objects
  (before 15) get a stretched rectangle with one round radius as a
  rectangle of its page size and that radius (the stretch moves out of
  its matrix), unless the stretch also widens an outline that scales
  with the object or the axes are skewed; then, and in later versions,
  the outline goes out as a curve. SVG and PDF write every rectangle as
  the path it draws, so the corners there are exactly as shown
  (`write::tests::corners_one_radius_cannot_hold_are_written_as_curves_and_stretched_fixed_ones_as_rectangles`,
  `tests::corners_that_keep_their_size_survive_native_svg_and_pdf`).
- The sizes on the bar are page sizes in the ruler unit; with relative
  scaling on a stretched rectangle shows the smaller of its two scales.
- Double-clicking the tool button draws a page frame: a rectangle the
  size of the page, selected. With Shift held, the rectangle frames the
  selection as drawn (its effects included); with nothing selected, the
  page.

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

71 shapes in five sets (`common_shapes.rs`), picked on the property bar:
the button shows the shape and its name; a click opens the sets as tabs
(the current shape's set first) over a grid of outlines, nine to a row,
each named in its tooltip. Picking one closes the grid.

- Basic shapes (18): parallelogram, trapezoid, diamond, triangle, right
  triangle, hexagon, octagon, plus, heart, lightning, crescent, ring,
  cylinder, cube, frame, teardrop, smiley face, no symbol.
- Arrows (12): right, left, up, down, left and right, up and down,
  four-way, chevron, pentagon arrow, notched, striped, bent.
- Flowchart (27): process, alternate process, decision, data, predefined
  process, internal storage, document, multiple documents, terminator,
  preparation, manual input, manual operation, connector, off-page
  connector, card, punched tape, summing junction, or, collate, sort,
  extract, merge, stored data, delay, magnetic disk, direct access
  storage, display.
- Banners (8): ribbon, folded ribbon, wave, double wave, scroll,
  explosion, burst, pennant.
- Callouts (6): rectangle, rounded, oval and cloud callouts, line and
  bent line callouts.

Each shape is drawn into the box dragged as one curve object with the
default fill and outline. Parts meant to look separate (a cylinder's top
and body, a cube's faces, a ribbon's tails, the pages behind a multiple
document) are separate subpaths edge to edge, never overlapping, and
holes (ring, frame, the smiley's eyes and mouth, the no symbol's
openings) run against the outline, so they stay open under any fill
rule. Inner details (the bars of a predefined process, the cross of a
summing junction, a callout's leader) are straight two-point lines that
are stroked, not filled. Rounded corners (alternate process, the
terminator's ends, a rounded callout) keep their size in long boxes.

## Action Lines

- Lines: default 12 (2..500), parallel (default) or radial. Lengths are
  pseudo-random from a fixed seed, so the same box gives the same
  object; the lines are grouped.

## Checks

- Given a 3 mm black default outline, while a rectangle or an ellipse is
  dragged out, the outline is painted at its width and the edge line is
  painted after it, on top (#621,
  `pick_drag_tests::the_edge_of_the_shape_being_drawn_shows_over_a_wide_outline`).
- `pick_drag_tests::overlays_are_drawn_on_a_white_line`
- `common_shapes::tests::every_shape_fills_its_box`: every shape stays in
  its box and fills at least 80 % of it both ways, and is named.
- `common_shapes::tests::rounded_corners_keep_their_size_in_long_boxes`:
  a terminator in a 100 x 20 mm box has ends of radius 10 mm; ring and
  frame areas are the outer less the hole.

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
- Given a new rectangle with 5 mm corners stretched to twice its width
  with a side handle, then its corners are still 5 mm circles; with
  relative scaling turned on and the height doubled, they grow.
- `ui::propbar::tests::corner_edits_change_the_selection_in_one_step_or_the_defaults`,
  `ui::propbar::tests::the_rectangle_tool_draws_with_the_default_corners`.
