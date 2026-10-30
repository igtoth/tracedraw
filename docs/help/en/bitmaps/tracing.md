# Tracing bitmaps

Tracing turns a bitmap into vector shapes that you can edit, recolor and
scale without losing quality: a scanned logo, a sketch, a photo.

## Quick trace {#quick-trace}

Select a bitmap and choose [[menu.bitmaps > menu.bitmaps.quick_trace]]. It uses
the preset chosen on the [[options.bitmap_tracing]] page of the global options
(Clipart at first).

[[menu.bitmaps.repeat_last_trace]] traces the selected bitmap again as the last trace did, without the dialog box, and [[menu.bitmaps.last_trace_settings]] opens the dialog box with the last trace's settings. A bitmap's right-click menu has these commands too, with [[menu.bitmaps.centerline_trace]] and [[menu.bitmaps.outline_trace]].

## The Trace Bitmap dialog box {#trace}

[[menu.bitmaps > menu.bitmaps.outline_trace]] and
[[menu.bitmaps.centerline_trace]] list presets; choose one to open the trace
dialog box.

- **Outline trace** fills areas of color with shapes:
  [[menu.bitmaps.trace_line_art]], [[menu.bitmaps.trace_logo]],
  [[menu.bitmaps.trace_detailed_logo]], [[menu.bitmaps.trace_clipart]],
  [[menu.bitmaps.trace_low_quality]] and [[menu.bitmaps.trace_high_quality]].
- **Centerline trace** follows lines with open curves:
  [[menu.bitmaps.trace_technical]] and [[menu.bitmaps.trace_line_drawing]].

![The Trace Bitmap dialog box](../../img/bitmaps-tracing.png)

The dialog box is a window you can resize and maximize. The preview fills
its left side, the settings sit in a panel at the right, and the trace is
made again whenever you change a setting: there is nothing to press to see
the result. Under the preview a line counts the **curves**, **nodes** and
**colors** of the trace. Click [[dialog.ok]] to place the shapes over the
bitmap, darkest first, or [[dialog.cancel]] to leave the bitmap as it is.

## The preview {#preview}

The [[dialog.trace_preview_label]] list chooses what the preview shows:

- [[dialog.trace_view_before_after]]: the bitmap above and its trace below,
  or [[dialog.view_side_by_side]]; [[dialog.before]] shows the bitmap alone.
- [[dialog.trace_view_large]]: the trace alone, as large as the preview.
- [[dialog.trace_view_wireframe]]: the curves of the trace drawn as thin
  lines over the bitmap. An [[dialog.trace_overlay]] slider appears beside
  the list: it sets how strongly the bitmap shows under the lines, from 0
  (the lines alone) to 100 (the bitmap as it is), 80 at first.

The buttons after the list zoom to fit, zoom out, zoom in and pan. The
wheel zooms around the pointer and a drag moves the picture; both panes
move together. Transparent parts show a checkerboard.

## Settings {#settings}

The [[dialog.trace_tab_settings]] tab holds:

- [[dialog.trace_type]] Outline or Centerline, and [[dialog.trace_image_type]]
  the preset. Choosing one sets the sliders below to the values that suit
  that kind of picture.
- [[dialog.detail]] (0 to 100): more detail keeps smaller areas and finer
  lines; less detail smooths specks and grain away.
- [[dialog.smoothing]] (0 to 100, 25 at first): smoother curves with fewer
  nodes.
- [[dialog.corner_smoothness]] (0 to 100): rounder corners.
- [[dialog.trace_remove]]: what to leave out of the trace. **Background
  color** removes the background where it reaches the edge of the picture,
  **Entire color from image** removes every area of that color, **None**
  traces everything. [[dialog.trace_remove_auto]] finds the background
  color along the picture's edge; [[dialog.trace_remove_specify]] uses the
  color of the swatch, which you can change, or pick from the preview with
  the eyedropper beside it.
- [[dialog.trace_objects]]: [[dialog.trace_merge_adjacent]] (on at first)
  makes one curve of every area of a color; off, each separate patch is a
  curve of its own. [[dialog.trace_remove_overlap]] keeps every shape to
  its own pixels; off, shapes reach a little under the ones above them, so
  no gaps show between neighbors. [[dialog.trace_group_by_color]] puts the
  curves of each color in a group of their own; it is available when the
  areas of a color are not merged already.
- [[dialog.delete_original]]: the bitmap goes when the trace is placed.

Settings that do not apply to the current trace type are grayed out: a
centerline trace reads only the detail and the smoothing.

## Colors {#colors}

The [[dialog.trace_tab_colors]] tab sets the [[dialog.color_mode]] (RGB,
grayscale, or black and white) and the [[dialog.trace_number_of_colors]]
(2 to 256), and lists the colors of the trace with the number of curves of
each. Tick two or more colors and click [[dialog.trace_merge_colors]] to
join them into the first one; click a color's swatch to draw its curves in
another color. Changing a setting that traces the picture again forgets
these edits, which belonged to the previous trace.

## Reset, undo and redo {#steps}

Under the settings panel, [[dialog.reset]] puts the preset's settings back,
and the two arrows step backward and forward through every change made in
the dialog box: each slider release, list choice, check box, color edit and
Reset is one step. A change made after undoing drops the steps that were
undone.

[[dialog.trace_options]] opens the [[options.bitmap_tracing]] page of the
global options, where you choose the quick trace preset and the choices
every trace starts with, and then returns to the dialog box. The
[[dialog.help_button]] button opens this page.
