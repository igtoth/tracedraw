# Procedural fill

A fill made of a pattern from a small library, each pattern drawn by
TraceDraw's own code from a few numbers: a tile of filled paths that
repeats over the object. The whole drawing is our own; no pattern code
or pattern definition comes from any other program.

## Model

`Fill::Procedural(Procedural { pattern, params, foreground, background,
seed })` in `crates/core/src/style.rs`:

- `pattern`: one of `ProceduralPattern::ALL` (below).
- `params`: `ProceduralParams`, the numbers every pattern reads the ones
  it uses from (`ProceduralPattern::params`), so switching patterns keeps
  them. Each has a serde default; a fill saved without them reads back
  with the defaults.
- `foreground`: the colour the tile's paths are filled with.
- `background`: the colour behind them covering the whole tile, or
  `None`: what is beneath the object shows through the pattern.
- `seed`: the start of the pseudo-random sequence of the patterns that
  scatter (the speckle); the same seed gives the same picture on screen,
  in exports and in print.

| Parameter | Field | Default | Range |
|---|---|---|---|
| Tile size (frequency) | `size_mm` | 10 mm | 0.5 to 200 mm |
| Line width | `line_width_mm` | 0.5 mm | 0.02 mm to half the tile |
| Cover (dots) | `share` | 0.3 | 0 to 0.78 |
| Amplitude (waves) | `amplitude` | 0.2 of the tile | 0 to 0.45 |
| Turns (spirals) | `turns` | 3 | 0.5 to 20 |
| Specks per tile (speckle) | `density` | 40 | 1 to 1000 |

Out-of-range numbers from a file are clamped (`ProceduralParams::clamped`),
never a panic.

## The patterns

`crates/core/src/procedural.rs` builds one tile per fill,
`procedural_tile(&Procedural) -> ProceduralTile { size, paths }`, kept in
a small cache keyed by the pattern, the numbers and the seed (the colours
are not part of the tile). Tile space is millimetres, origin at the
tile's bottom-left corner, Y up, as the page. Lines are stroked into
outlines (round joins) so every path is a filled region, filled with
the non-zero rule. With `s` the tile size and `w` the line width:

| Pattern | Tile | Drawing |
|---|---|---|
| Octagons and squares | `s` x `s` | One regular octagon per tile, side `s / (1 + sqrt 2)`, its flat sides on the tile's edges; the corners left between four octagons are the squares, so the octagon's eight edges draw the whole tiling. |
| Triangles | `s` x `s sqrt(3) / 2` | A horizontal line on the tile's bottom and top edges, the 60 degree family as the segments (0, 0) to (s/2, h) and (s/2, 0) to (s, h), the 120 degree family mirrored. |
| Bricks | `s` x `s` | Two rows of bricks `s` long and `s / 2` high, the upper row half a brick along; the bricks are the filled paths, inset `w / 2` on every side, so `w` of mortar between them shows the background. |
| Honeycomb | `s` x `s sqrt 3` | Regular hexagons `s` wide (flat to flat), point up, circumradius `R = s / sqrt 3`, rows `1.5 R` apart, every other row half a hexagon along; the tile holds two rows. |
| Crosshatch | `s` x `s` | The two diagonals of the tile, lines at 45 and 135 degrees `s / sqrt 2` apart. |
| Dots | `s` x `s` | One disc in the middle of the tile covering `share` of its area: `r = s sqrt(share / pi)`, at most `0.49 s`. |
| Waves | `s` x `s` | One period of `y = s / 2 + A sin(2 pi x / s)`, `A = amplitude * s`, as eight cubic segments from the exact tangents, stroked `w` wide. |
| Spirals | `s` x `s` | An evenly wound spiral `r = k theta` from the tile's centre out to `s / 2 - w` over `turns` turns, sixteen cubics a turn, stroked `w` wide with round ends. |
| Speckle | `s` x `s` | `density` discs of radius `w / 4` to `3 w / 4`, placed by a 64-bit mixing pseudo-random sequence from the seed, wholly inside the tile so the tiling has no seams. |

## Rendering

The rasteriser (`crates/render`) paints the tile once at the current
zoom into a pixmap of `round(tile x zoom)` pixels (1 to 2048 each way):
the background colour, then the paths in the foreground colour. The
pixmap is kept with the other procedural pictures (keyed by the fill and
the pixel size) and repeated by the pattern shader from the page origin,
so the tiles stay in place when the object moves and when the view pans.
There is no per-pixel work and nothing is rebuilt while the fill does not
change.

## Exports

- SVG: a `<pattern patternUnits="userSpaceOnUse">` of the tile's size
  holding a `<rect>` of the background (when there is one) and one
  `<path>` per tile path in the foreground colour; never a picture.
- PDF: a coloured tiling pattern (ISO 32000 section 8.7.3, PaintType 1,
  TilingType 1) whose cell is the tile in points with the same rectangle
  and paths, placed from the page's origin; the object is filled with
  `/Pattern cs /Pn scn`. One pattern object per fill on a page, however
  many objects use it. Spot and registration colours go in as their
  process stand-ins.
- EPS and the other formats without a counterpart (`.cdr`, DXF):
  rasterised as the pattern and texture fills are, or written as the
  foreground colour.
- Native `.tdraw`: stored as is (`"fill": "procedural"`).

## Properties panel

Fill > Procedural (the seventh kind picture, an octagon):

- the pattern list by name;
- a preview of four tiles, the renderer's picture of the tile paths,
  kept until the fill changes;
- the pattern's own numbers as sliders: tile size (mm, logarithmic),
  line width (mm), cover %, amplitude %, turns, specks per tile, and the
  seed with Regenerate (seed + 1);
- the Foreground colour and a Background tick with its colour.

Every edit goes through `App::patch_selection_fill` as the fill fields
of `FillField::PROCEDURAL` (pattern, size, line width, share, amplitude,
turns, density, seed, foreground, background): one undo step each, a
slider drag one step, and only the field that changed goes into each
selected object's own procedural fill, so objects with different sizes
keep them when one takes a new line width. The Edit Fill dialog and the
Interactive Fill tool's bar offer the same kind and editor. Fill winding
is not part of this panel.

## Checks

- Given each of the nine patterns at 8 mm, then the tile has paths,
  is 8 mm wide and 8 mm high but for the triangles (`8 sqrt(3) / 2`)
  and the honeycomb (`8 sqrt 3`).
- Given a speckle with seed 7, when it is built twice, or with another
  foreground, then the tiles are equal; with seed 8 they differ.
- Given a crosshatch, when the line width goes from 0.5 to 1.5 mm, then
  the paths' area more than doubles; given dots at a share of 0.5, then
  their area is 50 square mm of the 100.
- Given bricks with 1 mm of mortar on a white background rendered at
  4 px/mm, then the row at y = 5 mm is white, the row at 7.5 mm black at
  x = 2.5 mm, white at the joint at 5 mm, and the second tile repeats it.
- Given dots covering half the tile, then between 48 and 52 % of the
  pixels are dark.
- Given an SVG export of a honeycomb fill, then the file holds a
  `<pattern>` 6 mm wide and 10.3923 mm high with a `<rect>` and seven
  `<path>` elements and no `<image>`.
- Given a PDF export of a bricks fill, then the file holds one
  `/PatternType 1` object stepping the tile's size and the object is
  filled with `/Pattern cs /P0 scn`; two objects with the fill share it.
- Given a save and load of a drawing with a procedural fill of every
  number set and no background, then the drawing compares equal.
- Given two objects with procedural fills of different tile sizes, when
  the line width is changed in the Properties panel, then both take the
  width in one undo step and keep their sizes.
