# EMF and WMF import and export

Windows metafiles: the enhanced format (EMF) and the 16-bit format
(WMF, with or without the placeable header). Open and Import read both;
Export writes both. The layouts follow the public Windows metafile
specifications; the reader is a GDI record player
(`crates/io/src/emf.rs`).

## Import

- Page: EMF uses the header frame (0.01 mm units); device pixels map to
  it through the header bounds. A placeable WMF uses its bounding box
  and units per inch; a bare WMF takes the first window extent at 96
  dpi.
- Coordinates: logical to device through the map mode (MM_TEXT, the
  fixed metric modes, isotropic and anisotropic window/viewport) and,
  in EMF, the world transform (set, modify: left multiply, right
  multiply, identity); then to page millimetres with y up.
- Objects: pens (CREATEPEN, EXTCREATEPEN with caps, joins, user dash
  styles; PS_NULL and null brushes draw nothing; width 0 is a hairline),
  solid brushes (hatched and pattern brushes draw as their colour),
  fonts (LOGFONT height, weight 600+ as bold, italic, underline,
  strikeout, escapement), stock objects; handles reuse the lowest free
  slot in WMF as GDI does. Dash styles map to the outline presets.
- Geometry: MOVETO, LINETO, POLYLINE, POLYGON, POLYBEZIER, the TO and
  16-bit variants, POLYPOLYLINE, POLYPOLYGON (even-odd or winding from
  the polyfill mode), RECTANGLE, ROUNDRECT, ELLIPSE, ARC, ARCTO, CHORD,
  PIE, and GDI paths (BEGINPATH, CLOSEFIGURE, ENDPATH, FILLPATH,
  STROKEPATH, STROKEANDFILLPATH, ABORTPATH). Polygons are filled with
  the brush and outlined with the pen like GDI does; lines are outlined
  only.
- Clipping: SELECTCLIPPATH, INTERSECTCLIPRECT and EXTSELECTCLIPRGN
  (rectangle regions) set a clip in page space; objects drawn under a
  clip they do not fit in become clip frames with the clip as frame (text
  that is 60 % inside is left unclipped). SAVEDC and RESTOREDC keep the
  whole device context including the clip.
- Text: EXTTEXTOUTA/W and WMF TEXTOUT/EXTTEXTOUT become artistic text in
  the font's face, size (`|height|` for a negative height, 0.85 of a
  positive cell height), bold, italic and the text colour; alignment
  from SETTEXTALIGN (left, centre, right; top, bottom or baseline
  reference) and rotation from the escapement.
- Bitmaps: STRETCHDIBITS, STRETCHBLT, BITBLT, ALPHABLEND and the WMF
  STRETCHDIB, DIBSTRETCHBLT and DIBBITBLT records. DIBs of 1, 4 and 8
  bits (palette), 16 bits (555 or bit fields), 24 and 32 bits (BI_RGB
  or bit fields, alpha when any pixel has it) and embedded JPEG or PNG
  payloads; mirrored destinations flip the image. GRADIENTFILL
  triangles and rectangles are drawn flat in their average colour.
- Skipped with a warning: PolyDraw, unknown record types. GDI comments
  (EMF+ payloads included) are ignored; dual files still carry the GDI
  records that are read.
- Limits: 2,000,000 records, 2,000,000 points, 500,000 objects, DIBs up
  to 20,000 px a side and 50 Mpx, palettes up to 256 entries; truncated
  records stop the reader with a warning. Mutation testing finds no
  panic.

## EMF export

- One logical unit is 0.01 mm (MM_TEXT on a 2540 dpi device); the header
  frame is the page in 0.01 mm and the bounds the same area in pixels.
  Background mode transparent, graphics mode advanced.
- Every object is a GDI path: MOVETOEX, LINETO, POLYBEZIERTO (quadratic
  segments elevated to cubic), CLOSEFIGURE; FILLPATH with a solid brush
  (polyfill mode from the object's fill rule) and STROKEPATH with a
  geometric pen (EXTCREATEPEN: width, caps, joins, user-style dashes in
  logical units); hairlines are cosmetic pens of width 0. Arrowheads are
  filled paths.
- Gradient fills: the path becomes the clip, then 64 bands: linear
  fills as rotated quadrilaterals along the gradient axis, radial,
  conical and square fills as concentric ellipses from the outside in.
  Pattern, texture and mesh fills are their preview colour.
- Text: EXTCREATEFONTINDIRECTW (em height as a negative LOGFONT height,
  weight 700 for bold, italic, underline, strikeout, escapement and
  orientation from the object's rotation), SETTEXTCOLOR, SETTEXTALIGN
  with the baseline flag, one EXTTEXTOUTW per line at 1.2 em spacing.
- Bitmaps: a 32-bit bottom-up DIB (straight alpha) through
  STRETCHDIBITS under a world transform (SETWORLDTRANSFORM between
  SAVEDC and RESTOREDC), so rotated and skewed images keep their
  placement.
- clip frames: SAVEDC, the frame path as SELECTCLIPPATH (RGN_COPY), the
  contents, RESTOREDC, then the frame's outline. Groups, tables and
  symbol instances are expanded; live effects are evaluated first.

## WMF export

- Placeable header at 1440 units per inch (twips), so coordinates stay
  within 16 bits up to about 578 mm; MM_TEXT with the page as window
  extent.
- Curves are flattened (0.05 mm). Fills are POLYPOLYGON records with a
  solid brush and a null pen (even-odd or winding from the fill rule);
  outlines are POLYLINE records with a pen (width in twips, 0 for
  hairlines, dash presets mapped to the GDI dash styles). Gradient,
  pattern, texture and mesh fills become their preview colour (WMF has
  no gradients).
- Object handles take the lowest free slot, as GDI assigns them, so the
  SELECTOBJECT indices match what readers compute.
- Text: CREATEFONTINDIRECT (8-bit face name; characters outside Latin-1
  become `?`), SETTEXTCOLOR, SETTEXTALIGN with the baseline flag and one
  EXTTEXTOUT per line.
- Bitmaps: a 32-bit DIB through STRETCHDIB into the page bounds of the
  image (no world transforms in WMF, so rotated images fill their
  bounding box).
- clip frames are drawn unclipped: contents, then the frame outline.

## Checks

- Given an EMF with a 20 px pen, a blue brush, a rectangle at device
  (100, 100) to (300, 200) on a 1000 x 500 px, 100 x 50 mm device, then
  the import is a 20 x 10 mm rectangle at (10, 30) with a 2 mm red
  outline and a blue fill.
- Given a 2 x 2 24-bit DIB, then the pixels come back in order (top row
  first).
- Given a path selected as clip and a rectangle crossing it, then the
  rectangle is inside a clip frame whose frame is the clip.
- Given a placeable WMF at 1440 units per inch with a half-inch square,
  then the page is 25.4 x 12.7 mm and the square 12.7 x 6.35 mm.
- Given a rectangle, text and a linear gradient exported to EMF, then
  the reader gets them back with the same size, colours and position,
  and the gradient as at least 32 bands.
- Given a rectangle, text and a 2 x 1 bitmap exported to WMF, then the
  reader gets a filled polygon, an outlined polyline, the text at its
  baseline and the bitmap with its pixels, all within 0.05 mm.
