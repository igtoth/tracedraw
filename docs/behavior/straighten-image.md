# Straighten Image

Bitmaps > Straighten Image corrects lens distortion, rotation and
perspective of the selected bitmaps. OK
changes every selected bitmap in one undo step ("Straighten Image"); each
keeps its centre and its resolution, so a cropped result is smaller on
the page. The result is new pixels: a bitmap's effects list is baked in.

## Dialog

- The preview shows a copy at most 560 pixels across with the corrections
  and without cropping; the cropping area is the bright rectangle, the
  rest is dimmed. Tools: Zoom in and Zoom out (click a point to zoom by
  1.5 about it), Zoom to fit, 100% (one bitmap pixel per screen point),
  Pan (drag), and Rotate counterclockwise and Rotate clockwise (a quarter
  turn of the image, applied before the other corrections).
- Grid (on): lines fixed to the preview window to line the image up
  against; the slider sets the cell size (8 to 80 points, default 24)
  and the colour button its colour (default gray).
- Sliders: Correct lens distortion (-100 to 100, default 0), Rotate image
  (-15 to 15 degrees in steps of 0.1, positive counter-clockwise),
  Vertical perspective and Horizontal perspective (-100 to 100).
- Crop image (on) crops to the largest centred rectangle with the
  original proportions inside the corrected image; Crop and resample to
  original size (off, only with Crop image) brings it back to the
  original pixel size. Without cropping the canvas grows to hold the
  rotated image and the corners are transparent.
- Reset puts every setting back and fits the preview.

## Formulas

For an image of `w` by `h` pixels (after the quarter turns), every pixel
of the result, measured from the centre, takes its colour from the
original by these steps (bilinear, premultiplied alpha):

1. Rotation undone: `(x c - y s, x s + y c)` with `c, s` the cosine and
   sine of the angle (y down, so positive angles turn the picture
   counter-clockwise).
2. Perspective: `x' = x (1 - kv y / h)`, `y' = y (1 - kh x / w)` with
   `kv = vertical / 200` and `kh = horizontal / 200`. Negative vertical
   values widen the top (a building leaning back); negative horizontal
   values enlarge the left side (a photo taken from the right).
3. Lens: both coordinates times `1 + 0.3 (lens / 100) r^2`, with `r` the
   distance from the centre over half the diagonal. Negative values
   correct barrel distortion (pulling the edges out), positive ones
   pincushion.

The crop scale is the largest `k` (at most 1) for which a centred
rectangle of `k w` by `k h` maps entirely inside the original, found by
bisection on 48 points along each edge.

## Checks

- Given default settings, then the pixels are unchanged; a quarter turn
  clockwise puts the bottom left at the top left, and three more give the
  image back (`straighten::tests::neutral_settings_and_turns_keep_pixels`).
- Given a 10 degree rotation, then the source of a point right of the
  middle lies below it; the crop keeps the 3:2 proportions with no
  transparent pixel; resampled it keeps 60 by 40 pixels; without
  cropping the canvas grows and its corner is transparent
  (`rotation_crops_to_the_original_proportions_or_grows`).
- Given barrel correction, corners sample nearer the middle and need no
  crop; pincushion and perspective need one; settings out of range are
  clamped (`lens_and_perspective_move_pixels_the_right_way`).
- Given a bitmap turned and rotated, then it changes in one undo step,
  keeps its centre and one millimetre per pixel
  (`straightening_keeps_centre_and_resolution_in_one_step`).
