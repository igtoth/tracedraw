# Setting outlines

The outline is the line drawn along the edge of an object. You set its color,
width, style, corners, line ends and arrowheads in the [[docker.outline]]
section of the [[docker.properties]] panel.

## Opening the outline settings {#open}

- The [[tool.outline_pen]] tool (<kbd>F12</kbd>) opens the
  [[outline_pen.title]] dialog box with every outline setting: color, width
  and its unit, style, miter limit, dashes, corners, line caps, position,
  arrowheads, calligraphy, [[docker.behind_fill]], [[docker.scale_with_object]],
  the variable outline's width nodes and [[outline_pen.overprint]]. **OK**
  changes only what you changed, on every selected object; with nothing
  selected it sets the outline of new objects.

![The Outline Pen dialog box](../../img/lines-shapes-outlines-outline-pen.png)

- Double-click the outline part of the status bar.
- Right-click a color of the palette to color the outline of the selection.

## Outline Pen settings {#outline-pen}

- **Width**: 0.2 mm for new objects. [[docker.hairline]] draws a line one
  screen or printer dot wide at every zoom.
- [[docker.outline_fill]]: the outline can be painted like a fill: none,
  uniform color, gradient, pattern, texture or mesh. See
  [Fills](../color/fills.md). Hairlines and arrowheads keep the plain
  outline color.
- [[docker.caps]], the ends of open lines: [[docker.cap_butt]] (at first),
  [[docker.cap_round]] or [[docker.cap_square]].
- [[docker.corners]]: [[docker.join_miter]] (at first), [[docker.join_round]]
  or [[docker.join_bevel]].
- [[docker.dash]]: [[docker.dash_solid]], [[docker.dash_dashed]],
  [[docker.dash_dotted]] or [[docker.dash_dash_dot]]. The dashes grow with the
  width.
- Arrowheads, at the [[docker.arrow_start]] and the [[docker.arrow_end]] of
  open lines, filled in the outline color. To make your own, draw it, select
  it and choose [[menu.object > menu.object.create > menu.object.create_arrowhead]].
- The [[docker.nib_stretch]] (0.1 to 1) and [[docker.nib_angle_lc]] make a
  calligraphic outline: thinner across the nib than along it, like a flat pen.
- [[docker.behind_fill]] draws the outline under the fill, so only its outer
  half shows.
- [[docker.scale_with_object]]: the width grows and shrinks when you resize
  the object.

To turn an outline into an object you can fill, choose
[[menu.object > menu.object.convert_outline_to_object]] (<kbd>Ctrl+Shift+Q</kbd>).
The settings of new objects are explained in
[Object styles and defaults](../templates-styles/object-styles.md#document-defaults).

## Outline Color {#outline-color}

The [[tool.outline_color]] tool (<kbd>Shift+F12</kbd>) opens the
[[docker.color]] panel to choose the outline color of the selection. See
[The Color panel](../color/color-docker.md).

## Variable Outline {#variable-outline}

The [[tool.variable_outline]] tool, in the Shape flyout, makes an outline
thicker or thinner along its length with width nodes.

1. Select an object with an outline, such as a curve, a rectangle or an
   ellipse. An object with no outline gets a thin one.
2. Choose the [[tool.variable_outline]] tool.
3. Drag across the outline: a width node is added where you start, and the
   outline reaches to the pointer on both sides. Hold <kbd>Shift</kbd> (or
   <kbd>Alt</kbd>) as you start dragging to change only the side the pointer
   is on. <kbd>Shift</kbd>+click picks a run of nodes and
   <kbd>Ctrl</kbd>+click adds one; a drag then changes all the picked nodes.
4. Add more nodes the same way. Between nodes the width changes smoothly.

To change a node, click it and type the **Left width** and **Right width** on
the property bar (left is to the left of the path's direction). **Delete
node** or <kbd>Delete</kbd> removes the selected node, and **Clear variable
outline** gives back the plain outline.
