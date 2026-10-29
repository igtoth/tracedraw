# Vector pattern fill (and the placement of every pattern fill)

A fill made of vector objects repeated as a tile (a full-colour vector
pattern fill). The Placement section below covers the two-colour and
bitmap pattern fills too: the three kinds share it.

## Model

`Pattern::Vector { shapes, tile }`: the tile's objects in tile-local
coordinates (origin at the tile's bottom-left, Y up, millimetres) and the
tile size. Any object can be in the tile: groups, text, outlines, nested
fills and effects. The tile is drawn with the object's own renderer, so
what you see in the tile is what the pattern repeats.

## Creating one

Object > Create > Pattern Fill with a selection asks for the type,
Vector or Bitmap; Vector: the selected
objects become the tile (`Pattern::vector_from_shapes` translates them so
their joint bounds start at the origin and sets the tile to the bounds
size). The pattern becomes the default fill for new objects, like the
other Create commands; apply it to a selection from the Properties panel
(Fill > Pattern) or by drawing a new object.

## Properties panel

The fill type row has a button for each pattern kind: Pattern (the
two-colour tiles), Vector pattern and Bitmap pattern. Vector pattern
takes the first vector pattern the drawing holds (fills of its objects,
or the default fill a Create > Pattern Fill set); with none it opens
Object > Create > Pattern Fill. Bitmap pattern takes the first bitmap
pattern, else the first bitmap on a page as the tile, else opens Create >
Pattern Fill (in a browser, the picture import). Below the type row:

- Two-colour: the tile (8 built-in), front and back colour.
- Vector: object count and a picker over the drawing's vector patterns
  (each listed once, whatever its placement, as "Vector pattern n: k
  object(s), w x h mm"), plus "New from selection..." (Object > Create >
  Pattern Fill).
- Bitmap: the tile's pixel size and a picker over the drawing's bitmap
  patterns, the bitmaps on the pages ("Picture on the page (w x h px)",
  the tile as wide as the picture is on the page), "New from
  selection..." and "Import picture..." (a picture file read through
  `files`, its tile as wide as the picture at the resolution the file
  records, else 96 dpi; applied to the selection as one undo step).

Picking a pattern keeps the placement the fill had. Then the placement
fields every kind shares (below).

## Placement (all pattern kinds)

`PatternPlacement`, one per pattern of any kind (`Pattern::placement`),
all defaults meaning today's tiling: tiles the kind's own size, the first
tile's bottom-left corner on the page origin, upright, every tile the
same way up, anchored to the page. A fill with the default placement is
written to `.tdraw` without a `placement` key, so older files read and
untouched fills write as before.

| Field | Panel | Meaning |
|---|---|---|
| `size: Option<Size>` | Tile size W, H (mm) with a lock | The size of one tile on the page. The kind holds it where it can (a two-colour tile's edge, a bitmap tile's width, a vector tile's size with its content scaled along); a height the kind cannot hold goes to the placement. The lock (on by default, kept per panel) changes the other dimension along. Minimum 0.01 mm. |
| `offset_x`, `offset_y` (mm) | Offset X, Y | Where the first tile's bottom-left corner sits, from the page origin (or the object's origin with Transform with object). |
| `rotation` (degrees) | Rotation | The tiles turned around the offset point, counter-clockwise. |
| `skew` (degrees, -89..89) | Skew | The tiles' vertical sides tilted; positive leans the tops to the right. |
| `mirror_horizontal`, `mirror_vertical` | Mirror tiles (two buttons) | Every second column (row) of tiles flipped against its neighbour, so the pattern's edges meet seamlessly. |
| `shift_percent` (0..100), `shift: Row / Column` | Row or column offset (two pictures and a % field) | Each row (or column) moved along by that part of a tile against the one before it, as bricks are laid. |
| `transform_with_object` | Transform with object | The tiles follow the object's transformation (move, scale, turn with it) instead of staying anchored to the page. |

The placement is a transform from the tile to the page:

- The block that repeats is one tile, or two by two tiles when mirrored
  (`cell_tiles`), as a lattice (`cell`): the unit square to `cols x w`
  by `rows x h` mm, leaning along X by `rows x shift x w` for a row
  offset, along Y by `cols x shift x h` for a column offset, so each row
  (column) sits that much further along.
