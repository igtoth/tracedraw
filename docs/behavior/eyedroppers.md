# Colour Eyedropper and Attributes Eyedropper

Both tools have two modes shown on the property bar: Select (sample)
and Apply. A sample switches to Apply; the Select button or Esc goes
back. Esc also drops the sample and returns to the Pick tool. The
pointer is the pipette in Select mode and the paint bucket in Apply mode
(the pipette again while Shift is held with the attributes eyedropper,
whose Shift+click samples; `ui::cursors` tests, #403).

Colours and attributes are read from the innermost object under the
pointer: a group's child, or a clip frame's content inside the frame (the
frame itself where no content is under the pointer), their transforms
composed (`App::leaf_to_read`, #403).

Both eyedroppers, and the document palette's, read from any visible
object under the pointer, a locked one or one on a locked layer too
(reading leaves it as it is; `App::hit_test_to_read`, #365). Applying a
sampled colour or attributes never changes a locked object.

## Colour Eyedropper

- Property bar: Select Color, Apply Color, sample size (1x1, 2x2, 5x5
  pixels, for bitmaps), the sampled swatch with its description.
- Select mode, click on an object:
  - a bitmap gives the pixel under the pointer, averaged over the
    sample box (box anchored at the pointer pixel, clipped to the
    image), as RGB, whatever format the picture keeps (PNG, JPEG ...);
  - a solid fill gives its colour; a gradient fill gives its first
    stop; otherwise the outline colour;
  - Shift+click reads the outline colour first.
- Apply mode, click on an object: a solid fill with the sampled colour
  (`SetFill`); Shift+click sets the outline colour instead, creating a
  hairline outline when the object has none (`SetStroke`).
- The object to paint is the usual hit test, else the topmost unfilled
  closed object whose interior contains the point, so an empty
  rectangle can be filled by clicking inside it.

### Live readout (#990)

While the Colour Eyedropper is in Select mode, and while a colour
picker's eyedropper button waits for a click on the drawing, a box
beside the pointer shows the colour a click would take
(`App::hover_reading`, `ui::cursors::paint_color_readout`):

- a 14-point swatch of the colour;
- its name, in bold, when it has one: a spot colour's ink, else the first
  colour of the default palette or the palettes shown that is exactly
  equal (`ui::palette::name_of`);
- its values in the drawing's primary colour mode (Document Options;
  CMYK: C, M, Y, K in %, RGB: R, G, B 0 to 255), one per line, converted
  when the colour is in another model; spot colours show their tint and
  Registration its name;
- its hex value, upper case (`#E02020`).

The colour is the one the click would take: for the tool what
`sample_color_at` reads (Shift: the outline first); for a picker the
fill, else the outline, else white over empty paper. Off every object
the tool shows no box (a click there takes nothing). Apply mode, a held
button and the other tools show none.

The box sits 18 points right of and 20 below the pointer, on the
other side where the window ends, on the tooltip layer. Lag: the reading
is worked out again only when the pointer moves (to 1/1000 mm), the zoom,
page or drawing change, or Shift is pressed or released; the last
picture read stays decoded, so a bitmap is decoded once while the
pointer moves over it. The box is two rectangles and a few short texts,
with no render of the page. In `frame_times_of_the_eyedropper_readout`
(3000 objects and a 1200 x 900 picture, release build) a hover frame
costs the same with the eyedropper as with the Pick tool (about 0.85
ms on the test machine).

## Attributes Eyedropper

- Property bar: Select Object Attributes, Apply Object Attributes, the
  Properties menu (Outline, Fill, Text; all on by default), the
  Transformations menu (Size, Rotation, Position; off by default) and
  Effects (off by default).
- Select mode, click: reads fill, outline, the first text span's font
  (family, size, bold, italic), bounds width and height, rotation of the
  transform (angle of its x axis), bounds centre, opacity, drop shadow
  and live effects.
- Apply mode, click (Shift+click samples again instead): one batch
  "Copy Attributes" with the ticked groups.
  - Fill and Outline: `SetFill`, `SetStroke`.
  - Text: when both objects are text, every span of the target takes
    the sampled font; the text itself is kept.
  - Effects: opacity, drop shadow and the live effect list.
  - Size, Rotation, Position, in that order about the target's bounds
    centre: rotate by the angle difference, scale the bounds to the
    sampled width and height, then move the centre to the sampled
    position. One `TransformShapes` carries the three.

## Checks

- Given a red rectangle and an empty one, when the colour eyedropper
  clicks the red one then inside the empty one, then the empty one has
  a solid red fill; a Shift+click afterwards gives it a red outline.
- Given a 10 x 20 mm blue rectangle at 50 % opacity and a 10 x 10 mm
  empty one, with default groups the second gets the blue fill and keeps
  its size and opacity; with Size and Effects ticked it becomes 10 x 20
  mm about its own centre at 50 %.
- `eyedropper::tests::eyedroppers_read_from_locked_objects_but_never_write_to_them`
- `eyedropper::tests::pictures_give_their_pixels_wherever_they_are` (alone,
  grouped, inside a clip frame, kept as JPEG)
- `eyedropper::tests::the_hover_readout_names_the_colour_and_keeps_its_work`:
  name, CMYK or RGB values and hex; nothing worked out again while
  nothing changes; a picture decoded once; white for a picker over paper.
- `pick_drag_tests::the_colour_eyedropper_reads_out_the_colour_under_the_pointer`:
  the box drawn in the window over an object, not off the objects nor in
  Apply mode; a picker's eyedropper shows it too.
