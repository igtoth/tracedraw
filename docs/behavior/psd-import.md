# PSD import

Photoshop documents (`.psd`, and `.psb` large documents) open as bitmap
objects, one per layer, or one from the flattened composite when the
file has no layers.

## What is read

- Header: 8 and 16 bits per channel (32-bit float is sampled to 8),
  Bitmap, Grayscale, Indexed (planar palette from the colour mode
  data), RGB, CMYK (stored inverted, converted naively), Duotone (as
  gray) and Lab modes. The resolution resource (0x03ED) sets the page
  size; 72 ppi otherwise.
- Layers: bounds, name (Unicode `luni` wins over the Pascal name),
  opacity, visibility; channels 0..n, alpha (-1) and the layer mask (-2,
  applied as alpha with its default colour outside the mask bounds);
  raw, RLE (PackBits), ZIP and ZIP-with-prediction channel data; section
  dividers (groups) are skipped, their content stays in order; layers
  are placed by their pixel bounds at the document resolution. Each
  layer becomes a bitmap; File > Import wraps several in a group named
  after the file.
- Composite: used when there is no layer record; same decoders.

## Not read

Blend modes (every layer composites "normal"), adjustment and fill
layers (no pixels), smart-object transforms, vector masks, text layers
as text (their raster is kept), clipping groups.

## Checks

- A 2 x 1 raw RGB composite gives one 2 x 1 bitmap with red and green
  pixels; the page is 2 x 1 pt at 72 ppi.
- An RLE layer "Hi" at (1,1)-(3,3) with alpha gives a 2 x 2 bitmap named
  Hi at opacity 200/255, pixel (0,0) = (10, 30, 50, 255), pixel (1,0)
  alpha 128, placed one pixel in from the page's left and top.
- Garbage and truncated headers are errors, never panics.
- Real files: an RGB export of the 2019 banner and a 16-bit layered
  file from an image tool open with the right colours.
