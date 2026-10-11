# DXF import and export

ASCII DXF, the CAD exchange format. Open and Import read it; Export
writes the AutoCAD 2000 (AC1015) dialect that every CAD reader accepts.

## Import

- Units from `$INSUNITS` (unitless files are millimetres; inches, feet,
  centimetres, metres and the rest scale accordingly). The page is the
  `$EXTMIN`/`$EXTMAX` extents, or the content bounds, plus a 5 mm margin;
  the lower-left corner moves to the origin.
- Layers: one TraceDraw layer per DXF layer, in order of first use; the
  LAYER table supplies the "by layer" colour (ACI index 1..255, true
  colour 420 when present). "By block" entities take the INSERT's colour.
- Entities: LINE, LWPOLYLINE and POLYLINE/VERTEX (bulges become arcs),
  CIRCLE, ARC, ELLIPSE (full or arc, rotated by the major axis), SPLINE
  (control points and knots evaluated with de Boor; fit points as a
  polyline when there are no control points), SOLID/3DFACE/TRACE, HATCH
  (solid fills; pattern hatches as a 30 % tint with an outline; polyline
  and edge boundaries with lines, arcs, ellipse arcs and splines), TEXT
  and MTEXT (height to points, rotation, alignment, formatting codes
  stripped, `%%d`, `%%p`, `%%c` symbols), POINT (0.5 mm dot), INSERT
  (block as a group with position, scale and rotation, nested up to 16).
- Outline width: lineweight 370 in 1/100 mm, or a polyline's constant
  width (43); otherwise a hairline.
- Not imported: DIMENSION, LEADER, MLINE, IMAGE, 3D solids and regions
  (each reported once in the warnings).

## Export

- One LWPOLYLINE per outlined subpath, curves flattened at 0.05 mm,
  constant width 43 and lineweight 370 when the outline is not a
  hairline; colour as the nearest ACI index (62) plus the true colour
  (420).
- One solid HATCH per filled object (non-solid fills use their first
  colour), external polyline boundaries, one per closed subpath.
- TEXT per line of a text object (height from the point size, rotation
  and alignment kept); bitmaps are not written; clip frames write their
  frame and contents unclipped; live effects are baked.
- A LAYER record per document layer (frozen when hidden, locked flag,
  plot flag); names are made DXF-safe.

## Checks

- A LINE on layer "Walls" (colour 1) comes back red on a layer named
  Walls; an LWPOLYLINE with bulge 1 between (10,0) and (10,10) reaches
  x = 15; TEXT height 3.5 gives a 9.92 pt text.
- A 1 x 1 inch block inserted at scale 2 in an inch file gives a 50.8 mm
  group.
- Export then import of a filled, outlined rectangle and a text gives a
  hatch, a polyline (red, 0.5 mm) and the text.
- Malformed pairs and unknown entities never panic.
