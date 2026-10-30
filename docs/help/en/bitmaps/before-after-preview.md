# The Before and After preview

Every dialog box that changes a picture shows the picture before and after
the change in the same preview: Trace Bitmap, Remove Background, the color
mode dialog boxes (Black and White, Duotone, Paletted), every bitmap effect
and [[fx.image_adjustments]]. This page explains how to look at the picture
there.

## The dialog box {#layout}

These dialog boxes are windows you size: drag an edge or a corner, or click
the maximize button in the title bar. The preview fills the left of the
window and grows with it; the dialog box's own settings sit in a white
panel on the right, each label above its slider and the sliders as wide as
the panel, with [[dialog.ok]] and [[dialog.cancel]] under them. Under the
panel, [[dialog.reset]] puts the settings back and the two arrows beside
it undo and redo the changes made in the dialog box: each slider release,
list choice, check box and Reset is one step. A change made after undoing
drops the steps that were undone.

The preview follows every change of the settings by itself. While you drag a
slider it shows a quick, smaller version at once, and the full view when you
let go. Nothing is worked out again while the settings stay the same, so the
dialog box stays quick on large pictures.

## Choosing the view {#views}

The [[dialog.trace_preview_label]] list above the picture chooses what the
preview shows:

- [[dialog.trace_view_before_after]] (the default): the picture above and
  the result below.
- [[dialog.view_side_by_side]]: the picture on the left and the result on
  the right.
- [[dialog.before]]: only the picture as it is.
- [[dialog.after]]: only the result. Trace Bitmap calls this view
  [[dialog.trace_view_large]]: the trace alone, as large as the preview.
- [[dialog.trace_view_wireframe]] (Trace Bitmap only): the curves of the
  trace drawn as thin lines over the picture, to see how closely they
  follow it. An [[dialog.trace_overlay]] slider appears beside the list: it
  sets how strongly the picture shows under the lines, from 0 (the lines
  alone) to 100 (the picture as it is), 80 at first.
- [[dialog.view_split]] ([[fx.image_adjustments]] only): one picture, the
  original on the left of a dashed line and the result on its right. Drag
  the line to move it. [[fx.image_adjustments]] starts on [[dialog.after]].

The labels Before and After sit in the corners of the panes. Transparent
parts of the picture show a checkerboard.

## Zooming and panning {#zoom-pan}

The buttons beside the list:

- [[dialog.lab_zoom_fit]] (pressed at first) shows the whole picture in a
  pane.
- [[dialog.lab_zoom_out]] and [[dialog.lab_zoom_in]] zoom out and in by a
  quarter each click.
- The pan button (pressed at first) lets a drag on a pane move the picture.

The wheel over a pane zooms about the pointer, so the spot under it stays in
place. The two panes always share one zoom and one place: panning or zooming
one moves the other the same way, so you compare the same spot.

[[fx.image_adjustments]] also has [[dialog.lab_rotate_left]] and
[[dialog.lab_rotate_right]], which turn the view a quarter turn (the picture
itself does not turn), and 100 %, which shows one pixel of the picture per
point of the screen.

A spinning mark after the pan button means the dialog box is still working
out the result, such as a trace of a large picture; the window keeps
answering meanwhile.

## Picking colors on the preview {#eyedropper}

Some dialog boxes have an eyedropper that picks a color from the picture,
such as the background color to remove. While it is on, a click on a pane
takes the color under the pointer and the pan button is released. Click the
pan button to stop picking.

See also [Tracing bitmaps](tracing.md),
[Removing the background](remove-background.md),
[Bitmap color modes](color-modes.md) and
[Bitmap effects](../effects/bitmap-effects.md).
