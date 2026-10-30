# Drawing rectangles and ellipses

Rectangles and ellipses stay rectangles and ellipses after you draw them: you
can change their corners, turn an ellipse into a pie or an arc, and still
resize them freely. To edit them node by node, convert them to curves with
[[menu.object > menu.object.convert_to_curves]].

## Drawing with a box {#drawing}

The [[tool.rectangle]], [[tool.ellipse]], polygon, star, spiral, graph paper,
common shapes and action lines tools all draw by dragging a box. While you
drag, the object itself follows the pointer, with the settings of the
property bar.

- Hold <kbd>Ctrl</kbd> to draw a square, a circle or a regular shape.
- Hold <kbd>Shift</kbd> to draw from the center out. You can press or release
  it while dragging.
- The start and end points snap to what snapping is set to.

The new object takes the default fill and outline (no fill and a thin black
outline at first) and is selected. To change those defaults, see
[Object styles and defaults](../templates-styles/object-styles.md#document-defaults).

## Rectangle {#rectangle}

Choose the [[tool.rectangle]] tool (<kbd>F6</kbd>) and drag from one corner to
the opposite one. Double-click the tool's button to draw a rectangle the size
of the page; hold <kbd>Shift</kbd> while you double-click to frame the
selection instead.

### Corners {#rectangle-corners}

The property bar sets the corners of the selected rectangles, or of new ones
when nothing is selected:

- The corner style: **Round** (a quarter circle), **Scalloped** (a quarter
  circle cut inwards) or **Chamfered** (a straight cut).
- The four sizes: top left, top right, bottom left and bottom right. The size
  is the radius, or for a chamfer how far from the corner the cut starts. A
  corner is at most half the shorter side.
- **Edit corners together** (the lock, on at first) gives all four the size
  you type in one.
- **Relative corner scaling** (on at first): the corners grow and shrink with
  the rectangle. Turned off, they keep their size when you resize the
  rectangle and stay round when you stretch it.

You can also drag a corner node with the [[tool.shape]] tool to round all
corners at once.

## 3-Point Rectangle {#three-point-rectangle}

The [[tool.three_point_rectangle]] tool draws a rectangle at an angle:

1. Drag to draw the base line, at any angle (hold <kbd>Ctrl</kbd> for steps
   of 15 degrees).
2. Move the pointer to set the height and click.

## Ellipse {#ellipse}

Choose the [[tool.ellipse]] tool (<kbd>F7</kbd>) and drag a box. Hold
<kbd>Ctrl</kbd> for a circle.

On the property bar, choose **Ellipse**, **Pie** or **Arc**. For a pie or an
arc, set the start and end angles (in degrees, counterclockwise from the
right) and the direction. You can also drag the node of an ellipse with the
[[tool.shape]] tool to make a pie (drag inside) or an arc (drag outside).

## 3-Point Ellipse {#three-point-ellipse}

The [[tool.three_point_ellipse]] tool draws an ellipse at an angle:

1. Drag to draw the center line of the ellipse, at any angle (hold
   <kbd>Ctrl</kbd> for steps of 15 degrees).
2. Move the pointer to set the height and click.

## The Corners panel {#corners}

The [[docker.corners_docker]] panel rounds, scallops or chamfers the corners
of curves, which the rectangle settings cannot do. Open it from
[[menu.window > menu.window.dockers]].

![The Corners panel](../../img/lines-shapes-rectangles-and-ellipses-corners.png)

1. Select one or more objects. Rectangles, ellipses, polygons and text become
   curves when the corners are cut.
2. Choose the [[docker.corners_operation]]: [[docker.corners_fillet]] (a
   round arc that touches both sides), [[docker.corners_scallop]] (a notch,
   a quarter of a circle around the corner) or [[docker.corners_chamfer]] (a
   straight cut).
3. Set the [[docker.corners_radius]] for a fillet or scallop (2 mm at first),
   or the [[docker.corners_chamfer_distance]] for a chamfer: the first
   distance along the side the path comes in on and the second along the
   side it leaves on. [[docker.corners_lock]] keeps them the same.
4. The result shows as a dashed outline on the drawing. Click
   [[docker.apply]].

A corner is a node where two segments meet at an angle; smooth nodes are not
corners. With the [[tool.shape]] tool and some nodes selected, only those
corners are cut. A corner whose sides are too short for the cut is skipped.
On a stretched object a fillet or scallop still comes out round, with the
radius measured on the page.
