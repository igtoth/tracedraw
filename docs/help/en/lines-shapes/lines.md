# Drawing lines and curves

A line in TraceDraw is a **curve**: a path made of **segments** that join at
**nodes**. Segments can be straight or curved. A curve can be open, with two
ends, or closed, when its end meets its start, and then it can be filled.

The curve tools are in the Curve flyout of the toolbox. Every one of them
draws a curve with the default fill and outline of new objects; change them
afterwards in the [[docker.properties]] panel or with the color palette.

## Freehand {#freehand}

The [[tool.freehand]] tool (<kbd>F5</kbd>) draws a line where you drag, like
a pencil.

1. Choose the [[tool.freehand]] tool.
2. Drag on the page. Release the button to finish.

TraceDraw follows the pointer and smooths the line when you release it. The
smoothing (0 to 100, 50 at first) on the property bar sets how much: a
higher value gives fewer nodes and a softer line. When you are zoomed in,
the line keeps more detail.

To draw a straight line with this tool, click where the line starts and
click again where it ends. Hold <kbd>Ctrl</kbd> to keep the line to steps of
15 degrees. <kbd>Esc</kbd> drops the first click.

## 2-Point Line {#two-point-line}

The [[tool.two_point_line]] tool draws one straight segment: press where it
starts and release where it ends, or click twice. Hold <kbd>Ctrl</kbd> to
keep it to steps of 15 degrees.

## Bezier {#bezier}

The [[tool.bezier]] tool draws precise curves node by node.

1. Choose the [[tool.bezier]] tool.
2. Click to place a corner node (a cusp), or drag to place a smooth node and
   pull out its handles, which set the direction and the bend of the curve.
3. Go on placing nodes.
4. Double-click or press <kbd>Enter</kbd> to finish. To close the curve,
   click its first node. <kbd>Esc</kbd> throws away the curve you are
   drawing.

Hold <kbd>Ctrl</kbd> to place the next node in steps of 15 degrees from the
last one, or to turn a handle in such steps. Every node snaps to what
snapping is set to; see [Snapping](../pages-layout/snapping.md).

## Pen {#pen}

The [[tool.pen]] tool places nodes the same way as the [[tool.bezier]]
tool: click for a corner node, drag for a smooth node, double-click or
<kbd>Enter</kbd> to finish, click the first node to close. While the button
is down, the handle follows the pointer, so you see the curve take shape.

## B-Spline {#b-spline}

The [[tool.bspline]] tool draws smooth curves from control points, without
handles.

1. Click to place each control point. The curve follows the points, starting
   at the first and ending at the last, and always stays inside them. While
   you draw, the control points show as a dashed line.
2. Double-click or press <kbd>Enter</kbd> to finish. Click the first point
   (once there are three) to close the curve into a smooth loop.

The result is an ordinary curve; the control points are not kept.

## Polyline {#polyline}

The [[tool.polyline]] tool draws straight segments one after the other:
click to add each corner, and double-click or press <kbd>Enter</kbd> to
finish. Hold <kbd>Ctrl</kbd> to keep each segment to steps of 15 degrees.

## 3-Point Curve {#three-point-curve}

The [[tool.three_point_curve]] tool draws an arc through three points:

1. Drag from where the curve starts to where it ends. Hold <kbd>Ctrl</kbd>
   to keep this line to steps of 15 degrees, for a level arc.
2. Move the pointer to bend the curve, and click the point it passes
   through. Hold <kbd>Ctrl</kbd> for a bow, a part of a circle that stands
   evenly on the line.

## Continuing a curve {#continue}

To add to an open curve, select it, then draw with the [[tool.freehand]],
[[tool.two_point_line]], [[tool.bezier]], [[tool.pen]], [[tool.bspline]] or
[[tool.polyline]] tool starting (or ending) on one of its end nodes. The new
line becomes part of the curve:

- it runs on from that end;
- when it ends on the curve's other end, the curve closes;
- when it ends on an end of another part of the same curve, the two parts
  join.

The pointer shows a node with a line going on from it when you are over an
end node, and a ring when a click would close the curve. The curve keeps its
look and stays selected, and the change is one undo step. Closed curves,
several selected objects and objects inside groups are never continued. The
distance that counts as "on the node" is set on the [[options.curve_tools]]
page of the tool options; see [Tool options](../customize/tool-options.md#curves).

## The property bar {#property-bar}

While a curve tool is active, the property bar shows the position, size,
scale, angle and mirror buttons of the selection, its outline width, line
style and start and end arrowheads, and a button that closes an open curve.
The [[tool.freehand]] tool adds the smoothing. With nothing selected, it
reminds you how to finish a curve. See [Outlines](outlines.md) for the
outline settings.

## Editing curves afterwards {#edit}

Use the [[tool.shape]] tool to move, add and delete nodes, change their type
and bend segments; see [Shaping curves with the Shape tool](shape-tool.md).
