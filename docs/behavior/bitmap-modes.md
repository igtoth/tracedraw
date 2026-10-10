# Bitmap colour modes

Bitmaps > Mode converts the selected bitmaps; every selected bitmap
changes in one undo step. The dialogs show the first selected bitmap
before and after (a copy at most 240 px across; the copy's pixel sizes
follow its scale). Alpha is kept in every mode.

| Item | Result |
|---|---|
| Black and White (1-bit)... | dialog below |
| Grayscale (8-bit) | luminance `0.299 R + 0.587 G + 0.114 B` |
| Duotone (8-bit)... | dialog below |
| Paletted (8-bit)... | dialog below |
| RGB Color (24-bit) | pixels unchanged |
| Lab Color (24-bit) | each pixel through CIE Lab (D65) and back, unchanged within one level |
| CMYK Color (32-bit) | pixels unchanged (the document's colour management converts on output) |

## Black and White (1-bit)

Conversion methods:

- Line art: white when the gray level is at or above Threshold (0 to
  255, default 128).
- Ordered: an 8 x 8 Bayer matrix.
- Halftone (default): a screen of Round, Line or Square dots at Angle
  (default 45°) and Lines per inch (default 60) at the bitmap's
  resolution; the threshold field of round dots is
  `(cos u + cos v) / 4 + 0.5` in the screen's rotated cell coordinates.
- Cardinality-Distribution: a fixed per-pixel random threshold (a
  textured look).
- Jarvis, Stucki, Floyd-Steinberg: error diffusion with those kernels
  (divisors 48, 42 and 16).

Intensity (0 to 100, default 50; line art uses Threshold instead, and
the halftone screen has no slider) adds `(intensity - 50) * 2.55` to every gray level
before the conversion, so higher is lighter.

## Duotone (8-bit)

- Type: Monotone, Duotone (default), Tritone or Quadtone: one to four
  inks.
- Each ink has a colour (the swatch) and a tone curve: x is the gray
  level (0 black to 255 white), y the ink's coverage (0 to 100 %).
  Defaults: black ink from 100 % on black to 0 % on white; the second
  ink orange, 75 % on black, 50 % at the midpoint, 0 % on white; the
  third and fourth cyan and magenta at 55 % and 40 % on black.
- The curve editor: drag on empty space to add a point, drag a point to
  move it (the end points move up and down), double-click or drag it out
  of the grid to remove it. Curves are smooth (monotone cubic through the
  points). Show all draws the other inks' curves too.
- Inks print over one another: each channel is
  `prod(1 - coverage_i * (1 - ink_i))`.
- Load and Save keep the type, inks and curves in a JSON file.

## Paletted (8-bit)

- Palette: Uniform Colors (6 x 6 x 6 colours and 40 grays), Standard VGA
  (16), Adaptive and Optimized (median cut of the image's colours to
  Colors, 2 to 256, default 256; Adaptive splits the boxes with the
  widest range first, Optimized the most populous; an image with fewer
  colours keeps them all), Grayscale (256), System (VGA and the 216
  web-safe colours) and Custom (the document palette).
- Dithering: None, Ordered (Bayer, amplitude 64 levels times the
  intensity), Jarvis, Stucki, Floyd-Steinberg (the error spread times
  Dither intensity, 0 to 100, default 100).
- The processed palette is shown under the settings.
- Nearest colours are looked up through a 32 x 32 x 32 cache.

Not yet: the reference's Overprint tab, range sensitivity and saved
conversion presets; the current mode of a bitmap is not tracked, so it
is not greyed out in the menu.

## Convert to Bitmap

Bitmaps > Convert to Bitmap (also Object > Convert to Bitmap) turns the
selection into one bitmap in one undo step:

- Resolution: a list (72, 96, 150, 200, 300, 400, 600 dpi) and a value
  (36 to 2400, default 300).
- Color mode: Black and White (1-bit), Grayscale (8-bit), Paletted
  (8-bit), RGB Color (24-bit, default), CMYK Color (32-bit; the pixels
  stay, colour management converts on output). Black and white uses a 50
  % threshold, or Floyd-Steinberg when Dithered; Paletted an optimized
  256-colour palette, dithered by Floyd-Steinberg or not.
- Dithered: only for Black and White and Paletted (256 colours or fewer).
- Always overprint black: the bitmap is marked to overprint, so its black
  prints over the colours beneath (multiplied when overprints are
  simulated).
- Anti-aliasing (on): smooth edges; off renders hard pixel edges.
- Transparent background (on); off puts the selection on white.
- The uncompressed size is shown: `ceil(width x bits / 8) x height`.

## Monochrome bitmaps

A bitmap whose opaque pixels have at most two colours is monochrome.
With only monochrome bitmaps selected, a click on a palette colour
colours the background (the lighter colour, white at first) and a
right-click the foreground (the darker, black at first), in one undo
step ("Bitmap Color"); the object's own fill and outline are left alone.
A bitmap with effects or more colours takes the fill and outline as any
object does.

## Checks

- `bitmap_modes::tests::black_and_white_methods_give_two_levels_and_follow_tone`
- `bitmap_modes::tests::duotone_inks_multiply`
- `bitmap_modes::tests::paletted_uses_only_palette_colours`
- `bitmap_modes::tests::tone_curves_pass_through_their_points`
- `bitmap_modes::tests::lab_round_trips`
- `bitmap_modes::tests::conversions_apply_to_every_selected_bitmap_in_one_step`
- `ui::dialogs::tests::every_dialog_draws_and_accepts_enter` draws the
  three dialogs.
- `ops2::tests::convert_to_bitmap_applies_mode_dither_and_overprint`:
  black and white gives only black and white, grayscale equal channels,
  overprint marks the bitmap, no anti-aliasing leaves no half-covered
  pixel, and the size estimate rounds bits up per row.
- `tests::without_anti_aliasing_edges_are_hard` (renderer).
- `bitmap_modes::tests::monochrome_bitmaps_take_palette_colours`: a
  click colours the white pixels, a right-click the black ones, undo
  brings them back.
