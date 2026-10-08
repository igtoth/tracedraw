# Area Fill

Fills any enclosed area, even one formed by overlapping objects, by
creating a new object from the area under the click.

| Control | Default |
|---|---|
| Fill | use default (last used colour) |
| Outline | use default (hairline black) |

## Algorithm

1. Flatten every visible object on the page to polygons (0.05 mm
   tolerance), including open curves, which are treated as hairline
   polygons so they cut regions.
2. Build the planar arrangement of all edges (i_overlay "split" of all
   outlines unioned as lines).
3. Find the face containing the click point; its boundary (outer ring
   and holes) becomes a new closed path placed on top of the stack on
   the current layer, with the Area Fill colours.
4. If the click is outside every face (on the page background), do
   nothing.

## Checks

- Given two overlapping 20 mm circles 10 mm apart, when clicking in the
  lens-shaped overlap, then a new object with the overlap's area (within
  0.5%) is created on top.
- Given a click on empty page, then no command runs.
