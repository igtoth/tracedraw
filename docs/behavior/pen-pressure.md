# Graphics tablets and pen pressure

A graphics tablet's stylus moves the pointer and presses its button like
a mouse, so every tool works with it. Where the platform also reports how
hard the pen is pressed, the Freehand and Brush Strokes tools let that
pressure set the width of the stroke (#850). The mouse, and a pen whose
platform gives no pressure, draw exactly as before: an even width.

## Where the pressure comes from

The windowing layer (winit 0.30 through egui-winit 0.36) reports a pen as
a touch with a force in 0..1 (`egui::Event::Touch { force }`), and at the
same time moves the pointer and presses its primary button. TraceDraw's
raw input hook reads the force of the touch events of each frame
(`pen.rs`, `filter_raw_input`) before egui and the tools see them; a
touch that ends, or one with no force, clears it. The published winit does this on Windows
only; TraceDraw's patched copy (`vendor/winit`, see `vendor/README.md`)
adds X11 and macOS. What each platform gives:

| Platform | Pointer from the pen | Pressure |
|---|---|---|
| Windows (8 and later) | yes (`WM_POINTER` pen events) | yes, 0..1 from the pen's 0..1024 |
| Linux, X11 (also XWayland) | yes (XInput 2 motion) | yes: the pen's valuator labelled `Abs Pressure`, as a share of its range (patched copy) |
| Linux, Wayland | no: the windowing layer does not speak the tablet protocol (`zwp_tablet_v2`), and GNOME's compositor does not turn pen input into pointer input for clients that do not (its issue 2226 is open); KDE's did until Plasma 6.3 and now needs `KWIN_WAYLAND_EMULATE_TABLET=1` | no |
| macOS | yes | yes: the `pressure` of mouse events of the tablet point subtype (patched copy) |
| Browser | yes (pointer events) | no: the web layer sends pointer events without their pressure |

On X11 and macOS the patched copy sends the pressure as a touch of phase
Move only, never Start or End (those would press the button a second
time through egui-winit), with no force once the reading is at the
axis's minimum, so a lifted pen leaves no stale pressure. The hook then
takes out every moving touch whose id never started, so egui never sees
the pen as a touch: egui switches to touch screen mode for the whole
session on any touch (lists scroll by dragging, windows move from
anywhere, text in labels cannot be selected by dragging). A real touch
screen's touches start, move and end, and reach egui as before.

So on a Wayland desktop (Ubuntu's default) the stylus does nothing until
the window runs through X11: `tracedraw --x11` asks the windowing layer
for its X11 backend, so the window opens on XWayland, where the pen moves
the pointer, clicks and reports its pressure (XWayland lists the pen as
an XInput 2 device with the same `Abs Pressure` valuator). The
environment is left as it is, so programs TraceDraw starts (a browser
for a link) still run on Wayland. Without `DISPLAY` (no XWayland, no X
server) `--x11` stops at once with a message saying it needs XWayland or
an X11 session, and exits with status 2. `env -u WAYLAND_DISPLAY
tracedraw` opens the window on XWayland too.

A pen on Wayland itself needs the tablet protocol in winit; the browser
needs the pointer events' pressure in eframe's web layer. Neither needs a
native library of TraceDraw's own. An X11 touchpad driver that exposes
an `Abs Pressure` valuator for a finger (the old synaptics driver) would
drive the width too; the option below turns it off.

## What the pressure does

- Options > Tools > Curve tools > Graphics tablet > "Pen pressure sets the
  width of Freehand and Brush Strokes drags", on by default. Off, a pen
  draws like the mouse.
- Along a Freehand or Brush Strokes drag, the pressure at every sampled
  point is recorded with it. A drag that starts without pressure records
  nothing, so the stroke gets its usual even width. Once a drag has
  pressure, a point that comes without a reading (a move with no force,
  the pen lifting off) takes the last reading again.
- The recorded pressures are read by distance along the drawn polyline
  (before smoothing), and each width node averages the samples within
  half a node spacing of it, so the jitter of the pen's readings does
  not reach the outline; a node with no sample that close takes the
  nearest one.
- Freehand: the new line's outline gets 25 width nodes (`t` = 0, 1/24,
  ..., 1), each `w x (0.05 + 0.95 x pressure)` wide and never less than
  a hairline (0.0762 mm, or `w` when the outline is thinner still), `w`
  the default outline width: a feather-light touch leaves a hairline,
  full pressure the full width. A line that continues a selected curve keeps that
  curve's outline. The width nodes are the outline's variable width:
  the Shape tool edits the curve, and the Properties panel the outline,
  as for any variable-width line.
- Brush Strokes: the pen scales the width of the Preset, Brush and
  Expression modes by `(1 - p) + p x pressure`, `p` the Brush Strokes
  panel's Pressure setting (0.5 by default: half the width at no
  pressure, the full width at full pressure; 1: zero to full). Preset
  and Brush keep their profile along the stroke (the taper, the wave,
  the jittered edge) and multiply it by the pen's scale; Expression
  takes the pen's pressure in place of the drawing speed. The
  calligraphic nib and the sprayer's objects keep their size.
- Apply to curve (Brush Strokes panel) has no pen: it builds the stroke
  as before.
- Nothing is done per frame beyond reading the frame's touch events (a
  frame without any costs one scan of its events) and pushing one number
  per sampled point: the stroke's preview stays the
  thin line, and the widths are built once, on release.

## Checks

- Given a Freehand drag with a pen pressed at 0.9 over the first half
  and 0.2 over the second, then the new line's outline has width nodes
  more than twice as wide in the first third as in the last, never
  wider than the outline width; the same drag with the mouse, or with
  the option off, gives an outline without width nodes (test
  `pen_pressure_sets_the_width_of_a_freehand_line`).
- Given Brush Strokes in Expression mode at width 6 mm and Pressure 1,
  when a pen presses 1.0 then 0.25 along an even drag, then the stroke
  is 6 mm wide at the start and less than 2/5 of that at the end; the
  mouse's stroke takes its widths from the speed, as before (test
  `pen_pressure_drives_an_expression_stroke`).
- Given a pen that sends only moving touches (X11, macOS), pressed at
  0.9 then 0.2 with one reading without a force in the middle and none
  as it lifts, then the Freehand line is still wide then thin, no width
  node is below a hairline, and egui reports no touch screen (test
  `a_pen_that_only_sends_moving_touches_sets_the_width`).
- The pressure follows the touch events (a start or move with a force
  sets it, a move without one, an end or a cancel clears it, readings
  are clamped to 0..1); moving touches that never started give their
  pressure and never reach egui, whose `has_touch_screen` stays off,
  while a touch screen's start, moves and end reach it; a stroke repeats
  its last pressure at a point without one; a Freehand line is never
  thinner than a hairline; and the profile averages the samples within
  the window and scales the width between the floor and the full width
  (tests in `pen.rs`).
