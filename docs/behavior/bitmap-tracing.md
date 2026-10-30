# Bitmap tracing (bitmap to vector)

| Mode | Use | Defaults |
|---|---|---|
| Quick trace | one click | the preset of Options > Global > Bitmap Tracing (Clipart by default) |
| Centreline: technical illustration, line drawing | strokes | |
| Outline: line art, logo, detailed logo, clipart, low/high quality image | filled regions | smoothing 25, detail per preset, colours per preset |

| Control | Default | Range |
|---|---|---|
| Trace type | the menu item's (Outline, Centerline) | the Image type list follows: Technical illustration and Line drawing for Centerline; Line art, Logo, Detailed logo, Clipart, Low quality image, High quality image for Outline |
| Image type | the menu item's | choosing one sets Detail, Smoothing, Corner smoothness, Colors and the colour mode from the preset, with the choices of Options > Global > Bitmap Tracing |
| Detail | per preset | 0..100 (tick marks every 10, the value in the box at the right) |
| Smoothing | 25 | 0..100 |
| Corner smoothness | 0 | 0..100 |
| Remove | Background color (Options > Global > Bitmap Tracing) | Background color, Entire color from image, None |
| Remove: Automatically / Specify | Automatically | Specify: a colour swatch and an eyedropper that takes the colour from a click on the preview |
| Objects: Merge adjacent objects of the same color | on | |
| Objects: Remove object overlap | off | |
| Objects: Group objects by color | off | |
| Image: Delete original image | off (Options > Global > Bitmap Tracing) | |
| Colors tab: Color mode | RGB (Black and white for the line art presets) | RGB, Grayscale, Black and white |
| Colors tab: Number of colors | per preset | 2..256 |
| Colors tab: list | the trace's colours with their curve counts | tick two or more and Merge selected colors; the swatch of a row changes the colour its curves are drawn in |
| Reset, undo, redo | | Reset: the image type's settings; the arrows step through every settings state of the dialog, Reset included |
| Preview | Before and After | the shared preview (`before-after-preview.md`): Before and After (the picture above, the trace below), Before and After side by side, Before, Large Preview (the trace alone in the whole area), Wireframe Overlay (the curves as blue lines over the picture); zoom to fit, zoom out, zoom in, pan, the wheel zooms, a drag pans |
| Overlay | 80 | with Wireframe Overlay only, a slider in the preview bar: how strongly the picture shows under the curves, 0 (the curves alone) to 100 (the picture as it is); a tint on the picture's rectangle, nothing traced or worked out again |
| Options... | | Options > Global > Bitmap Tracing, then back to the dialog |

## Commands

