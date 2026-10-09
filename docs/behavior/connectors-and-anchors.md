# Connectors and anchors

Straight, right-angle and rounded right-angle connectors between objects,
and the Anchor Editing tool that places the points they attach to.

## Anchors

Every object has four implicit anchors, the midpoints of its bounding box
sides. Custom anchors are stored in the object data as `anchor.<n>` =
`fx,fy`, fractions of the bounding box (0..1), so they move and scale with
the object and survive save and load without a model change.

## Anchor Editing tool

- Click an object: it becomes the target and its anchors are shown as
  diamonds (side anchors translucent, custom anchors white, the selected
  one in the selection colour).
- Click inside the target: adds a custom anchor at the click.
- Drag a custom anchor: moves it (clamped to the bounds).
- Delete, the property bar's Delete Anchor button, or a double-click:
  removes the selected custom anchor. Remove All Custom Anchors clears
  them.
- Hit radius: 6 screen pixels at any zoom.

## Connectors

A connector drag starts on one object and ends on another. The end points
are:

- the anchor of the start object nearest to where the drag began, when
  the drag began on an anchor (within the hit radius),
- the anchor of the end object nearest to where the drag ended, when it
  ended on one,
- otherwise the closest pair of anchors between the two objects.

Right-angle connectors go from the start anchor to an elbow at the
midpoint of the dominant axis and then to the end anchor; the rounded
variant replaces the two corners by quadratic arcs of radius
`min(3 mm, length / 4)`. The result is a plain open path named
"Connector" with no fill, so it edits like any curve.

While a connector is dragged the anchors of both objects are shown, and
the rubber-band line starts at the anchor that will be used.

## Checks

- Given a custom anchor at the top-right corner of a 10 mm square, when
  the square is scaled by 2, then the anchor is still at the top-right
  corner (fractions, not millimetres).
- Given that anchor and a second object to the right, when a connector is
  dragged from the anchor to the second object, then it starts at the
  anchor and ends at the second object's left side midpoint.
- Given `with_anchors(data, [])`, then no `anchor.*` key remains and the
  other object data is untouched.
