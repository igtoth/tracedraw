# Draw in Perspective

View > Draw in Perspective shows a perspective grid over the page. While
it is on, every object a drawing tool makes (rectangles, ellipses,
polygons, curves, brush strokes, text frames...) is laid onto the active
plane of the grid. Objects made with the Pick tool (pasted, duplicated,
moved) are left as they are.

## The bar

At the top left of the drawing window:

- Finish: leaves the mode (the grid goes; the objects keep their
  perspective).
- 1-Point, 2-Point, 3-Point: the kind of grid (a new grid of that kind,
  sized to the page).
- The planes: Ground, Left, Right (and Top with 1-Point). The active
  plane's grid lines are strong, the others faint.
- Grid: show the grid lines.
- Depth (5 to 98 %): how far towards the vanishing points the grid
  reaches.
- Put on Plane: lays the selected objects (drawn before, pasted or
  imported) onto the active plane, one undo step each.

## Defaults

- Horizon at 55 % of the page height.
- 1-Point: the vanishing point in the middle of the horizon; the near
  face from 15 % of the width, 70 % of the width wide, 8 % up from the
  bottom and 75 % of the height tall; depth 70 %.
- 2-Point: vanishing points at -35 % and 135 % of the width on the
  horizon; the near corner in the middle, 8 % up; depth 45 %.
- 3-Point: as 2-Point, with the vertical vanishing point at 220 % of the
  height above the corner; the walls' vertical edges lean to it.

## Handles

Drag on the page: the vanishing points (circles; with 2- and 3-Point the
two side points share the horizon), the near bottom corner and the near
top corner (squares; with 1-Point the top corner also sets the width).

## How objects are laid down

Each plane is the projective image of a unit square (`homography`, the
same as the Perspective effect's). The drawn object's box is taken onto
the plane: each side of the box becomes the mean of its two corners'
plane coordinates, giving a rectangle on the plane, whose corners on the
page are the object's Perspective effect (Object > Add Perspective),
so the result is exact and the Shape tool edits its four corners. A box
drawn past a plane's vanishing line is pulled towards the grid until it
fits, and one that lies wholly beyond it stays flat. Drawing the object
and laying it down are one undo step.

## How the Perspective effect maps an outline

The object's box goes onto the four corners by the projective map that
takes its corners there (`live::perspective_in`). The nodes and control
points of the outline are mapped, so every segment stays one segment:
straight sides stay straight and a curve bends as its handles do. Two
neighbouring corners dragged onto one point (a triangle) bring the
handles along that side to the point too, so the outline comes to a tip
there: an ellipse becomes a drop. Corners that meet, or that were dragged
a little past each other (within a fiftieth of the frame), are set a
hair apart in the order of the side across from them; the box could not
be mapped onto three points, and the object stayed flat or, with the
corners crossed, came out as a sliver running off past the frame (#879,
`live::tests::a_perspective_with_two_corners_together_makes_a_tip`). A
frame folded over itself on purpose puts part of the outline past the
horizon: that outline is flattened and mapped point by point instead
(`a_folded_perspective_stays_finite`).

## The Perspective effect on a group

Object > Add Perspective on a group warps every object in it with the
group's four corners, before the objects' own effects (a contour rings
the warped object). Ungroup (Ctrl+U, also Ungroup All) keeps the
perspective: each object takes a Perspective effect at the front of its
stack, the group's projection as seen in the object's space, so the
objects stay drawn as they were and the Shape tool edits each one's
corners. An object that had a perspective first in its stack keeps one:
the two are folded into it. Only the group's perspective is kept; its
other effects go with the group, as before (#674).

## Checks

- Given a group of two rectangles, one scaled, the group turned and
  given a perspective, and the other rectangle with no effect, an
  outside contour, a push and pull distortion or a perspective of its
  own, when it is ungrouped, then each rectangle has one Perspective
  effect, first in its stack, and is drawn where it was in the group,
  its contour or distortion included
  (`command::tests::ungrouping_keeps_the_perspective_on_each_object`,
  core).
