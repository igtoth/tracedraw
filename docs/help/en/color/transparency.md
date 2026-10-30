# Transparency

Transparency lets what lies below an object show through it. It can be the
same all over, or change across the object like a gradient, or follow a
pattern or texture.

## Applying transparency {#apply}

- With the [[tool.transparency]] tool: click an object, then choose a kind of
  transparency on the property bar, or drag across the object for a gradient
  transparency from opaque (where you press) to transparent (where you
  release). The gradient's nodes then show on the object, as with the
  [[tool.interactive_fill]] tool: drag the start, the end or the round skew
  node; drag the bar between two nodes (the midpoint) to move where the blend
  is half way; double-click the line to add a node, a node to remove it.
- In the [[docker.transparency]] section of the [[docker.properties]] panel.

## Settings {#settings}

- The kind: [[transparency.none]], [[transparency.uniform]] (at first),
  [[transparency.fountain]] (linear, radial, conical, square or shaped, as
  gradient fills), [[transparency.pattern]] or [[transparency.texture]].
- The amount: 0 to 100% (50% at first). For a gradient, the stops are shades
  of gray: white is opaque, black is fully transparent.
- [[transparency.merge_mode]]: how the colors mix with what lies below.
  Normal (at first) simply shows through; the others, such as Add, Subtract,
  Difference, Multiply, Divide, If lighter, If darker, Hue, Saturation, Color
  and Invert, combine the colors in other ways.
- Whether it applies to the [[transparency.target_fill]], the
  [[transparency.target_outline]] or [[transparency.target_all]].
- **Freeze**: keeps what shows through as it is now, even if the objects
  below move.

To remove it, click [[toolbar.clear_transparency]] at the end of the property
bar (or choose [[transparency.none]] as the kind).

To copy the transparency of another object, use
[[menu.object > menu.object.copy_effect > menu.object.copy_transparency_from]].
For see-through areas defined by an object's brightness, see
[Opacity masks](../objects/clip-frames.md#opacity-mask).
