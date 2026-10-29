# Transparency

| Control | Default | Range |
|---|---|---|
| Type | Uniform | None, Uniform, Gradient (linear, radial, conical, square, shaped), Pattern, Texture |
| Transparency | 50% | 0..100 |
| Merge mode | Normal | the 28 modes of the table below |
| Apply to | Fill and outline | fill, outline, both |
| Gradient stops | white at 0 %, black at 100 % | opacity 0..100 %, position 0..100 %, at least two stops |
| Foreground, Background (pattern, texture) | 100 %, 0 % | opacity 0..100 % each |

Gradient transparency uses the same geometry as gradient fills with
grayscale stops where white = opaque and black = transparent. The
object is rendered to a scratch layer, multiplied by the transparency
mask, then composited with the merge mode. A shaped gradient follows
the object's outline (`tests::shaped_transparency_follows_the_object`).
A gradient mask turns, mirrors and skews with its object as a gradient
fill does (see `gradient-fill.md`, #775).

The Transparency tool shows a gradient mask's nodes as Interactive Fill
shows a fill's (`fill_tool.rs`, #448): start, end and skew handle, the
nodes between and the midpoint slider of each blend; a drag across an
object sets the line (a gradient mask there already keeps its nodes), a
double click on the line adds a node and on a node removes it. Edits are
one undo step "Transparency" per drag; the fill is left alone
(`fill_tool::tests::the_transparency_mask_has_nodes_and_a_midpoint`).

## The Transparency section

The Transparency tool's property bar and the Transparency section of the
Properties docker share one set of controls (`toolbar.rs`,
`transparency_controls`): the type, what it applies to, the type's own
settings and the merge mode. A change goes to every selected object as
one undo step "Transparency", each object keeping its other settings
(see `mixed-selection.md`).

- Uniform: the amount slider.
- Gradient: the angle and the gradient type; in the docker also the list
  of the mask's stops, one row each with the stop's opacity (the grey of
  the stop: 100 % white and opaque, 0 % black and clear), its position
  along the gradient, Add stop (a new stop half way to the next one, in
  the grey the gradient has there) and Remove stop (two stops always
  stay). The list sends the whole run of stops to each selected gradient
  mask; an object with another mask gets the gradient. Stops that differ
  across the selection show as mixed
  (`dockers::tests::transparency_section_lists_stops_and_greys`,
  `mixed::tests::transparency_stops_and_greys_change_one_setting_at_a_time`).
- Pattern and Texture: Foreground and Background, the opacity of the
  mask's two greys (the tile's or texture's two colours), and Swap, which
  exchanges them. They go to each selected pattern or texture mask, the
  tile and the texture kept; an object with another mask gets the mask
  shown.

Uniform transparency with Normal is the object's opacity. With any
other merge mode it is kept as a mask of one grey (its lightness the
opacity, `mixed::uniform_mask`), drawn the same way, so Multiply and the
rest work on uniform transparency too, as in the reference program; the
Transparency part reads it back as Uniform with its amount and merge
mode (`mockup_tests::mockup_steps`).

## Merge modes

The object (the source, `s`) drawn through its mask is mixed with
everything beneath it (the backdrop, `b`) by the merge mode, each
channel in 0..1 unless the mode says otherwise. The mix applies where
both are present; where only one is, that one shows, as the usual
compositing of blend modes does (`tracedraw_render::composite_layer`).
The `.tdraw` name of a mode is its name in lower case without spaces
(`softlight`, `colordodge`).

| Mode | Result | SVG `mix-blend-mode` | PDF `/BM` |
|---|---|---|---|
| Normal | `s` | | |
| Add | `b + s`, clipped | | |
| Subtract | `abs(b - s)` (drawn as Difference) | difference | Difference |
| Difference | `abs(b - s)` | difference | Difference |
| Multiply | `b * s` | multiply | Multiply |
| Divide | drawn as Screen | screen | Screen |
| If Lighter | `max(b, s)` | lighten | Lighten |
| If Darker | `min(b, s)` | darken | Darken |
| Texturize | `b * L(s)`, `L` the source's lightness (0.2126 r + 0.7152 g + 0.0722 b) | multiply (nearest) | Multiply (nearest) |
| Hue | the source's hue with the backdrop's saturation and lightness | hue | Hue |
| Saturation | the source's saturation with the backdrop's hue and lightness | saturation | Saturation |
| Color | the source's hue and saturation with the backdrop's lightness | color | Color |
| Lightness | the source's HSL lightness `(max + min) / 2` with the backdrop's hue and saturation | luminosity (nearest) | Luminosity (nearest) |
| Invert | drawn as Exclusion | exclusion | Exclusion |
| And, Or, Xor | drawn as the Xor compositing operator | | |
| Red, Green, Blue | drawn as Normal | | |
| Behind | the source only where the backdrop is clear (destination over) | | |
| Screen | `b + s - b * s` | screen | Screen |
| Overlay | Hard Light with source and backdrop exchanged | overlay | Overlay |
| Soft Light | `s <= 0.5`: `b - (1 - 2 s) b (1 - b)`; else `b + (2 s - 1) (D(b) - b)`, `D(b) = ((16 b - 12) b + 4) b` for `b <= 0.25`, else `sqrt(b)` | soft-light | SoftLight |
| Hard Light | `s <= 0.5`: `2 s b`; else `screen(b, 2 s - 1)` | hard-light | HardLight |
| Color Dodge | `b = 0`: 0; `s = 1`: 1; else `min(1, b / (1 - s))` | color-dodge | ColorDodge |
| Color Burn | `b = 1`: 1; `s = 0`: 0; else `1 - min(1, (1 - b) / s)` | color-burn | ColorBurn |
| Exclusion | `b + s - 2 b s` | exclusion | Exclusion |

The SVG and PDF exports draw a non-uniform transparency at the mask's
mean opacity (they have no masks of their own here) and set the blend of
the table on a group (SVG, `style="mix-blend-mode:..."`) or a graphics
state (PDF, `/ExtGState /BM`) around the object; modes without a blend
draw normally, and PDF/X-1a flattens the object into a picture instead
(`svg::tests::merge_modes_go_out_as_mix_blend_mode`,
`pdf::tests::transparency_merge_modes_go_out_as_blend_modes`). Drop
shadows use the same table (`drop-shadow.md`).

Tests: `render::tests::merge_modes_mix_with_the_backdrop_as_their_formulas_say`
(a pixel of each new mode against its formula),
`behind_draws_only_where_the_backdrop_is_clear`,
`pixel_merges_composite_alpha_like_blend_modes`;
`io::tests::transparency_merge_modes_round_trip_in_native_files`.
