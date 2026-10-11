# Vector Mosaic

Effects > Vector Mosaic opens the Vector Mosaic panel, which turns the
selected objects (vector or bitmap, any number) into a vector mosaic of
tiles. Apply makes the mosaic as one group on
top of the selection, in one undo step ("Vector Mosaic"), and selects it.

## Settings

| Control | Default | Range | Meaning |
|---|---|---|---|
| Density | 10 | 1 to 100 | Tiles per inch along the rows and columns: the pitch is `25.4 / density` mm |
| Scale | 1 | 0.1 to 5 | Tile size factor: a tile is `pitch x scale` across |
| Screen angle | 0° | -90 to 90 | The rows turned counter-clockwise |
| Keep original | on | | Off deletes the selection after the mosaic is made |
| Limit colors | off, 8 | 2 to 256 | The tiles' colours reduced to at most that many (median cut) |
| Method | Uniform (white matte) | | How tiles follow the source, below |
| Merge adjacent | 1 | 1 to 36 | The most tiles of one colour combined into one |
| Weld adjacent overlap | off | | One curve per colour instead of one object per tile |
| Shape | Circle | Circle, Square, Custom | The tile; squares and custom tiles turn with the screen |

The preview under Shape shows the tile. For Custom, select a closed curve
and click Select: its outline, fitted to a unit square, becomes the tile
(the curve itself stays). Apply needs a selection, and for Custom a tile.

## How the mosaic is made

1. The selection is rendered at `6 x density` dots per inch (36 to 600)
   on transparency.
2. Tile centres lie on a square screen of the pitch, turned by the screen
   angle about the selection's centre; centres outside the selection's
   bounds are dropped.
3. Each tile averages a 3 x 3 grid of samples over its cell
   (premultiplied), giving a colour and an opacity `a`.
4. Method:
   - Uniform (white matte): every tile, the colour flattened on white
     (`c a + 255 (1 - a)`), full size.
   - Size modulation 1 (opacity): the size times `sqrt(a)`, so the area
     follows the opacity; the colour as sampled.
   - Size modulation 2 (luminosity): the flattened colour, the size times
     `sqrt(1 - L / 255)`, so darker areas give bigger tiles.
   A tile smaller than 5 % of the full size is left out.
5. Limit colors builds a median-cut palette of the tiles' colours and
   moves each tile to the nearest.
6. Merge adjacent: with a maximum of `n`, blocks of `k x k` neighbouring
   tiles of the same colour and size, `k` from `floor(sqrt(n))` (at most
   6) down to 2, become one tile `k` times as big at their centre.
7. Output: each tile an object filled with its colour and no outline
   (circles as ellipses, squares and custom tiles as curves); with Weld
   adjacent overlap, one curve per colour holding every tile of it.

A mosaic of more than 60 000 tiles is refused with a message in the
status bar (lower the density), so a dense screen cannot freeze the
editor.

## Not covered

The mosaic is made at once, not in the background. Welding joins the tiles of a colour into one
curve without removing the overlaps (they print as one area).

## Checks

- Given a red half and a transparent half, Uniform covers the bounds
  with tiles of 2.54 mm, white ones on the transparent part
  (`vector_mosaic::tests::uniform_tiles_cover_the_bounds_flattened_on_white`).
- Opacity leaves no tile on transparency; Luminosity makes the dark side's
  tiles bigger (`opacity_and_luminosity_size_the_tiles`).
- Limiting to 2 colours leaves at most 2; merging 4 makes fewer, bigger
  tiles; a turned screen stays inside the bounds; welding makes one curve
  per colour; a too dense screen is refused
  (`limits_merges_screens_and_welds`).
- Applying without keeping the original replaces it with a group in one
  undo step, and undo brings it back
  (`applying_groups_the_mosaic_in_one_step`).
