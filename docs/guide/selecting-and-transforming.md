# Selecting and transforming

Almost everything starts with a selection. This page shows how to pick
objects, move, scale, turn and slant them, and line them up precisely.

## The Pick tool

The Pick tool is the first button of the toolbox. Press `Space` to get it
from most tools and `Space` again to go back to the tool you were using;
`Ctrl+Space` gets it from any tool, even while text is edited.

To select objects:

- **Click** an object to select it. An unfilled object is picked by its
  outline (within 3 pixels) unless **Tools > Options > Tools > Pick >
  Treat all objects as filled** is on (it is on by default).
- **Shift+click** adds an object to the selection, or takes it out.
- **Alt+click** selects the object *beneath* the selected one under the
  pointer. Keep clicking to dig down through a stack; after the bottom
  object you come back to the top.
- **Tab** and **Shift+Tab** step through the objects in stacking order.
- **Ctrl+A** selects everything; double-clicking the Pick tool's button
  does the same.
- **Esc** deselects.

The status bar tells you what is selected and on which layer, for
example "2 Objects Selected on Layer 1".

### Marquee selection

To select several objects at once, drag on an empty part of the page. A
dashed blue frame follows the pointer:

- Objects lying wholly inside the frame are selected.
- Hold **Alt** when you let go to select every object the frame
  *touches* as well.
- Hold **Shift** to add to the current selection.
- Hold **Ctrl** to keep the frame square.

If you would rather have touching be the default, turn on **Tools >
Options > Tools > Pick > Marquee selects touched objects**; Alt then
selects only what is wholly inside.

### Freehand Pick

The **Freehand Pick** tool, in the Pick tool's flyout, works like the
Pick tool except that dragging on empty space draws a lasso. When you let
go, objects wholly inside the loop are selected; hold **Alt** at the end
to take every object the lasso touches, or hold **Ctrl** to drag a
rectangle instead.

### Selection groups

To come back to a set of objects quickly:

- `Ctrl+0` to `Ctrl+9` store the selection under that number.
- The number alone (`0` to `9`) selects those objects again; press it
  again to zoom to them.
- `Alt+0` to `Alt+9` add the selection to that group.

Groups are saved with the drawing.

## Groups

To group objects so they move together: select them and press `Ctrl+G`.
`Ctrl+U` ungroups. With a group in the selection, the property bar also
offers **Ungroup All**.

To work on one object inside a group without ungrouping, **Ctrl+click**
it. Each Ctrl+click goes one level further into nested groups. The
object shows round handles, and moving, scaling, colouring or deleting
it changes that object only. **Ctrl+Shift+click** adds another object
inside a group.

**Combine** (`Ctrl+L`) makes one curve of several parts; **Break Apart**
(`Ctrl+K`) splits such a curve again.

## Move objects

- **Drag** a selected object, or the small cross at its centre.
- Hold **Ctrl** while dragging to keep the move horizontal or vertical.
- Press **Space** during a drag to drop a copy where the selection is,
  and go on moving.
- **Right-click** during a drag to leave the original in place and move
  a copy; right-click again to take that back.
- Press **Esc** during a drag to cancel it.

While you drag, the objects stay put and a blue outline shows where they
will land; the status bar shows the distance and angle.

### Drag with the right mouse button

Drag a selected object with the right button and let go: a menu offers
**Move Here**, **Copy Here**, **Copy Fill Here**, **Copy Outline Here**,
**Copy All Properties**, **Place Inside** (the object under the pointer
becomes a clip frame for the selection) and **Cancel**.

### Nudge with the arrow keys

The arrow keys move the selection by the nudge distance (2.54 mm to
start with):

- `Shift`+arrow: the super nudge distance (5.08 mm).
- `Ctrl`+arrow: the micro nudge distance (0.254 mm).

Change the nudge distance on the property bar with nothing selected, or
in **Layout > Document Options > Rulers**.

### Duplicate

- `Ctrl+D` duplicates the selection at the duplicate distance.
- `+` (with the Pick tool) duplicates it in place, with no offset.
- `Ctrl+Shift+D` opens the **Step and Repeat** panel.

## Scale and stretch

A selection shows eight black square handles:

- A **corner handle** scales in proportion from the opposite corner.
- A **side handle** stretches one direction from the opposite side.
  Dragging past the opposite side mirrors the object.
- Hold **Shift** to scale from the centre.
- Hold **Ctrl** to scale in steps of 100 % (dragging across gives a
  mirror image at the same size).
- Hold **Alt** on a corner handle to scale freely.

