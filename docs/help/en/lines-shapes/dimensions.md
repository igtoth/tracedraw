# Dimension lines and callouts

Dimension lines show the size of things in a technical drawing: a length, a
horizontal or vertical distance, an angle or the length of a segment.
Callouts point at something with a leader line and a label. Both stay live:
when you move, scale or turn them, they are measured and drawn again.

## Placing a dimension {#placing}

Every dimension is placed in clicks, and the property bar says what to click
next. While the last point is wanted, the dimension follows the pointer.
<kbd>Esc</kbd> starts again. Points snap as drawing does, so snapping to
nodes or objects gives exact measures; see [Snapping](../pages-layout/snapping.md).
The new dimension is selected.

### Parallel {#parallel}

The [[tool.parallel_dimension]] tool measures the straight distance between
two points, at any angle:

1. Drag from the first point to the second, or click both.
2. Move the pointer and click where the dimension line goes.

### Horizontal or Vertical {#horizontal-vertical}

The [[tool.hv_dimension]] tool measures only the horizontal or the vertical
distance between two points, whichever is larger. Place it as a parallel
dimension.

### Angular {#angular}

The [[tool.angular_dimension]] tool measures an angle:

1. Click the vertex of the angle.
2. Click a point on the first side, then a point on the second side.
3. Click where the arc goes.

The angle is measured counterclockwise from the first side.

### Segment {#segment}

The [[tool.segment_dimension]] tool measures a segment of an object: click
the segment to take its two ends, then click where the dimension line goes.

## Dimension style {#dimension-style}

The property bar shows these settings with the dimension tools and when the
selection is all dimensions. Changes apply to the selected dimensions and to
the next ones you make.

- **Precision**: 0 to 6 decimals (2 at first).
- **Units**: the drawing units (at first) or any other unit. Lengths follow
  the drawing scale, so at 1:100 a 10 mm line reads 1000 mm; see
  [Document options](../pages-layout/document-options.md#rulers).
- **Show units**: the unit after the number (the degree sign for angles).
- **Prefix** and **Suffix**: text before and after the number.
- **Text**: above the line (at first), in the line, or below the line.
- **Label size**: 1 to 500 points (10 at first).
- **Font** of the label (the font of new text at first) and the **label
  color** (black).
- **Line color** (black) and **line width** (a hairline at first, up to
  50 mm), for the lines and arrowheads.
- **Arrowheads**: arrow (at first), open arrow, dot, tick or none.
- **Round ends**: round caps and joins on the lines (off).
- The **Options** menu: the **extension line gap** between the measured
  points and the extension lines (0), how far the extension lines run
  **past the line** (2 mm), **horizontal text** for linear dimensions (off),
  **join lines at corners** (off), which makes the extension lines and the
  dimension line (or an angle's arc) one line that turns a corner instead of
  running past it, and a **corner radius** (0 to 100 mm) that rounds those
  corners. The menu stays open while you type in it.

The label always reads left to right and stays upright. Double-click it with
the [[tool.pick]] tool to edit it in place: text that still holds the
measured value keeps it live (the text before and after it become the prefix
and suffix); text without it shows as typed until you delete it.

## Dimensions that follow objects {#follow}

A dimension whose ends you placed on an object holds to it: on its nodes
(its corners, say), or on its edges (the sides of a rectangle, not only its
corners). When you move, resize, turn or reshape the object, the dimension
goes along, each end keeping its place on its node or along its edge, and
the value changes with it. Moving the dimension by itself lets it go, as does
deleting the object or the node it held to.

When you move, scale or turn a dimension, it is measured and drawn again: a
dimension scaled by 2 reads twice the length, and its label stays upright.

[[menu.object.break_apart]] (<kbd>Ctrl+K</kbd>) frees the lines, arrowheads
and label as objects of their own, and
[[menu.object.convert_to_curves]] (<kbd>Ctrl+Q</kbd>) makes the dimension a
group of plain curves. Either way it is no longer a dimension.

## Callout {#callout}

The [[tool.callout]] tool draws a label with a leader line in two segments.

1. Drag from the tip (what the callout points at) to the elbow, or click
   both.
2. Click where the text goes. The text goes on that side of the elbow.
3. Type the text. The sample text is selected, so typing replaces it.

The leader always ends at the text: when you type more, change the size or
move the text on its own (<kbd>Ctrl</kbd>+click it), the leader follows. Move
the tip or the elbow with the [[tool.shape]] tool.

The property bar, shown with the tool and when the selection is all callouts,
sets:

- the **outline width** of the leader and shape (a hairline at first);
- the **line style**;
- the **head at the tip** (an arrow at first, or none);
- the **callout shape** around the text: none (at first), underline,
  rectangle, rounded rectangle or ellipse;
- the **gap** between the text and the leader or shape (1 mm at first).
