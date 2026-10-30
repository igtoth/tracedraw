# Blends

A blend draws a series of intermediate steps between two objects, changing
shape, size and color from one to the other. Use it for smooth shading,
repeated patterns or a shape that turns into another.

## To blend two objects {#create}

With the tool:

1. Choose the [[tool.blend]] tool from the Effects flyout.
2. Drag from the start object to the end object.

With the panel:

1. Select two objects.
2. Open the [[docker.blend]] panel with [[menu.effects > menu.effects.blend]]
   (<kbd>Ctrl+F8</kbd>).
3. Set the options and click [[docker.apply]].

![The Blend panel](../../img/effects-blend.png)

Both objects stay objects of the drawing: move, resize or recolor either one
and the steps follow. Deleting the end object removes the blend.

## Blend settings {#settings}

With a blend selected, every change on the property bar or in the panel
applies at once.

- [[docker.blend_by_steps]]: the number of steps (5 at first, up to 999).
- **Spacing**, for a blend on a path: the distance between steps; the number
  of steps follows from the length of the path.
- **Rotation**: turns the steps progressively; **Loop** turns them about the
  middle between the two objects instead.
- **Acceleration**: the object slider makes the steps crowd towards one end;
  the color slider does the same for the colors. [[docker.blend_apply_sizing]]
  makes the object acceleration change the sizes too, and
  [[docker.blend_link_accel]] keeps the two sliders together. **Reset** puts
  both back to even.
- [[docker.blend_color_blend]]: [[docker.blend_direct]] (straight from one
  color to the other), [[docker.blend_clockwise]] or
  [[docker.blend_counterclockwise]] (around the color wheel).

The tool's property bar has the steps, rotation and the two accelerations.

## Blending along a path {#path}

1. Select the blend.
2. Click **New path** in the [[docker.blend]] panel and click a curve.

The steps follow the curve. **Rotate all objects** turns each step along the
curve, and [[docker.blend_full_path]] places the first and last steps at the
ends of the curve.

## Removing or splitting a blend {#clear}

**Clear blend** removes the steps and leaves both objects. **Break apart**
turns the steps into objects of their own, grouped.
[[menu.object > menu.object.break_blend_apart]] (<kbd>Ctrl+K</kbd>, also on the
right-click menu) does the same.
