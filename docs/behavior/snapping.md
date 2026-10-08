# Snapping and guides

| Setting | Default |
|---|---|
| Snap to objects | on, threshold 10 px (screen) |
| Snap to guides | on |
| Snap to grid | off; document grid 10 mm, 10 subdivisions |
| Snap to page | on |
| Dynamic guides | off (Alt+Shift+D); angles 0, 45, 90, 135 |
| Alignment guides | off (Alt+Shift+A) |
| Snap modes | node, intersection, midpoint, quadrant, tangent, perpendicular, edge, centre, text baseline |

Snapping is evaluated in screen pixels; the nearest candidate under the
threshold wins; priority order: node, intersection, midpoint, quadrant,
centre, edge, grid, page. Alt+Q toggles all snapping.

## Checks

- Given a guide at x = 50 mm and threshold 10 px at 100%, when an object
  is dragged to x = 51 mm, then it lands at 50 mm.
