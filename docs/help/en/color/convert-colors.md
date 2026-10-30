# Converting colors to CMYK or RGB

A drawing often mixes colors defined in RGB (for screens) with colors defined
in CMYK (for printing inks). Before you send a drawing to print, or reuse a
print job on the web, [[menu.tools > menu.tools.convert_colors]] turns every
color of the selected objects, or of the whole drawing, into one model in a
single step.

## Converting the colors {#convert}

1. To convert only some objects, select them first.
2. Choose [[menu.tools > menu.tools.convert_colors]].
3. Under [[convert_colors.to]], choose [[convert_colors.cmyk]] (the default)
   or [[convert_colors.rgb]].
4. Under [[convert_colors.which]], choose [[convert_colors.selection]] (the
   default when something is selected) or [[convert_colors.drawing]], which
   also takes in the pages' backgrounds and the symbols.
5. Tick [[convert_colors.spots]] to turn spot colors into their CMYK or RGB
   mix as well (off at first).
6. Click [[dialog.ok]].

The status bar says how many colors changed. All the changes undo in one
step. When every color is already in the model you chose, or nothing is
selected, nothing changes.

## What is converted {#what}

- Solid fills, the stops of gradients, the colors of mesh fills, patterns
  and textures.
- Outlines, text and table cells.
- The colors of effects (contours, blends, extrusions, bevels, block shadows
  and the others) and of drop shadows.
- Objects inside groups and clip frames, at any depth.
- Bitmaps take the color mode: converted to CMYK, a picture's pixels change
  to show the colors the inks can print; converted to RGB, only the mode
  changes and the pixels stay. See
  [Bitmap color modes](../bitmaps/color-modes.md).

The registration color never changes, so marks still print on every plate.
Spot colors stay spot colors unless you tick [[convert_colors.spots]]. A
color that is already in the model stays as it is.

## How the colors are worked out {#profiles}

The conversion goes through the color management profiles: the drawing's own
CMYK profile when it carries one, else the RGB and CMYK profiles, the
rendering intent and the black point setting of
[[menu.tools > menu.tools.color_management]]. See
[Color management](color-management.md).

> **Tip:** A color converted to CMYK and back to RGB is not the color you
> started with when it was too bright for the inks. Keep a copy of the
> drawing before converting a whole drawing.
