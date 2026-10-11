# Outline Pen and Outline Colour

The Outline Pen tool (F12) opens the Properties
panel on the Outline section and returns to the Pick tool; Outline
Colour (Shift+F12) opens the Colour panel. The model is
`tracedraw_core::Stroke`.

## Fields and defaults

| Field | Default | Notes |
| --- | --- | --- |
| Colour | black (C0 M0 Y0 K100 for new objects) | any colour model |
| Width | 0.2 mm (0.567 pt) for new objects; imported outlines keep theirs | a hairline (0.0762 mm, 0.216 pt) draws one device pixel at every zoom |
| Caps | butt | butt, round, square |
| Corners | mitre | mitre (limit 4), round, bevel |
| Dash | solid | presets dashed 4:2, dotted 1:1, dash dot 6:2:1:2, in multiples of the width |
| Arrowheads | none / none | start and end; presets and custom heads, filled in the outline colour |
| Nib stretch | 1.0 | 0.1 to 1: the calligraphic nib's thickness across its axis |
| Nib angle | 0 | degrees |
| Behind fill | off | the outline is drawn under the fill |
| Scale with object | off | the width follows the object's scale factor `sqrt(|det|)` |

## Calligraphic outline

- With stretch below 1 the outline is the area swept by an elliptical
  nib of `width` along the nib angle and `width * stretch` across it:
  each flattened segment contributes the parallelogram the nib's support
  sweeps (half-width `sqrt((a cos phi)^2 + (b sin phi)^2)` for the
  segment normal at angle `phi` to the nib axis), each vertex the nib
  itself, and the union is filled with the outline colour. Open paths
  end with nib-shaped caps; dashes are ignored.
- The same band is written by the SVG and PDF exports as a filled path,
  so the stroke looks the same outside the editor.
- A hairline is never calligraphic.

## Checks

- Given a 6 mm flat nib (stretch 0.1) at 0 degrees, a vertical line is
  6 mm wide and a horizontal line 0.6 mm tall.
- Given stretch 1, the band equals a 2 mm round-capped stroke.
- Given a rectangle with a 3 mm calligraphic outline, the SVG has the
  rectangle without a stroke and a filled band path in the outline
  colour.
