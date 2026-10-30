# Drawing with a graphics tablet

A graphics tablet's pen moves the pointer and clicks like a mouse, so every
tool works with it. Where the system also tells TraceDraw how hard the pen
presses, the Freehand and Brush Strokes tools let that pressure set the width
of the stroke: press lightly for a thin line, harder for a wide one. The
mouse draws an even width, as before.

## Drawing with pressure {#draw}

1. Choose the [[tool.freehand]] tool, or the [[tool.brush_strokes]] tool in
   the Preset, Brush or Expression mode.
2. Draw with the pen, pressing harder where the stroke should be wider.

- **Freehand:** the new line gets a variable-width outline that follows the
  pressure along it. A feather-light touch gives a hairline, full pressure
  the full outline width of new lines. You can edit the line with the Shape
  tool and its outline in the [[docker.properties]] panel, as any line with
  a variable width (see
  [Outlines](outlines.md#variable-outline)).
- **Brush Strokes:** the pen scales the width of Preset, Brush and Expression
  strokes. The [[docker.pressure]] setting of the Brush Strokes panel (0.5 at
  first) says how much the width follows the pen: at 0.5 a stroke with no
  pressure is half as wide as at full pressure; at 1 it goes from nothing to
  the full width. Preset and Brush keep their shape along the stroke (a
  taper, a wave) and scale it by the pen; Expression takes the pen's
  pressure in place of the drawing speed. Calligraphic strokes and the
  sprayer's objects keep their size.

The stroke shows as a thin line while you draw and takes its widths when you
lift the pen. [[docker.apply]] to a curve in the Brush Strokes panel has no
pen, so it builds the stroke without pressure.

## Turning pressure off {#option}

The pressure is on at first. To draw with the pen at an even width, turn off
[[options.pen_pressure]] under [[options.pen_tablet]] on the
[[options.curve_tools]] page of
[[menu.tools > menu.tools.options > menu.tools.options_tools]].

## Which systems give the pressure {#systems}

| System | The pen moves the pointer | Pressure |
|---|---|---|
| Windows 8 and later | yes | yes |
| Linux with X11 | yes | yes |
| Linux with Wayland | no (see below) | no |
| macOS | yes | yes |
| Browser version | yes | no |

On a Wayland desktop (the default of several Linux systems) the pen does not
reach TraceDraw yet. Start TraceDraw with `tracedraw --x11` from a terminal:
the window then opens through XWayland, where the pen moves the pointer,
clicks and reports its pressure.

> **Tip:** For more control over the stroke afterwards, draw with pressure,
> then adjust the width nodes with the [[tool.variable_outline]] tool.
