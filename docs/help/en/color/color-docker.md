# The Color panel

The [[docker.color]] panel (<kbd>Shift+F11</kbd>) mixes a color and applies it
to the fill or outline of the selection. The [[tool.outline_color]] tool
(<kbd>Shift+F12</kbd>) opens it too. Open it from
[[menu.window > menu.window.dockers]].

![The Color panel](../../img/color-color-docker.png)

## Choosing a color {#choose}

1. Select objects. The panel starts from the fill color of the first one.
2. Mix the new color in one of the views:
   - [[docker.color_viewers]]: click or drag in the square to set the
     saturation (left to right) and brightness (bottom to top), and in the
     strip beside it to set the hue;
   - [[docker.color_sliders]]: one slider per component of the
     [[docker.model]]: RGB, CMYK (at first), HSB, HSL, Lab, Grayscale or YIQ;
   - [[docker.color_palettes]]: the swatches of a palette.

   The three buttons at the top show or hide each view; any mix can show.
   You can also type a `#rrggbb` value.
3. Click [[docker.fill]] or [[docker.outline]] at the bottom of the panel to
   apply the new color.
   The lock ([[docker.lock_colors]]) ties fill and outline together, so either
   button applies the color as both. The "..." button ([[docker.more_actions]])
   drops the other actions: [[docker.no_fill]], [[docker.no_outline_action]],
   [[docker.swap_colors]], [[docker.add_to_document_palette]],
   [[docker.add_to_spot_colors]] and [[docker.add_to_color_styles]].

The old color and the new one show side by side. When the new color cannot
be printed exactly with CMYK inks, the panel shows a warning.

## Sampling a color {#sample}

Click [[docker.eyedropper]] and click anywhere on the drawing to take the
color there. Then click [[docker.use_sampled]] to make it the new color. See
also [Eyedroppers](eyedroppers.md).

## Spot inks {#spot-inks}

Under the buttons, [[docker.spot_inks]] lists the spot inks of the drawing;
see [Spot colors](spot-colors.md).
