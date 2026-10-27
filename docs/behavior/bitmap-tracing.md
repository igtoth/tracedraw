# Bitmap tracing (bitmap to vector)

| Mode | Use | Defaults |
|---|---|---|
| Quick trace | one click | the preset of Options > Global > Bitmap Tracing (Clipart by default) |
| Centreline: technical illustration, line drawing | strokes | |
| Outline: line art, logo, detailed logo, clipart, low/high quality image | filled regions | smoothing 25, detail per preset, colours per preset |

| Control | Default | Range |
|---|---|---|
| Detail | per preset | 0..100 |
| Smoothing | 25 | 0..100 |
| Corner smoothness | 0 | 0..100 |
| Colour mode | RGB | RGB, CMYK, Grayscale, Black and white |
| Number of colours | per preset | 1..256 |
| Delete original | off (Options > Global > Bitmap Tracing) | |
| Remove background | on (Options > Global > Bitmap Tracing) | auto or colour |

## Commands

Bitmaps > Quick Trace, Repeat Last Trace (the selected bitmaps traced
with the last trace's settings, no dialog), Open Last Trace Settings
(the dialog on them), Centerline Trace and Outline Trace with their
presets; a bitmap's right-click menu has the same, as in the reference
program (#452, `ops2::tests::the_last_trace_can_be_repeated_and_reopened`).
The last trace is kept for the session.

## Dialog preview

The Trace Bitmap dialog opens with the bitmap before and its trace after,
side by side (200 points each, a checkerboard behind transparency). The
trace is worked out on a copy of the bitmap at most 240 pixels on its
longer side, again whenever a setting changes: closed regions filled with
their colours, centrelines stroked. Preview still counts the curves of the
full trace.

## Algorithm (outline mode)

1. Noise (#611, off at detail 95 and up, where the specks stay too):
   each colour channel of each opaque pixel becomes the median of the
   pixel and its four edge neighbours (opaque ones only), once at detail
   75 and up, twice below. Lone odd pixels and the grain of compressed
   images go, straight edges, square corners and one pixel lines along
   rows or columns stay (a line's end pixels go). Without it the grain of
   flat colours split them into many shades whose patches left holes
   showing the page inside the art.
   Blur a little when detail < 50, then quantise the colours with median
   cut to the number of colours (black and white: by brightness).
2. Specks (#611): every 4-connected patch of one colour smaller than
   `round(((100 - detail) / 10)^2)` pixels (none at detail 100, 4 at 80,
   16 at 60, 42 at 35) takes the colour most of its neighbours have, so
   stray dots and the fringes of soft edges make no curves of their own.
   Transparent pixels are left alone.
   Thin bands (#611, off where the specks are, at detail 95 and up): a
   patch almost none of whose pixels has all four neighbours in it (at
   most about 2 px thick), touching two or more colours, where the two it
   touches most are at least 64 apart (RGB distance, so the steps of a
   gradient stay) and its colour lies between them (within
   `max(12, 0.08 * distance)` of the RGB segment joining them), is the
   anti-aliasing of their edge: each of its pixels takes whichever of
   the two most of its 8 neighbours have (the nearer colour on a tie).
   A thin patch touching one colour only, such as a hairline on a fill,
   stays.
3. For each colour, the boundary rings of its mask along the pixel
   edges, looked for in the box of its pixels only. A region's mask also
   takes the pixels of the regions stacked above it (lighter ones, not
   the background) within 2 px of its own, so it reaches a little under
   its neighbours: their curves are fitted each on its own, but no page
   shows between them (#611). A region with no lighter region drawn over
   it is not grown.
4. Smoothing 0: each ring as straight lines along the pixel edges
   (Douglas-Peucker, tolerance `0.3 + (100 - detail) / 100 * 2.5` px).
   Otherwise each ring is fitted (#611):
   - one point in the middle of every pixel edge, so stair steps become
     slopes;
   - corners where the ring turns by more than `60 + corner smoothness *
     0.8` degrees over 3 pixels (the sharpest point of each run), moved
     onto the pixel corner standing out, so a square keeps its corners;
   - the points between corners smoothed `round(1 + smoothing * 0.12)`
     times with weights 1, 2, 1;
   - each stretch between corners (a ring without corners in quarters
     meeting smoothly) fitted with as few cubic Beziers as keep every
     point within `0.2 + (100 - detail) / 100` px: least squares on
     chord-length parameters with Newton steps, split at the worst point
     (P. J. Schneider, "An Algorithm for Automatically Fitting Digitized
     Curves", Graphics Gems, 1990). Ends take the direction over 4
     points.
5. Stack regions dark to light; remove the region of the background
   colour (the most common colour on the border) when requested.

Centreline mode: Zhang-Suen thinning, then the skeleton is traced into
polylines, smoothed (Catmull-Rom) and output as curves with a stroke
width equal to the mean local thickness.

## Checks

- `trace::tests::traces_are_smooth_curves_with_sharp_corners_and_no_specks`:
  a 30 px circle is 2 to 12 curves, no straight steps, within the
  fitting tolerance of the circle and 2 % of its area; a square with
  specks around it is one square with corners on its pixel corners; at
  detail 100 the specks stay.
- `trace::tests::despeckle_fills_small_patches_with_their_surroundings`.
- `trace::tests::denoise_removes_lone_pixels_and_keeps_edges`.
- `trace::tests::noisy_flat_art_traces_without_holes_or_specks`: the
  yellow disc and black band with noise of up to 10 per channel, Clipart
  preset: under 1000 subpaths (over 1400 before) and fewer than 1 in 200
  pixels inside the disc showing the page (about 3800 of 9800 before).
- `trace::tests::neighbouring_regions_leave_no_gaps_and_edges_make_no_slivers`:
  an anti-aliased yellow disc crossed by a black band on white, Clipart
  preset: two or three regions, and drawn back no pixel more than 2 px
  inside the art is white or lets the page through.
- `trace::tests::thin_edge_bands_go_to_their_neighbours`,
  `trace::tests::close_colours_and_colour_counts_are_kept` (a 246 grey
  logo on white at the Logo preset and with two colours, a grey ramp at
  96 colours), `trace::tests::the_highest_detail_keeps_thin_lines`,
  `trace::tests::dilation_grows_a_mask_by_a_square`.

- Given a 200x200 image of a black circle on white, outline mode, 2
  colours, then one closed curve with area within 2% of the circle.
- Given the same with "remove background", then the white region is not
  emitted.
