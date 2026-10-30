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

With a graphics tablet, the pen's pressure sets the width of the line along
it; see [Drawing with a graphics tablet](pen-pressure.md).

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

While you drag a handle, hold <kbd>C</kbd> to make the node a cusp (only the
outgoing handle follows), or <kbd>Alt</kbd> to move the node itself. Click
the last node again to make it a cusp. A handle pulled out of the drawing
window (over a panel or a toolbar) ends where you let go.

While you draw, <kbd>Ctrl+Z</kbd> takes back the last node or handle, one at a
time, and <kbd>Ctrl+Shift+Z</kbd> puts them back. A finished curve keeps these
steps: undoing goes back to drawing it, with the tool chosen again and a line
going on from the last node left, so you can undo node by node and draw on at
once.

What you are drawing shows over any color: the segments placed so far lie on
a white line, and the handles are dashed in blue and white.

## Pen {#pen}

The [[tool.pen]] tool places nodes the same way as the [[tool.bezier]]
tool: click for a corner node, drag for a smooth node, double-click or
<kbd>Enter</kbd> to finish, click the first node to close. While the button
is down, the handle follows the pointer, so you see the curve take shape.
Its pointer is a pen nib whose tip is where the node goes.

Two options sit on its property bar (and on the [[options.curve_tools]] page
of the tool options):

- [[options.pen_preview]] (on at first): a solid blue line on a white one
  runs from the last node to the pointer, so you see the next segment before
  you click.
- [[options.pen_auto_add_delete]] (on at first): with a curve selected and
  no curve being drawn, a click on a segment adds a node and a click on a
  node deletes it. While you draw a curve, every click places the next node,
  also on a segment or a node of a curve, so drawing never changes other
  curves by accident.

With no curve being drawn, the Pen and Bezier tools also edit the selected
curve: drag a node that is not an end node to move it, as with the Shape
tool.

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

The [[tool.bezier]] and [[tool.pen]] tools also go on from an open curve that
is not selected: press on one of its end nodes to select it and draw on. The
next segment leaves smoothly, along the mirror of the curve's last handle;
drag from the end node to pull a new handle instead, or click the end node a
second time to make it a cusp, so the next segment leaves it straight.

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
