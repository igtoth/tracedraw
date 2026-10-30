# Selecting objects

Before you change an object, you select it. The [[tool.pick]] tool
(<kbd>Space</kbd> switches to it from most tools, and <kbd>Space</kbd> again
takes you back to the tool you were using) selects, moves and resizes
objects; the [[tool.freehand_pick]] tool does the same but selects with a
freehand loop.

## The Pick tool {#pick}

- **Click** an object to select it. A filled object counts where it is
  filled; an unfilled one on its outline. Text, groups, tables and pictures
  count anywhere inside their box.
- <kbd>Shift</kbd>+click adds an object to the selection or takes it out.
- **Drag** around objects, starting on an empty spot: the objects wholly
  inside the dashed frame are selected. Hold <kbd>Alt</kbd> as you finish the
  drag to select every object the frame touches. Hold <kbd>Shift</kbd> to
  turn over the objects the frame takes: those selected are deselected and
  the others added. Hold <kbd>Ctrl</kbd> to keep the frame square.
- <kbd>Ctrl</kbd>+click selects an object inside a group, one level further
  in at each click. Its handles are round, and changes act on it alone; drag
  it from anywhere on it to move it alone. <kbd>Ctrl</kbd>+click on a clip
  frame starts editing its contents instead (see
  [Clip frames](clip-frames.md#edit-contents)).
- <kbd>Alt</kbd>+click selects the object beneath the selected one, to reach
  objects that are covered.
- <kbd>Tab</kbd> and <kbd>Shift+Tab</kbd> select the next or previous object.
- <kbd>Esc</kbd> selects nothing. [[menu.edit > menu.edit.select_all]] and a
  double-click on the tool's button select every object;
  [[menu.edit.select_all]] also selects all text, guidelines or nodes.

The selected objects show eight square handles around them and a small cross
in the middle. The status bar says what is selected and on which layer.

The pointer shows the move arrows only over what is selected. Over any other
object it keeps the plain arrow, since a press there picks that object first;
dragging an unselected object still picks and moves it. A right click while
you drag a selection frame is ignored: the frame goes on and selects when you
let go of the left button.

Drawing and shaping tools whose click has no job of its own (the rectangle,
ellipse, polygon and other shape tools, the Knife, the connectors, Blend and
others) select too: click an object to select it, <kbd>Shift</kbd>+click to
add or take it away, <kbd>Ctrl</kbd>+click to reach into groups, and click
empty space to select nothing. A drag still draws. The line tools and the
Text tool keep their clicks.

### Moving, resizing and turning {#transform-with-pick}

- Drag a selected object, or the cross in its middle, to move it. Hold
  <kbd>Ctrl</kbd> to move only horizontally or vertically. The object snaps
  to what snapping is set to.
- Drag a corner handle to resize in proportion, a side handle to stretch in
  one direction. Hold <kbd>Shift</kbd> to resize from the center,
  <kbd>Ctrl</kbd> to resize in steps of 100% (crossing the other side
  mirrors the object), <kbd>Alt</kbd> to resize freely.
- Click a selected object again to show the rotation handles: drag a curved
  corner arrow to rotate, a side arrow to skew. Hold <kbd>Ctrl</kbd> for steps
  of 15 degrees. Drag the ring in the middle to turn about another point;
  double-click it to put it back in the middle. In this rotate mode, a node of
  the selection (or the middle of a segment) dragged turns the selection
  about the ring; let go near another object's edge and the node lands
  exactly on it, so an arc turned about one end can be laid onto a line. The
  rotate handle still snaps to other objects' nodes, midpoints and centres to
  aim the turn at one.
- While dragging, press <kbd>Space</kbd> or click the right button to leave
  a copy where the objects were; <kbd>Esc</kbd> cancels the drag.
- The arrow keys nudge the selection by the nudge distance (2.54 mm at
  first), with <kbd>Shift</kbd> by the super nudge distance and with
  <kbd>Ctrl</kbd> by the micro nudge distance. Set them on the
  [[options.rulers]] page of the document options.
- <kbd>+</kbd> duplicates the selection in place.

### Dragging with the right button {#right-drag}

Drag a selected object with the right mouse button. When you release, a menu
offers to move it there, copy it there, copy its fill, outline or all its
properties to the object under the pointer, or place it inside that object
as a [clip frame](clip-frames.md).

### The property bar {#pick-property-bar}

With nothing selected, the property bar shows the page size and orientation,
the drawing units and the nudge distance. With a selection, it shows the
position (X and Y), the size (W and H), the scale, the rotation angle, the
mirror buttons, the outline width, to front and to back, and convert to
curves. Text adds the text settings, and bitmaps the bitmap commands. With
several objects, a group or a curve of several parts, it adds group,
ungroup, combine, break apart, the shaping commands and the
[[docker.align_distribute]] panel.

### Selection groups {#selection-groups}

Press <kbd>Ctrl</kbd> and a number from 0 to 9 to remember the selection under
that number. Press the number alone to select those objects again, and again
to zoom to them. <kbd>Alt</kbd> and the number add the selection to the
group. Groups are saved with the drawing.

## The Freehand Pick tool {#freehand-pick}

The [[tool.freehand_pick]] tool works like the [[tool.pick]] tool, but a drag
from an empty spot draws a loop. When you release, every object wholly inside
the loop is selected; hold <kbd>Alt</kbd> as you release to select every
object the loop touches, <kbd>Shift</kbd> to add to the selection. Hold
<kbd>Ctrl</kbd> to draw a rectangle instead.

The options of the [[tool.pick]] tool, such as whether a frame selects the
objects it touches, are on the [[tool.pick]] page of the tool options; see
[Tool options](../customize/tool-options.md#pick).