- `tile_copies` lists where the tile is drawn inside that block (with
  the flips of mirrored copies, and copies at the sides filling the
  leaning edges); `tile_uv` gives the tile coordinates under a point of
  the block for patterns sampled per pixel (two-colour).
- `place` = translate(offset) x rotate(rotation) x skew(tan(skew)), the
  block's millimetres to the page; `transform(tile)` = `place x cell`.
- The rasteriser draws the block once into a pixmap sized by its edges
  on screen (two-colour up to 1024 px, vector 2048, bitmap no sharper
  than the picture) and fills the object with one repeating shader whose
  transform is `screen x [object] x place x cell x pixels`. A bitmap
  pattern without mirrored or shifted tiles repeats its decoded picture
  directly; two-colour and bitmap blocks are cached. One pattern draw per
  fill, no per-pixel work beyond the shader's.

Edits in the panel go through `patch_selection_fill` with the fields
`TileSize`, `TileOffset`, `TileRotation`, `TileSkew`, `TileMirror`,
`TileShift` and `TransformWithObject` (`FillField::PATTERN`): one undo
step each, into each selected object's own pattern (a bitmap pattern
keeps its picture when its rotation changes). Values that differ across
the selection show as mixed (empty fields, both mirror buttons off), and
a value typed into a mixed field applies to every object.

`.cdr` files: the reader is unchanged. `docs/cdr-format.md` records the
tile offset, rcp offset and flag fields of pattern fills by size only,
not their units or meaning, and says the vector pattern's offsets,
angle and skew are not read; nothing is read into the placement until
those are confirmed on files we own.

## Rendering

The tile is rasterised at the current zoom into a pixmap of
`round(tile x zoom)` pixels (clamped to 1..2048) and repeated with the
same anchoring as the bitmap and two-colour patterns: tiles anchor at the
page origin (the offset from it), so they do not slide when the object
moves or the view pans, unless Transform with object is on. See
Placement above for mirrored, shifted, turned and skewed tiles.

## Export

- SVG: a `<pattern patternUnits="userSpaceOnUse">` whose tile is the
  block that repeats (two tiles wide or tall where mirrored), with the
  tile content as vector paths once per copy, and `patternTransform`
  carrying the placement (with the object's transform in front for
  Transform with object). Two-colour patterns put the block's picture
  (32 px per tile) in an `<image>`, bitmap patterns their picture once
  per copy.
- PDF: a tiling pattern (`/PatternType 1`, unit cell, `/XStep` and
  `/YStep` 1) whose content is the block's image at 150 dpi (with a soft
  mask for clear parts of a vector tile), the placement in its
  `/Matrix`; the object is filled with `/Pattern cs /Pn scn`.
- EPS: rasterised at the export resolution, clipped to the object.
- Native `.tdraw`: stored as is (`"pattern": "vector"`, the placement
  only when set).

## Checks

- Given a 10 mm tile with a red square in its lower-left quarter filling
  a 20 x 20 mm rectangle, then pixels at (2.5, 2.5) and (12.5, 12.5) mm
  are red and (7.5, 7.5) mm is the page colour.
- Given a 10 mm two-colour checker mirrored horizontally over 20 x 10
  mm, then the pixel at (12.5, 2.5) mm is the back colour (the second
  column flipped) where the unmirrored pattern has the front colour.
- Given a row offset of 50 % on that checker over 20 x 20 mm, then the
  second row's front cells sit at x 5..10 and 15..20 mm.
- Given a vector pattern with Transform with object on a rectangle
  moved 5 mm right through `Command::TransformShapes`, then the tiles
  move 5 mm right too; without it they stay on the page grid.
- Given an SVG export, then the file contains a `<pattern>` with the
  tile's `<path>` and no `<image>`, and a mirrored, offset and turned
  two-colour pattern a `patternTransform="matrix(...)"` with the
  placement.
- Given a PDF export, then a pattern fill is a `/PatternType 1` pattern
  whose `/Matrix` holds the tile size, offset and rotation (the object's
  translation too with Transform with object).
- Given a save and load of a document with a vector pattern, or with a
  placed two-colour pattern, then the document compares equal, and an
  unplaced fill writes no `placement`.
- Given two objects whose patterns differ in rotation, then the panel
  shows the rotation as mixed, and a rotation set goes into each
  object's own pattern as one undo step.
