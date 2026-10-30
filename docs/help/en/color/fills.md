# Fills

A fill paints the inside of a closed object. TraceDraw has six kinds of
fill: [[docker.fill_none]], [[docker.fill_uniform]] (one flat color),
[[docker.fill_fountain]] (colors that blend into each other),
[[docker.fill_pattern]], [[docker.fill_texture]] and [[docker.fill_mesh]]
(colors that flow between the nodes of a grid).

## Ways to fill an object {#ways}

- Click a color of the palette to give the selection a uniform fill.
- Press <kbd>F11</kbd> for the [[edit_fill.title]] dialog box (see
  [below](#edit-fill)), or use the [[docker.fill]] section of the
  [[docker.properties]] panel: choose the kind of fill with the row of
  pictures, then its settings.
- Use the [[tool.interactive_fill]] tool (<kbd>G</kbd>) to set gradients on
  the object itself.
- Use the [[tool.area_fill]] tool to fill an area enclosed by several objects.
- Use the [[tool.mesh_fill]] tool (<kbd>M</kbd>) for a mesh fill.

With nothing selected, the fill you choose becomes the fill of new objects.

## Uniform fills {#uniform}

A uniform fill is one color, in any color model. Choose it from the palette,
the [[docker.color]] panel (see [The Color panel](color-docker.md)) or the
fill section of the [[docker.properties]] panel.

## Gradient fills {#gradient}

A gradient blends two or more colors. Its settings, in the fill section of
the [[docker.properties]] panel:

- The type: [[fill.linear]] (along a line), [[fill.radial]] (from the center
  out), [[fill.conical]] (around the center), [[fill.square]] (in growing
  squares) or [[fill.shaped]] (following the outline of the object, from the
  edge inwards).
- [[docker.gradient_presets]]: ready-made gradients to start from.
- The **stops**: the colors and where they sit along the gradient, 2 to 64.
  Select a stop to change its color or position; [[docker.add_stop]] and
  [[docker.remove_stop]] add and remove stops. [[docker.import_gradient]]
  takes the colors from a `.ggr` gradient file.
- The **angle** of linear, conical and square gradients, the
  [[docker.edge_pad]] (how much of each end is solid color) and the center
  offset of radial, conical and square gradients.
- [[docker.repeat]]: [[docker.repeat_none]], [[docker.repeat_sawtooth]] (the
  colors run again from the start) or [[docker.repeat_mirror]] (they run back
  and forth), with the number of [[docker.repeat_count]].
- [[docker.bilinear]]: the colors run from the first to the last and back.
- [[docker.invert_colours]]: the colors run in reverse order.

A strip under the type shows the whole run of colors as it will be drawn.

## The Edit Fill dialog box {#edit-fill}

<kbd>F11</kbd> opens the [[edit_fill.title]] dialog box on the fill of the
selection (or of new objects, with nothing selected).

- The row of pictures at the top picks the kind of fill.
- A uniform fill shows the color viewer.
- A gradient shows a preview of the fill and the [[edit_fill.fill_picker]]
  with the built-in gradients on the left. On the right: the type, how it
  repeats, and the band of colors with its nodes. Click a node to pick it and
  drag it to move it; double-click the band to add a node and a node to
  remove it. The picked node's color, transparency, position and
  [[edit_fill.midpoint]] are under the band, with [[edit_fill.reverse]].
  [[edit_fill.transform]] has the rotation, the center (for radial, conical
  and square gradients), the edge pad, mirror and inversion. The preview has
  the object's proportions and shows the start, end and skew nodes: drag them
  to move, turn and stretch the gradient as on the canvas (<kbd>Ctrl</kbd>
  turns in steps of 15 degrees).
- **OK** changes only what you changed on every selected object, with
  [[edit_fill.overprint]].
- While the dialog box is open, the selection shows the fill as you change it; **Cancel** or <kbd>Esc</kbd> puts the old fill back.

![The Edit Fill dialog box](../../img/color-fills-edit-fill.png)

## The Interactive Fill tool {#interactive-fill}

The [[tool.interactive_fill]] tool (<kbd>G</kbd>) edits fills on the object.

1. Choose the tool and drag across an object: the gradient starts where you
   press and ends where you release, keeping the colors of a gradient the
   object has already. Hold <kbd>Ctrl</kbd> to keep the angle to steps of 15
   degrees. A click on an object without a gradient gives the default fill.
2. Drag the nodes to change the gradient: the start node moves it, the end
   node sets its length and direction, and the round skew node tilts the
   bands (or makes a radial gradient an ellipse).
3. Double-click the line between the nodes to add a color node there;
   double-click a color node to remove it. Drag a color node along the line,
   or the short bar between two nodes to move the point where their colors
   meet half way.
4. Click a node to pick it: a small bar opens under it with the node's color
   (click the swatch for the color picker) and its transparency. A palette
   color clicked while a node is picked goes to that node, and the property
   bar shows the node's color, transparency and position.
5. Drag a palette color onto a node to recolor it, or onto the line between
   the nodes to add a color there.

The property bar has the kind of fill and its settings: the color of a
uniform fill; for a gradient the type, the start and end colors, the angle
and the edge pad.

## The Area Fill tool {#area-fill}

The [[tool.area_fill]] tool fills any closed area, even one formed by several
overlapping objects, by making a new object of that area. The objects around
it stay as they are.

1. Choose the [[tool.area_fill]] tool.
2. Choose the fill and outline on the property bar: **Use default** (the fill
   and outline of new objects; a color when that is no fill, so the area
   shows), **Specify** (the color, and for the outline the width, beside it)
   or **None**.
3. Click inside the area.

The new object goes on top of the active layer and is selected. A click on an
outline, or in an area that nothing encloses, makes nothing.

## Mesh fills {#mesh-fill}

A mesh fill is a grid of nodes over an object, each node with its own color
and transparency; the colors flow smoothly between the nodes. It suits
realistic shading.

1. Choose the [[tool.mesh_fill]] tool (<kbd>M</kbd>) and set the rows and
   columns on the property bar (2 by 2 at first, up to 50).
2. Click an object. It gets a mesh in the color of its fill.
3. Click a node to select it, then click a color of the palette to color it.
   Drag nodes to bend the flow of colors.
4. Double-click inside the mesh to add a node exactly there, on a new row
   and column through that point; select a node and press <kbd>Delete</kbd> to remove its row and
   column.

The property bar also has **Smooth color**, which blends the colors more
softly, the **transparency** of the selected node, and **Clear mesh**, which
removes the mesh fill. Changing the rows and columns of a mesh keeps its
colors.

## Pattern and texture fills {#pattern-texture}

- A [[docker.fill_pattern]] fill repeats a tile: two colors, a vector
  pattern or a bitmap. See [Pattern fills](patterns.md).
- A [[docker.fill_texture]] fill is a generated, natural-looking texture:
  [[texture.clouds]], [[texture.marble]], [[texture.noise]] or
  [[texture.wood]], with its colors and scale.

## Several fills on one object {#several-fills}

In the fill section of the [[docker.properties]] panel, [[docker.add_fill]]
paints another fill over the object's own, starting as a copy to change.
Further fills show as "Fill 2", "Fill 3" and so on, each with its own editor
and a button to remove it. The outline section can add further outlines the
same way.

Fills can also be painted on outlines; see
[Setting outlines](../lines-shapes/outlines.md#outline-pen).
