# Coordinates docker

Window > Dockers > Coordinates. Draws a new object, or replaces the
selected one, from typed coordinates, with a live preview: the drawing
shows the object as a dashed outline in the selection colour and marks
its origin point (a polygon's centre, a line's start, the chosen point of
a multipoint curve) with a blue node.

## Objects

The buttons along the top pick the object: Rectangle and Square behind
one flyout, Ellipse and Circle behind another, Polygon and Regular
polygon behind a third, then Star, Complex star, 2-point line and
Multipoint curve. The chosen object's name is shown under the buttons.

| Object | Fields |
|---|---|
| Rectangle, Ellipse | Position: the origin point (3 x 3 selector, centre by default) and its x, y; Object size: width and height (diameters), Set proportional; Angle of rotation (about the origin point); Bounding box: lower-left and upper-right corners, only while the angle is 0 |
| Square, Circle | Position as above; the side or the diameter; Angle of rotation |
| Polygon, Star, Complex star | Points or sides (complex stars 5..500, others 3..500); star sharpness 1..99, complex star sharpness 1..(points - 1) / 2 - 1; Angle of rotation (about the centre); Bounding circle: centre x, y, horizontal and vertical diameters of the ellipse the vertices lie on, Set proportional |
| Regular polygon | Points or sides; Side length (the diameter is side / sin(180° / points)); Angle of rotation; Bounding circle: centre and diameter |
| 2-point line | Start point and endpoint; Line length and Angle of rotation (changing them moves the endpoint) |
| Multipoint curve | The points (number, x, y; click chooses one), the chosen point's x and y, Add point, Delete point, Set point interactively, Auto-close/open curve (closes only with three points or more) |

Positions are shown from the ruler origin, in the document unit; typed
values may carry another unit (see `toolbars.md`). Changing the origin
point keeps the object where it is.

## Set interactively

Each button with a pointer and target waits for the drawing:

- clicks: origin point (polygon centre), lower-left and upper-right
  corners, start point, endpoint, the multipoint curve's chosen point
  (a new one at the end when none is chosen);
- drags: width, height, side, diameter, side length and line length take
  the drag's length; the angle takes its direction.

Snapping applies to the clicked and dragged points. While a drag is
under way it is shown as a dashed line. A right click cancels; clicking
the pressed button again cancels too. The docker says which kind of
input it waits for.

## Create and replace

- Create object adds the object with the default fill and outline on the
  active layer and selects it (one undo step, "Create Object").
- Replace object is available when one rectangle, ellipse, polygon or
  curve is selected: it becomes the object the fields describe and keeps
  its fill, outline, name and place in the stacking order; envelopes and
  perspective are dropped, as are rotations and skews the fields do not
  hold (one undo step, "Replace Object").
- Selecting an object reads it into the fields: rectangles (squares when
  the docker shows squares and the sides match), ellipses without an arc
  (circles likewise), polygons, stars and complex stars, curves made of
  straight segments (two points open: a line; otherwise a multipoint
  curve). Size comes from the object's scale, the angle from its x axis.

## Keyboard

With the point list focused (click it): Insert adds a point after the
chosen one, Delete removes the chosen one, Up and Down choose the
previous or next point. Double-clicking a point chooses it and waits for
a click on the drawing.

## Checks

- `coords::tests::a_rectangle_turns_about_its_origin_point`
- `coords::tests::polygons_stars_and_lines_from_their_fields`
- `coords::tests::multipoint_curves_add_delete_and_close`
- `coords::tests::objects_load_into_the_fields`
- `coords::tests::create_and_replace_are_one_step_each`
- `coords::tests::every_object_draws_from_the_defaults`
- `coords::tests::picks_wait_for_a_click_or_a_drag`
- `ui::mod::tests::window_draws_with_every_docker_tool_and_selection`
  draws the docker.
