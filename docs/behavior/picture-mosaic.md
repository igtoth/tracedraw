# Picture Mosaic

Effects > Picture Mosaic opens the Picture Mosaic panel, which recreates
the selected objects (vector or bitmap, any number) as a mosaic of
pictures from an image library. Apply makes
the mosaic on top of the selection in one undo step ("Picture Mosaic") and
selects it.

## Image library

Browse chooses a folder; its pictures (PNG, JPEG, BMP, GIF, WebP, TIFF,
not those in subfolders) are read in name order, at most 1000 of them,
and each gets the average colour of its centred square (flattened on
white). Reading happens on worker threads while the panel shows a
progress bar; files that cannot be read are skipped. The panel shows
the folder's name (the full path as a tooltip) and how many pictures it
holds. In the browser build there are no folders to read, so the panel
says libraries need the desktop application.

## Settings

| Control | Default | Range | Meaning |
|---|---|---|---|
| Keep original | on | | Off deletes the selection once the mosaic is made |
| Columns | 30 | 2 to 300 | Tiles across; Rows shows what the selection's shape gives |
| Blending | 20 % | 0 to 100 | How much of the selection shows over the tiles |
| Allow duplicates | on | | Off uses each picture once while unused ones remain |
| Tile spacing | 2 | 0 to 20 | With duplicates, the fewest tiles between two uses of one picture |
| Composition | Single | Single, Stack, Array | What the mosaic is made of, below |
| Edges | Stretch | Stretch, Remove | What happens to the last, partial row |
| Output priority | Document resolution | | How the pixel size is chosen, below |

Under the options the panel shows the mosaic's size in pixels for the
current selection. Apply needs a selection and a library.

## How the mosaic is made

1. The grid: with `c` columns over a selection `w` by `h` mm, a cell is
   `s = w / c` wide. Stretch gives `round(h / s)` rows (at least one) and
   stretches the cells to cover the selection exactly. Remove gives
   `floor(h / s)` rows of square cells and leaves out the strip along the
   bottom they do not cover. More than 1500 rows is refused with a
   message (use fewer columns).
2. The tile size in pixels, `tw` across:
   - Document resolution: `w / 25.4 x dpi / c` with the drawing's
     resolution.
   - Custom resolution: the same with the panel's resolution (10 to
     2400 dpi).
   - Custom tile dimensions: the tile width (4 to 2000 px).
   - Custom output dimensions: the mosaic width (16 to 15000 px) over the
     columns.
   The tile height is `tw` for Remove and `tw` times the cell's height
   over its width for Stretch. The mosaic stays within 15000 pixels a
   side and 64 million pixels in all; a bigger request is scaled down.
3. The selection is rendered at about 8 pixels a column (36 to 600 dpi)
   on transparency, and each cell gets its average colour (flattened on
   white).
4. Each cell, row by row, takes the library picture whose average colour
   is nearest, by `2 dr² + 4 dg² + 3 db²`, among the pictures the
   duplicate rule allows: with duplicates, none used within the tile
   spacing in either direction; without, none used before. When no
   picture is allowed (a small library) the nearest is used anyway.
5. Each picture's centred square is resized to the tile (once per
   picture) and the tiles are laid out on white.
6. Output:
   - Single: one bitmap with the selection blended over the tiles,
     `p (1 - a t) + q a t` for a tile pixel `p`, a selection pixel `q` of
     opacity `a` and the blending `t`.
   - Stack: a group of the mosaic bitmap and, with blending above 0, the
     rendered selection as a bitmap on top at opacity `t`.
   - Array: a group with one bitmap per tile (a picture used again shares
     its image) and the blend on top as for Stack.

## Limits

A library is one folder, and it is not remembered between sessions:
choose it again after restarting. Picture files are read again when the
mosaic is made, so a library on a slow drive makes Apply slower.

## Checks

- Rows follow the selection's shape; Remove keeps only whole rows and
  reports the part covered (`picture_mosaic::tests::grids_follow_the_reference_shape`).
- Cells take the nearest picture; with spacing 1 no picture sits next to
  itself; without duplicates each picture is used once
  (`cells_take_the_nearest_picture_and_respect_duplicates`).
- Tiles land in their cells and a half blend of black goes halfway
  (`mosaics_tile_and_blend`).
- A folder is read on worker threads in name order, skipping unreadable
  files and other kinds of file
  (`libraries_index_in_name_order_in_the_background`).
- Each output priority gives its pixel size, stretched cells keep the
  selection's shape, and huge or odd requests stay within the limits
  (`output_sizes_follow_the_priority_and_stay_bounded`).
- Single, Stack and Array each make one undo step with the expected
  objects; without a library or with too many rows nothing changes
  (`applying_makes_one_step_in_each_composition`).
