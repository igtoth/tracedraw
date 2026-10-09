# Curve tools

Freehand, 2-point line, Bezier, Pen, B-spline, Polyline, 3-point curve,
Shape recognition and Sketch. All produce open (or closed, when the end
meets the start) `Path` objects with the toolbox defaults.

## Freehand (F5)

- The pointer is sampled every 0.2 mm of movement; on release the points
  are simplified with a tolerance of `max(0.15, 1.2 / zoom)` mm (so a
  zoomed-in drawing keeps more detail) and fitted with smooth cubic
  segments (`geometry::smooth_path`).

## Bezier, Pen and B-spline

- Click places a cusp node; click-drag places a smooth node and pulls
  its handle out symmetrically (the handle follows the pointer while the
  button is down).
- Double-click or Enter finishes; Esc discards the curve being drawn;
  clicking the first node (within 5 screen pixels, once the curve has
  three nodes) closes the curve.
- Snapping applies to every placed node.

## Polyline and 2-point line

- Polyline: clicks add straight segments (no smooth handles);
  double-click or Enter finishes. 2-point line: the second click
  finishes the segment at once.

## 3-point curve

- Drag the chord, then click the point the curve passes through; the
  curve is the quadratic through the three points (the click at t = 0.5),
  written as a cubic.

## Shape recognition (Shift+S)

- The stroke is simplified at 1 mm. It is "closed" when its ends are
  closer than a quarter of its longest side. Recognised results:
  - open stroke with at most 3 simplified points: a straight line;
  - closed with 3 or 5 points: a triangle; 4 (or 5) points: a rectangle,
    in both cases the stroke's bounding box;
  - closed with more points whose area is within 20 % of the bounding
    ellipse's: an ellipse;
  - anything else: a smoothed curve (closed when the stroke was).

## Sketch (S)

- The stroke is simplified at 0.8 mm and smoothed. When an open curve is
  selected and the new stroke starts within 5 mm of its end, the stroke
  is appended to it; otherwise a new curve is created and selected, so
  consecutive strokes chain.

## Checks

- Given a freehand drag of 100 jittered points along a line, then the
  result has at most a handful of nodes and spans the same bounds.
- Given a shape-recognition stroke tracing a square, then the result is a
  `Rect`; tracing a circle gives an `Ellipse`; a short open stroke gives
  a two-node line.
