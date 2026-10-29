# Before and After preview

One preview serves every dialog that changes a picture: Trace Bitmap,
Remove Background, the colour mode dialogs (Black and White, Duotone,
Paletted), every bitmap effect dialog and Image Adjustments
(`apps/tracedraw/src/ui/before_after.rs`). Each dialog is a window the
user sizes (960 x 640 points at first, never under 720 x 480, a
maximize button in the title bar; Trace Bitmap keeps its 1040 x 700):
the preview takes the left of the window and grows with it, the
dialog's own settings sit in a column 340 points wide at the right
(320 for Trace Bitmap), OK and Cancel under them.

## Controls

| Control | Default | Meaning |
|---|---|---|
| Preview list | Before and After (Image Adjustments: After) | Before and After (the picture above, the result below), Before and After side by side, Before, After; Trace Bitmap adds Wireframe overlay (the trace's curves over the picture), Image Adjustments adds Before and After split (one picture, the original left of a dashed divider that drags, the result right) |
| Zoom to fit | pressed | the whole picture in a pane |
| Zoom out, Zoom in | | the zoom divided or multiplied by 1.25, between a tenth of the fit and 64 points per preview pixel |
| Pan | pressed | a drag on a pane pans; while a dialog's eyedropper picks on the preview the button is released, and clicking it ends the picking |
| Busy spinner | | after the pan button while the dialog is working something out off the interface thread (the trace) |
| Image Adjustments | | Rotate left, Rotate right (the view, a quarter turn) and 100 % (one bitmap pixel per screen point) after the shared buttons |

The two panes of a view share one zoom and one place: a drag in either
pans both, the wheel zooms both about the pointer (the point under it
stays where it is), and the labels Before and After sit in the panes'
corners. Transparent parts show a checkerboard (one 2 x 2 texture
repeated, a single rectangle a frame). Under the panes a dialog may
have a status line (Trace Bitmap's curves, nodes and colours).

## Cost

- The preview copy is as large as the panes show: the longer side of a
  pane in pixels, rounded up to a multiple of 256 (so a resize by a few
  points makes no new copy), at most 768 pixels (1024 for Trace Bitmap,
  whose trace is of the whole picture anyway). The copy is made once
  per source and size and kept.
- The result is worked out once per change of the settings (or of the
  copy) and kept with its texture; a frame draws the textures and
  nothing else. While the pointer is held outside the panes (a slider
  dragged) the result is made on a copy at most 240 pixels across, so
  the preview follows the slider at once, and the shown size follows
  when the pointer is released; the last result stays on show until the
  new one is in, so nothing flickers.
- Trace Bitmap's trace is made off the interface thread
  (`trace_preview.rs`), the picture decoded once; see
  `bitmap-tracing.md`.
- The view state (view, zoom, place, turns, divider) is part of the
  dialog's state (Trace Bitmap, the effect dialogs) or kept between
  openings for the dialogs whose state is their settings alone.

## Checks

- `before_after::tests::turned_points_map_back`: a quarter turn maps
  preview points back to the same point of the picture.
- `before_after::tests::the_copy_side_follows_the_pane`: 700 x 600
  points of panes ask for a 768 pixel copy, stacked or side by side; a
  300 x 500 pane for 512; the cap holds.
- `before_after::tests::panes_share_the_area_and_fit_the_picture`:
  stacked panes split the height, side by side the width, and the fit
  zoom shows the whole picture in a pane, turned too.
- `before_after::tests::results_are_made_once_per_key`: a result is
  made once per key and found again without being made.
- `pick_drag_tests::every_picture_dialog_has_the_shared_preview`: Remove
  Background, Black and White, Duotone, Paletted, an effect dialog,
  Image Adjustments and Trace Bitmap all draw the Preview list, the
  zoom and pan buttons and the panes; the zoom buttons and the fit
  button change the zoom, the Preview list shows one pane for Before,
  a drag pans (the place changes), and ten idle frames allocate no new
  texture and ask for no frame (nothing is worked out per frame).
- `pick_drag_tests::the_remove_background_eyedropper_picks_on_the_shared_preview`:
  the eyedropper's click on a pane adds the colour under it, and the
  pan button ends the picking.
- `pick_drag_tests::the_split_views_divider_drags`: the divider drags
  where the button went down on it; a drag elsewhere pans.
