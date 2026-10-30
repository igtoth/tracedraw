# Changing the color mode of bitmaps

[[menu.bitmaps > menu.bitmaps.mode]] converts the selected bitmaps to another
color mode. Every selected bitmap changes in one undo step, and transparency is
kept. The modes with settings open a dialog box you can size, with the
picture before and after the change in the
[Before and After preview](before-after-preview.md) on the left and the
settings in a panel on the right; under the panel [[dialog.reset]] puts
them back while the two arrows undo and redo the changes made in the
dialog box.

| Mode | Result |
|---|---|
| [[menu.bitmaps.mode_bw]] | black and white only (see below) |
| [[menu.bitmaps.mode_grayscale]] | shades of gray |
| [[menu.bitmaps.mode_duotone]] | one to four inks (see below) |
| [[menu.bitmaps.mode_paletted]] | at most 256 colors (see below) |
| [[menu.bitmaps.mode_rgb]] | the bitmap's mode becomes RGB; the pixels stay as they are |
| [[menu.bitmaps.mode_lab]] | Lab color |
| [[menu.bitmaps.mode_cmyk]] | the bitmap's mode becomes CMYK and its pixels show the colors the inks can print, worked out with the color management profiles |

The menu ticks [[menu.bitmaps.mode_rgb]] or [[menu.bitmaps.mode_cmyk]] when
every selected bitmap is in that mode, and the status bar names the mode of
a selected bitmap, such as "Bitmap 600 x 400 px (CMYK)". A CMYK picture of a
`.cdr` file opens in the CMYK mode, and `.tdraw` files keep the mode.
[[menu.tools > menu.tools.convert_colors]] sets the mode of every bitmap it
converts in the same way (see [Converting colors](../color/convert-colors.md)).

## Black and white {#black-and-white}

![The Black and White dialog box](../../img/bitmaps-color-modes-black-and-white.png)

Choose the conversion method:

- **Line art**: white where the gray is at or above the **threshold** (128 at
  first), black elsewhere;
- **Ordered**: a regular pattern of dots;
- **Halftone** (at first): a screen of round, line or square dots at an
  **angle** (45 degrees) and a number of **lines per inch** (60);
- **Cardinality-distribution**: a random-looking texture;
- **Jarvis**, **Stucki** and **Floyd-Steinberg**: error diffusion, the
  smoothest results for photos.

**Intensity** (0 to 100, 50 at first) makes the result lighter or darker.

## Duotone {#duotone}

![The Duotone dialog box](../../img/bitmaps-color-modes-duotone.png)

A duotone prints a grayscale picture with a few inks, which gives rich tones
with few colors.

1. Choose the type: Monotone, Duotone (at first), Tritone or Quadtone, for one
   to four inks.
2. Click an ink to change its color.
3. Edit its curve: across is the gray level (black on the left), up is how
   much of that ink prints. Drag on empty space to add a point, drag a point
   to move it, double-click it or drag it off the grid to remove it. **Show
   all** draws the other inks' curves too.
4. **Save** and **Load** keep the inks and curves in a file.

## Paletted {#paletted}

![The Paletted dialog box](../../img/bitmaps-color-modes-paletted.png)

- **Palette**: Uniform Colors, Standard VGA (16 colors), Adaptive and
  Optimized (the picture's own colors, reduced to the number of **colors**
  you set, up to 256), Grayscale, System or Custom (the document palette).
- **Dithering**: None, Ordered, Jarvis, Stucki or Floyd-Steinberg, with the
  dither intensity.

The resulting palette shows under the settings.

## Coloring black and white bitmaps {#monochrome}

With only two-color bitmaps selected, click a palette color to color their
background, and right-click to color their foreground.
