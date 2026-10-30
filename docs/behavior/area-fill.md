# Area Fill

Fills any enclosed area, even one formed by overlapping objects, by
creating a new object from the area under the click. The objects that
enclose it stay as they are.

## Property bar

| Control | Choices | Default |
|---|---|---|
| Fill | Use default (the default fill for new objects; a colour when that is none, so the area shows), Specify (the colour beside it), None | Use default; the colour is yellow (C0 M0 Y100 K0) |
| Outline | Use default (the default outline for new objects), Specify (the width and colour beside it), None | Use default; 0.2 mm black |

The colour buttons open the colour palette's swatches and, under Other,
a swatch that opens the colour picker beside the button for any other
colour (see `contour.md`, #1262). The choices are kept in the preferences
(`tools.smart_fill_mode`, `smart_fill_color`, `smart_outline_mode`,
`smart_outline_width`, `smart_outline_color`).

## Algorithm

1. The outlines: every outline of every visible object on the page
   (stroked or not, open curves too, a curve crossing itself enclosing
   the areas between its crossings), groups opened up; an object with
   live effects counts as it is drawn (its warped outline and the
   objects its effects add). The area may lie anywhere, on the page or
   off it on the desktop, in a drawing of any size (#499).
2. The end of an open outline that stops within 0.01 mm of another
   outline (or of its own start) is joined to it by a short line, so a
   drawing whose lines nearly meet, as files carry from rounding,
   encloses its cells all the same (#899). A wider gap stays open.
3. The outlines, flattened to 0.02 mm, cut a box around them all (their
   bounds and 2 mm more) into faces (`shaping::enclosed_area`). The face
   under the click is the area; a face that reaches the box's margin is
   open (nothing encloses it), and a click there or outside the box
   makes nothing and the status bar says so; a click exactly on an
   outline makes nothing.
4. The outlines' own curves are put back into the face's sides, as the
   Shaping commands do: the new object follows the outlines exactly,
   its sides the same Bezier segments, with nodes only where outlines
   cross (#734; it was traced from pixels before, which gave jagged
   sides with many nodes). Outlines inside the area make holes in it.
   The object is put on top of the active layer, selected, with the
   fill and outline the bar says; one undo step.

## Checks

- `ui::fill_tool_bars::tests::smart_fill_takes_the_bars_fill_and_outline`
  (a 40 mm square's inside becomes a 40 mm object; the bar's fill and
  outline, its own colour and outline, none).
- `ui::fill_tool_bars::tests::smart_fill_works_off_the_page_in_a_large_drawing`
- `shaping::tests::enclosed_area_keeps_the_outlines_curves`
- `shaping::tests::enclosed_area_closes_hairline_gaps` (lines 0.0002 mm
  short of a frame divide it; 0.2 mm short they do not; an open ring
  closed by a hairline is an area of its own)
- `pick_drag_tests::area_fill_keeps_the_curves_it_fills_between`
- Given a circle 60 mm across, when the Area Fill tool clicks inside it,
  then the new object has four curves, every point within 0.02 mm of the
  circle; given the circle overlapping a rectangle, a click in the
  overlap makes exactly the overlap, its round side the circle's curve.
