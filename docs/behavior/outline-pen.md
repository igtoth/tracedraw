# Outline Pen and Outline Colour

The Outline Pen tool (F12) opens the Outline Pen dialog and gives the
canvas back to the tool before it; Outline Colour (Shift+F12) opens the
Colour panel. The model is `tracedraw_core::Stroke`.

## Outline Pen dialog

(#357, `ui/outline_pen.rs`) On the first selected object's outline,
values that differ across the selection shown empty, or on the default
outline when nothing is selected. Groups:

- General: Color (the colour picker), Width (a list with None, hairline
  and 0.25 to 24 pt, a box for any width, the outline unit), Style (the
  dash patterns drawn, then custom ones; "..." opens Edit Line Style and
  comes back), Mitre limit (with mitred corners), Dashes (as they come,
  or aligned to corners and ends), Corners, Line caps and Position as
  picture buttons.
- Arrowheads: start and end.
- Calligraphy: Stretch (10 to 100 %), Angle (-90 to 90 degrees), Default
  (100 %, 0), and a picture of the nib; dragging on it sets angle and
  stretch. Behind fill, Scale with object.
- Variable Outline: the width nodes as dots along a bar; a click on the
  bar adds a node with the width there, on a dot picks it (the nearest
  within 6 points); a drag on a dot slides it along the bar, 0 to 100 %,
  kept between its neighbours, the fields following, and a drag on bare
  bar adds a node there and slides it (a drag there moved the dialog,
  #652); its position (a position typed past a neighbour puts the nodes
  in order again), Side 1 and Side 2 widths and their Total (a new total
  keeps their ratio); Delete Node
  (`outline_pen::tests::a_width_node_drags_along_the_bar_between_its_neighbours`,
  `a_slid_node_stops_at_its_neighbours_and_the_ends`).
- Overprint outline, "?", OK, Cancel.

OK sends only the fields that were changed (also those set to the value
shown) to every selected object, with Overprint in the same undo step
(`App::apply_outline_pen`, `the_outline_pen_applies_what_was_changed`);
with nothing selected it changes the default outline (asking which
kinds when Options say so).

While the dialog is open the selected objects show what it holds (#741,
`App::preview_outline_pen`): each change in the dialog is drawn at
once, as an engine preview made again only when a value changes (never
every frame). OK keeps it as one undo step; Cancel, Esc and the close
button take it back, the drawing, the history (redo included) and the
saved state as they were. Edit Line Style opened from the dialog takes
the preview back, and it shows again on the return. With nothing
selected (the default outline) nothing is previewed.

## Fields and defaults

| Field | Default | Notes |
| --- | --- | --- |
| Colour | black (C0 M0 Y0 K100 for new objects) | any colour model |
| Width | 0.2 mm (0.567 pt) for new objects; imported outlines keep theirs | a hairline (0.0762 mm, 0.216 pt) draws one device pixel at every zoom |
| Caps | butt | butt, round, square |
| Corners | mitre | mitre, round, bevel |
| Mitre limit | 45 degrees | 1 to 179 degrees; shown with mitred corners |
| Dash | solid | presets dashed 4:2, dotted 1:1, dash dot 6:2:1:2, and custom line styles, in multiples of the width |
| Align dashes | off | fit the dashes to the corners and ends (with a dash pattern) |
| Position | centred | outside, centred, inside |
| Arrowheads | none / none | start and end; presets and custom heads, filled in the outline colour |
| Nib stretch | 1.0 | 0.1 to 1: the calligraphic nib's thickness across its axis |
| Nib angle | 0 | degrees |
| Behind fill | off | the outline is drawn under the fill |
| Scale with object | off | the width follows the object's scale factor `sqrt(|det|)` |

## Properties docker

The Properties docker's outline section (the default outline with
nothing selected, the selection's outline otherwise) shows the common
settings in view: Paint, Hairline and the width, Caps, Corners, the
start and end arrowheads, Dash (and custom dashes), Align dashes, Edit
Line Style, Position, Behind fill and Scale with object. The mitre limit
(with mitred corners), the nib stretch and the nib angle sit under an
Advanced header below them, closed at first unless one of them is set
away from its default (45 degrees, 100 %, 0 degrees) or differs between
the selected objects, so nothing set is hidden (#487,
`ui::dockers::tests::every_outline_option_is_in_the_properties_docker`).

## Width unit

Widths are kept in millimetres. The Properties docker's outline section
and the property bar show and take them in the outline unit, picked
next to the width: points (the default), millimetres, centimetres,
inches or pixels. The choice is a preference (`outline_units` in the
settings) shared by both places and the callout bar. The width list
offers round values in that unit: points 0.25, 0.5, 0.75, 1, 1.5, 2, 3,
4, 6, 8, 10, 12, 16; millimetres (and centimetres) 0.1 to 3 mm; inches
0.003 to 0.1; pixels 1 to 16. A number typed with a unit after it
(`1 mm`) is read in that unit. Checked by
`ui::propbar::tests::outline_widths_read_in_the_outline_unit` and
`ui::propbar::tests::the_width_presets_follow_the_outline_unit`.

## Mitre limit

A mitred corner sharper than the mitre limit angle is bevelled. The
angle `a` is kept in the model (`Stroke::miter_limit`, degrees) and
given to the renderer, SVG (`stroke-miterlimit`), PDF (`M`) and EPS
(`setmiterlimit`) as the ratio of mitre length to width
`1 / sin(a / 2)` (45 degrees: 2.6; 5 degrees: 22.9). New outlines start at 45 degrees, so a thick outline on lettering bevels its narrow turns instead of growing spikes several widths long (#894; the reporter's comparison video shows new outlines with a limit of 45 degrees). They started at 5 degrees before. SVG files read back the other way:
`a = 2 asin(1 / ratio)`. Outlines in files saved before the field
existed keep the limit they were drawn with, ratio 4 (about 29
degrees). `.cdr` files: the limit is neither read nor written (where it
lives in the outline record is not known); outlines read from them take
the default.

## Position

For the closed subpaths of an object the outline can lie wholly
outside the path (the object looks larger by the width), centred on it
(half on each side, the default) or wholly inside it (the object keeps
its size). Open subpaths are always centred. Hairlines, calligraphic
and variable outlines are always centred.

The area of an inside or outside outline is the outline stroked at
twice the width (dash lengths kept in millimetres), intersected with
the object's even-odd fill area (inside) or with that area cut away
(outside), and merged with the centred outline of any open subpaths
(`shaping::outline_band`). The renderer fills this area instead of
stroking, and SVG and PDF write it as a filled path in the outline's
paint (no stroke on the object), so exports look the same as the
screen. Curves are flattened at 0.02 mm for the cut.

## Align dashes

With Align dashes a dash pattern is fitted to each run of the path
between corners (a turn of more than 3 degrees between neighbouring
segments; smooth nodes do not split a run). A run of length `L` with
pattern length `P` (in mm: the pattern times the width) takes
`n = max(1, round(L / P))` repeats, stretched by `L / (n P)`, and starts
and ends with half the first dash. The halves that meet at a corner,
and at the start of a closed subpath, join into one dash, so every
corner and every end of an open line lies inside a dash
(`shaping::aligned_dashes`). Aligned dashes are drawn and exported as
filled areas, like an inside or outside outline.

## Edit Line Style

The Edit line style button (Properties docker, outline section, and at
the end of the property bar's line style list) opens a grid of cells,
one cell per outline width (2 to 32 cells): clicking a cell switches it
between dash and gap. The pattern is read from the cells as on and off
runs, starting at the first dash after a gap; it needs at least one
dash and one gap. Add keeps it as a new line style, Replace changes the
one picked in the list, Delete forgets it (outlines that use it keep
their pattern). Custom line styles are kept in the settings
(`custom_dashes`, at most 64) and listed after the presets.

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

- `ui::outline_pen::tests::the_outline_shows_live_while_the_dialog_is_open`
- Given a selected rectangle and the dialog open, when the width is set
  to 2 mm, then the rectangle's outline is 2 mm at once, idle frames do
  not change the drawing, Esc puts back the old width and history, and
  OK keeps the new width as one undo step.
- Given a 6 mm flat nib (stretch 0.1) at 0 degrees, a vertical line is
  6 mm wide and a horizontal line 0.6 mm tall.
- Given stretch 1, the band equals a 2 mm round-capped stroke.
- Given a rectangle with a 3 mm calligraphic outline, the SVG has the
  rectangle without a stroke and a filled band path in the outline
  colour.

## Outline paint

The Properties docker's outline section has Paint with every kind a fill
has, as the same row of pictures and the same editors: None, Uniform,
Gradient, Pattern, Texture and Mesh.

- None takes the outline away (as clearing the Outline box does).
- Uniform is the outline's colour (`Stroke::color`).
- The other kinds give the outline its own fill (`Stroke::fill`), edited
  as an object's fill is (for a gradient: type, built-in gradients,
  stops, repeat, bilinear, invert). It spreads over the outline's bounds
  (the path's bounds grown by half the width) and paints only the
  outline's area: the stroke, or the band of a calligraphic or variable
  outline. A mesh's nodes are local to the object, as a mesh fill's are;
  a shaped gradient follows the outline's own area (dark at its edges
  when the first colour is dark).
- Hairlines, arrowheads and painterly marks keep the outline colour.

Exports:

| Format | Outline with its own paint |
|---|---|
| SVG | `stroke` refers to the gradient or pattern (linear, radial; conical and square as linear, shaped as radial); a texture or mesh strokes with a representative colour (the mean of its colours). A calligraphic band is filled with the paint. |
| PDF | the outline's area (`shaping::stroke_area`: width, caps, joins, miter limit 4, dashes) clipped with the non-zero rule and shaded (gradients) or painted with the renderer's pixels at 150 dpi (patterns, textures, meshes, shaped gradients). |
| EPS, EMF and the rest | the outline colour. |
| `.cdr` | the outline colour (outline fills are not written). |

Position, mitre limit and dash alignment are checked by
`shaping::tests::inside_and_outside_outlines_cover_one_side_of_the_path`,
`shaping::tests::aligned_dashes_put_corners_and_ends_on_dashes`,
`style::tests::position_mitre_limit_and_dash_alignment_round_trip`,
`style::tests::mitre_limit_angles_and_ratios_convert_both_ways`, the
renderer's `tests::inside_outlines_stay_inside_the_fill`, the SVG
export's `svg::tests::positioned_outlines_are_written_as_filled_areas`,
the native format's round trip and `ui::line_style::tests`.

Checked by `tests::outlines_take_a_gradient_fill` and
`tests::outlines_take_pattern_texture_mesh_and_shaped_paints` in the
renderer, `svg::tests::outline_paints_are_written_as_the_stroke_paint`,
`pdf::tests::gradient_options_and_outline_paints_reach_the_pdf`,
`shaping::tests::stroke_area_covers_the_outline_and_hairlines_cover_nothing`
and `ui::dockers::tests::outline_paint_offers_every_fill_kind`.

