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

A connector drag starts on one object and ends on another; while it is
dragged, a blue hairline on a white line twice as wide runs from the
start anchor to the pointer, so it shows over a black fill (#672; the
Blend tool's drag from one object to another shows the same line from
the object's centre). The end points are:

- the anchor of the start object nearest to where the drag began, when
  the drag began on an anchor (within the hit radius),
- the anchor of the end object nearest to where the drag ended, when it
  ended on one,
- otherwise the closest pair of anchors between the two objects.

Right-angle connectors go from the start anchor to an elbow at the
midpoint of the dominant axis and then to the end anchor; the rounded
variant replaces the two corners by quadratic arcs of the bar's corner
radius (3 mm at first), never more than a quarter of the distance or half
a leg. The connector is an open path named "Connector" with no fill and
the default outline (`connectors.rs`).

## Staying attached

The connector keeps what it joins in its object data (`connector`): the
two objects, each anchor as fractions of its object's bounds, how it runs,
the corner radius and where its ends were last put. After every change of
the drawing:

- its objects moved, scaled or reshaped while the connector did not: it
  runs again between their anchors, in the same undo step;
- moved together with its objects: it stays attached;
- moved on its own (or pasted elsewhere): it is let go and stays where it
  is as a plain line (Undo attaches it again);
- one of its objects deleted: it stays where it is, and follows again if
  Undo brings the object back;
- one of its objects copied without it (Duplicate, a right click during a
  drag and its Repeat, Copy and Paste in the same drawing): a copy of the
  connector, of the same kind, outline and anchors, joins the object at the
  other end to the copy, in the same undo step; copied with the connector,
  or pasted into another drawing, no more is added (#476,
  `connectors::tests::copies_of_connected_objects_bring_their_connector`).

## Property bar

With the connector tools, and when the Pick tool's selection is all
connectors (after the position and size fields): Straight, Right angle,
Rounded right angle (with nothing selected they pick the tool; with
connectors selected they change how those run), the corner radius of
rounded ones (a drag of the field is one undo step), then for selected
connectors the outline: width, style, arrowheads.

While a connector is dragged the anchors of both objects are shown, and
the rubber-band line starts at the anchor that will be used.

## Checks

- `connectors::tests::connectors_follow_their_objects`
- `connectors::tests::right_angle_and_rounded_routes`
- `connectors::tests::selected_connectors_change_kind`
- Given a custom anchor at the top-right corner of a 10 mm square, when
  the square is scaled by 2, then the anchor is still at the top-right
  corner (fractions, not millimetres).
- Given that anchor and a second object to the right, when a connector is
  dragged from the anchor to the second object, then it starts at the
  anchor and ends at the second object's left side midpoint.
- Given `with_anchors(data, [])`, then no `anchor.*` key remains and the
  other object data is untouched.
