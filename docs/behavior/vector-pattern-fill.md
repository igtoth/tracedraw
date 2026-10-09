# Vector pattern fill

A fill made of vector objects repeated as a tile. the target design
calls it a full-colour (vector) pattern fill.

## Model

`Pattern::Vector { shapes, tile }`: the tile's objects in tile-local
coordinates (origin at the tile's bottom-left, Y up, millimetres) and the
tile size. Any object can be in the tile: groups, text, outlines, nested
fills and effects. The tile is drawn with the object's own renderer, so
what you see in the tile is what the pattern repeats.

## Creating one

Object > Create > Vector Pattern Fill with a selection: the selected
objects become the tile (`Pattern::vector_from_shapes` translates them so
their joint bounds start at the origin and sets the tile to the bounds
size). The pattern becomes the default fill for new objects, like the
other Create commands; apply it to a selection from the Properties docker
(Fill > Pattern) or by drawing a new object.

## Properties docker

Pattern > Vector: object count, tile width and tile height. Changing a
dimension scales the tile content with it, so the drawing keeps its
shape. Minimum tile: 0.01 mm.

## Rendering

The tile is rasterised at the current zoom into a pixmap of
`round(tile x zoom)` pixels (clamped to 1..2048) and repeated with the
same anchoring as the bitmap and two-colour patterns: tiles anchor at the
page origin, so they do not slide when the object moves.

## Export

- SVG: a `<pattern patternUnits="userSpaceOnUse">` with the tile content
  as vector paths.
- PDF and EPS: rasterised at the export resolution (150 dpi in EPS),
  clipped to the object.
- Native `.tdraw`: stored as is (`"pattern": "vector"`).

## Checks

- Given a 10 mm tile with a red square in its lower-left quarter filling
  a 20 x 20 mm rectangle, then pixels at (2.5, 2.5) and (12.5, 12.5) mm
  are red and (7.5, 7.5) mm is the page colour.
- Given an SVG export, then the file contains a `<pattern>` with the
  tile's `<path>` and no `<image>`.
- Given a save and load of a document with a vector pattern, then the
  document compares equal.