Bitmaps > Quick Trace, Repeat Last Trace (the selected bitmaps traced
with the last trace's settings, no dialog), Open Last Trace Settings
(the dialog on them), Centerline Trace and Outline Trace with their
presets; a bitmap's right-click menu has the same (#452, `ops2::tests::the_last_trace_can_be_repeated_and_reopened`).
The last trace is kept for the session.

## The dialog

A window the user sizes (1040 x 700 points at first, never under 780 x
540; a maximize button in the title bar fills the main window, pressing
it again restores the size). Left, the shared Before and After preview
of the first selected bitmap (`before-after-preview.md`): a Preview
list (Before and After, Before and After side by side, Before, Large
Preview, Wireframe Overlay, with the Overlay slider beside the list in
the overlay view), zoom to fit, zoom out and zoom in buttons, a pan
button, and a spinner while a trace is being made; under the preview
the status line `Curves: N, Nodes: N, Colors: N` (the whole picture's
trace; with a dots sign after it while only the quick look is in).
Before and After stacks the picture over its trace, each in its own
pane, with the same zoom and place; the wheel zooms about the pointer
(fit / 10 to 64 points per preview pixel), a drag pans, the images sit
on a checkerboard (one 2 x 2 texture repeated, a single rectangle a
frame). The preview copy is at most 1024 pixels on its longer side.
Right, the
Settings and Colors tabs over the shared settings panel (the field
colour, framed, each slider under its label and stretched to the
panel's width, a rule between groups; `before-after-preview.md`) and
the Reset button with the undo and redo arrows under it; bottom, the
help button, Options..., OK and Cancel. Enter is OK, Esc and the close
button cancel. Controls that do not apply are greyed, not hidden: a
centreline trace greys Corner smoothness, Remove and the Objects boxes;
Group by color is greyed while Merge adjacent is on (every colour is
one curve already, so no groups are made).

### Live preview

There is no Preview button: the trace is made again whenever a setting
changes, off the interface thread (`trace_preview.rs`), and the After
pane, the wireframe and the counts take the result when it comes in.

- Every request traces in two stages: a quick look on a copy at most 240
  pixels across (left out when the picture is no bigger), then the whole
  picture. The quick look shows within tens of milliseconds; the whole
  picture's trace replaces it when ready, and OK places exactly that
  trace (not made again), so OK is immediate once the counts are in.
- A change while a trace runs cancels it and the new settings are
  traced instead; the pending request is always the latest, never a
  queue. The tracer looks at the flag all along: every few rows of the
  noise removal and of the colour matching, in each round of the colour
  reduction, every 4096 pixels of the despeckling, between colours,
  between the patches of a colour and between the rings fitted, between
  the passes of the thinning; so a stale trace ends within moments and
  the preview never stays busy on it. Closing the dialog raises the
  flag of the running trace too, and the results of a cancelled trace
  are never shown.
- While a slider is dragged (the pointer held) a new value waits 80 ms
  for the value to rest before it is traced; a click, a list, a check
  box or a release trace at once.
- Nothing is traced again while the settings that change the trace
  and the picture are the ones on show: Group objects by color, Delete
  original image, the background colour while nothing is removed, the
  number of colours in black and white, the outline settings of a
  centreline trace are not part of the request. A colour edited on the
  Colors tab is not traced again either: the whole picture's trace on
  show takes the new colours and is drawn again off the interface
  thread. A frame never waits for a trace (results arrive through a
  channel and the worker asks for a repaint); the picture is decoded
  once per bitmap, and the Colors tab's list of colours is worked out
  once with each result, not every frame.
- In the browser (no threads) the quick look is made in the frame; the
  whole picture is traced on OK when it is larger than 400 000 pixels.
  The quick look is then the request's last result: the preview rests
  on it (nothing is traced again while the settings stand, the spinner
  stops, an idle dialog asks for no frames) and the counts show it
  without the dots sign. Every result says whether it is the last of
  its request (`Shown::done`), so a result that will be followed by
  another never counts as the end.
- A trace whose thread ended without a result is not started again:
  the preview keeps what it shows until a setting changes.

### Remove

- Background color: the regions of the background colour that reach the
  picture's edge (4-connected flood from the border) are left out; the
  patches of that colour inside the picture (the white of an eye) are
  traced as curves of their own.
- Entire color from image: every pixel of the background colour is left
  out, inside the picture too.
- None: the background is traced like any colour.
- Automatically: the background colour is the commonest palette colour
  along the border. Specify: the colour of the swatch, or the colour
  under a click on the preview with the eyedropper (taken from the
  picture, so an anti-aliased or compressed colour still stands for the
  palette colour nearest it).

### Objects

- Merge adjacent objects of the same color (on): every region of a
  colour is one combined curve (its holes included). Off: each separate
  patch (8-connected) of a colour is a curve of its own, with its holes.
  In this tracer every region of a colour is found at once, so "adjacent"
  joins every region of the colour, not only the touching ones.
- Remove object overlap (off): a region reaches 2 px under the regions
  drawn over it (the underlap of #611), so no page shows between
  neighbours. On: each region is exactly its own pixels, as the counts
  of nodes then show; gaps can show between curves fitted on their own.
- Group objects by color (off): on, the trace's group holds a group per
  colour (in the order the colours stack), each with that colour's
  curves.

### Colors tab

Color mode (RGB, Grayscale: the picture's greys quantised to the number
of colours, Black and white: two colours by brightness), Number of
colors (2 to 256), `N color(s) in the trace`, then the colours of the
trace on show with their curve counts, each with a check box and a
colour swatch. Merge selected colors joins the ticked colours into the
first of them in the list's order (the pixels of the others take its
label before the regions are found, so one combined region comes out);
a colour of the list must match a palette colour within 12 (RGB
distance), so a palette changed by other settings never joins colours
by accident. The swatch of a row changes the colour its curves are
drawn in (kept as traced colour to shown colour). A change to what is
traced (detail, smoothing, corner smoothness, colours, mode, trace
type or image type) forgets the merges and the edits, which belonged to
the trace before; a step of the undo and redo arrows does not, it
brings back the settings of that step as they were, merges and edits
included.

### Reset, undo, redo

The settings as they stand after each change (a slider's value once the
pointer is released, not every value of a drag) are a step; the arrows
step back and forward through them, and a change after an undo drops
the steps undone; the arrows themselves never drop the steps to redo.
Reset puts the image type's settings back as a step of its own, so it
can be undone.

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
5. Stack regions dark to light; leave out the background (see Remove
   above) when requested.

Centreline mode: Zhang-Suen thinning, then the skeleton is traced into
polylines, smoothed (Catmull-Rom) and output as curves with a stroke
width equal to the mean local thickness.

## Checks

- `pick_drag_tests::the_trace_preview_follows_a_slider_without_a_button`:
  a click on the Detail slider changes the value and the trace on show
  is made again, with no Preview button in the dialog.
- `pick_drag_tests::changing_the_trace_type_swaps_the_image_types`:
  Centerline in the Trace type list chooses Technical illustration and
  the Image type list offers Line drawing, not Logo.
- `pick_drag_tests::the_eyedropper_specifies_the_background_colour`:
  Specify, the eyedropper and a click on the black part of the preview
  make black the background, so the white is traced instead.
- `pick_drag_tests::merge_adjacent_and_group_by_color_change_the_result`:
  two black squares are one curve, two with Merge adjacent off, and OK
  with Group by color places a group holding one group of the two; with
  Merge adjacent on again, Group by color is greyed (a click does
  nothing).
- `pick_drag_tests::the_trace_preview_list_offers_large_preview_and_the_wireframe_overlay`:
  Large Preview shows the trace alone in the whole area; Wireframe
  Overlay shows the Overlay slider (80 at first), clicks on it set the
  value, the slider goes with the other views, and the idle dialog in
  the overlay view asks for no frame and traces nothing again.
- `pick_drag_tests::reset_and_the_arrows_undo_the_settings`.
- `pick_drag_tests::the_arrows_keep_merged_colours_and_the_steps_to_redo`:
  two colours merged, then a new detail forgets the merge; undo brings
  it back with the step to redo kept, redo goes forward again.
- `trace_dialog::tests::only_settings_that_change_the_trace_change_its_key`.
- `pick_drag_tests::frame_times_of_the_trace_dialog` (ignored): frame
  times of a slider drag with a 2000 x 2000 picture in the dialog, how
  long the whole picture's trace takes to show, and the idle dialog's
  frames once it is in (how many ask for another frame).
- `pick_drag_tests::frame_times_of_the_trace_dialog_resting_on_the_quick_look`
  (ignored): the same where the whole picture waits for OK (the
  browser's way), idle on the quick look and dragging Detail.
- `pick_drag_tests::the_trace_dialog_rests_once_the_trace_is_in`: with
  the trace in, on the desktop and where the whole picture waits for
  OK, idle frames ask for no frame, start no trace and draw no spinner;
  a change traces once more and the dialog rests again.
- `trace_preview::tests`: a request gives the quick look then the whole
  trace, a cancelled one nothing, and the live preview traces each
  distinct request once, debounced while the pointer is held; new
  colours alone are not traced again
  (`new_colours_alone_are_not_traced_again`); closing the dialog ends
  the running trace soon (`clearing_stops_the_running_trace`); with the
  whole picture deferred the quick look is the last result
  (`with_the_whole_picture_deferred_the_quick_look_is_the_last_result`)
  and the preview rests on it (`a_deferred_request_rests_on_its_quick_look`);
  a trace that died is not started again
  (`a_trace_that_died_is_not_started_again`).
- `trace::tests::removing_the_background_or_the_entire_colour_or_nothing`,
  `trace::tests::merge_adjacent_off_makes_a_curve_per_patch`,
  `trace::tests::remove_overlap_keeps_regions_to_their_pixels`,
  `trace::tests::merged_and_edited_colours_and_grey_mode`,
  `trace::tests::a_cancelled_trace_gives_nothing`,
  `trace::tests::every_stage_stops_at_the_flag`,
  `trace::tests::a_trace_cancelled_while_it_runs_ends_soon`,
  `trace::tests::scaled_renders_and_wireframes_and_stats`.

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