You can also type the size, scale percentages and position on the
property bar. The **object origin** grid at its left chooses which point
of the selection X and Y refer to, and which point stays put when you
type a new size.

## Rotate and skew

Click a selected object a second time. The handles change to arrows:

- **Curved arrows** at the corners rotate the selection about the centre
  of rotation. Hold **Ctrl** to turn in steps of the constrain angle
  (15 degrees; change it in **Tools > Options > TraceDraw > Edit**).
- **Straight double arrows** at the sides skew (slant) the selection,
  with the opposite side fixed. Hold **Shift** to skew about the centre
  line.
- The **centre of rotation**, a ring with a dot, can be dragged to turn
  about another point. Double-click it to put it back in the middle.
  TraceDraw remembers where you put it for that object.

Click the object again to go back to the scale handles.

### Free Transform

The **Free Transform** tool, in the Pick flyout, transforms the
selection about the point where you press. Its property bar offers four
modes: Free Rotation, Free Angle Reflection, Free Scale and Free Skew.
For one drag, **Alt** scales, **Ctrl** skews and **Shift** reflects.
Turn on **Apply to Duplicate** to transform a copy and leave the
original. See [the Free Transform notes](../behavior/free-transform.md).

## Snapping

Snapping pulls the pointer to useful points while you draw and move. To
start with, TraceDraw snaps to objects, guidelines and the page.

- `Alt+Q` turns all snapping off and on (also the **Snap Off** button).
- `Alt+Z` snaps to objects; `Ctrl+Y` snaps to the document grid.
- `Alt+Shift+D` turns on dynamic guides; `Alt+Shift+A` alignment guides.
- The **Snap To** list on the standard toolbar chooses what to snap to.

When you start a move on one of the selection's points (a node, a
midpoint, the centre), that point is the one carried onto other objects.
A mark and a small label, such as "node" or "midpoint", show what you
snapped to. **Tools > Options > TraceDraw > Snapping** has the snapping
radius (10 pixels) and the snapping modes: node, intersection, midpoint,
quadrant, tangent, perpendicular, edge, centre and text baseline.

Detail: [the snapping notes](../behavior/snapping.md).

## The Transformations panel

For exact moves, turns and sizes, open the Transformations panel:

| Keys | Page |
|---|---|
| `Alt+F7` | Position |
| `Alt+F8` | Rotate |
| `Alt+F9` | Scale and Mirror |
| `Alt+F10` | Size |

**Object > Transformations** opens any of its five pages, including
**Skew**.

To use it:

1. Select the objects.
2. Choose the page and type the values (distances in your ruler unit,
   angles in degrees counter-clockwise, scales in percent).
3. Pick the reference point on the 3 x 3 grid (the centre to start
   with).
4. To make copies instead of changing the selection, set **Copies**
   (up to 500). Each copy is transformed one step further than the last.
5. Press **Apply**.

On the Position page, **Relative position** moves the selection by X and
Y; turned off, it moves the reference point to X and Y. Detail:
[the Transformations notes](../behavior/transformations.md).

## Align and distribute

To line objects up, open **Align and Distribute**
(**Object > Align and Distribute**).

- Six **Align** buttons line up left edges, centres or right edges, and
  top edges, centres or bottom edges.
- **Align to** chooses the target: the active object (the last one
  selected; with one object selected, the page), the page edges, the
  page centre, or the grid.
- Eight **Distribute** buttons space edges, centres or gaps evenly
  across the selection (at least 3 objects) or across the page (at
  least 2).
- **Use Outline** counts the outlines' width in the boxes.

Keyboard shortcuts, with a selection: `L`, `R`, `T` and `B` align left,
right, top and bottom; `E` and `C` centre horizontally and vertically;
`P` centres on the page. These keys align to the active object; a single
object aligns to the page.

Detail: [the Align and Distribute notes](../behavior/align-distribute.md).

## Stacking order

| Keys | Moves the selection |
|---|---|
| `Ctrl+Home`, `Ctrl+End` | to the front or back of the page |
| `Shift+PgUp`, `Shift+PgDn` | to the front or back of its layer |
| `Ctrl+PgUp`, `Ctrl+PgDn` | forward one, back one |

Layers, and the order of objects within them, are shown in the
**Objects** panel. See [the Objects panel notes](../behavior/objects-panel.md).

## Where next

- Draw new objects: [Drawing](drawing.md).
- Reshape them node by node: [Shape editing](shape-editing.md).
- The full table of Pick tool actions: [Pick tool notes](../behavior/pick-tool.md).
