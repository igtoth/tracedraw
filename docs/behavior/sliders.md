# Sliders

Every slider in the bars, dockers and dialogs is the one widget
`ui::Slider` (`apps/tracedraw/src/ui/slider.rs`). Only the Color docker's
channel sliders, whose tracks show a colour gradient, and the Trace Bitmap
dialog's sliders still use the toolkit's slider.

## Look

- The label, when the slider has one, sits above the control, left
  aligned and ending with a colon ("Detail:").
- Under it the track: a thin groove, 3 points thick, in the theme's
  `slider_rail` colour, spanning the width of a column (a docker, a
  dialog's body) up to the number box; in a row of controls (a bar) it
  takes `Spacing::slider_width`, or the width the caller gives.
- Tick marks under the groove, like a ruler: minor ticks 3 points long,
  major ticks 6 points, in the theme's `ruler_tick` colour. A round major
  value (1, 2 or 5 times a power of ten) falls about every 45 points;
  minors divide a major into fifths (fourths for a 2) and are never
  closer than the step or than 3 points. Ticks lie at multiples of their
  spacing (0, 20, 40 for a 0 to 100 range), not at offsets from the
  range's start. A logarithmic slider ticks at 1, 2 and 5 of each decade.
  `.ticks(false)` draws none.
- The thumb: an upright slip 6 by 14 points, taller than the groove,
  with the button fill and the control border; the accent border when
  hovered, focused or dragged; the panel fill and the dim border when
  disabled. Everything is rounded to whole pixels at every scaling.
- The number box at the right of the groove (`NumField`, 64 points wide
  with its spin arrows) shows the value with its suffix (`%`, `pt`,
  `°`). `.show_value(false)` leaves it out. A mixed selection shows an
  empty box.

## Behaviour

| Input | Effect |
| --- | --- |
| Press or drag on the track | The thumb jumps to the pointer and follows it |
| Left/Right arrow (focused) | One step down/up (Up/Down on an upright slider) |
| Page Down/Up (focused) | Ten steps |
| Home/End (focused) | The range's start/end |
| Wheel over the track | One step a notch; Ctrl a tenth, Shift ten |
| Number box | Typing then Enter or leaving applies; the arrows and the wheel step |

The step is the one given (`step_by`), one for an integer slider, else a
round hundredth of the range (1 for 0 to 100, 0.05 for 0.1 to 5). Every
change snaps to a multiple of the step from the range's start and stays
in the range; the value handed to the slider is clamped to the range too.
The box and the thumb always show the same value: the box reads the
value each frame it is not being typed in.

`Response::changed()` is set by every way of changing the value;
`dragged()` and `drag_stopped()` come from the track, as before, so the
dockers that apply a value on release keep working.

## Checks

- Given a 0 to 100 percent slider in a 300 point column, when drawn,
  then the label "Detail:" is above it, the box reads "40 %", the groove
  leaves the box its room and 21 tick segments (majors every 20, minors
  every 5) lie under the groove.
- Given the slider, when the track is pressed at 75 %, dragged to 20 %,
  stepped with the arrows, Page Up, End and the wheel, then the value
  follows each and the box shows it.
- Given the slider, when "250" is typed into the box and Enter pressed,
  then the value is 100 (clamped); with a step of 25, a press at 60 %
  gives 50.
- Given a disabled slider, when pressed or wheeled, then the value stays.
- Given the Export dialog at JPEG, when its Quality track is pressed,
  then the export settings take the quality.
