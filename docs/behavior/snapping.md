# Snapping and guides

| Setting | Default |
|---|---|
| Snap to objects | on, threshold 10 px (screen) |
| Snap to guides | on |
| Snap to grid | off; document grid 10 mm both ways through the ruler origin (Document Options > Grid) |
| Snap to baseline grid | off; baselines from the page top (see `rulers-grid-guidelines.md`) |
| Snap to pixels | off; whole pixels of the document resolution from the page corner |
| Snap to page | on |
| Dynamic guides | off (Alt+Shift+D); angles 0, 45, 90, 135 |
| Alignment guides | off (Alt+Shift+A) |
| Snap modes | node, intersection, midpoint, quadrant, tangent, perpendicular, edge, centre, text baseline |

Snapping is evaluated in screen pixels; the nearest candidate under the
threshold wins; priority order: node, intersection, midpoint, quadrant,
centre, edge, grid, page. Alt+Q toggles all snapping.

A guideline being dragged (or dragged out of a ruler) and the ruler origin
snap to everything except guidelines.

## Checks

- Given a guide at x = 50 mm and threshold 10 px at 100%, when an object
  is dragged to x = 51 mm, then it lands at 50 mm.
