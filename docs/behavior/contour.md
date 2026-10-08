# Contour

Concentric copies of an object's outline, inside, outside or to centre.

| Control | Default | Range |
|---|---|---|
| Direction | Outside | To centre, Inside, Outside |
| Steps | 1 | 1..999 (to centre: computed) |
| Offset | 1.0 mm | 0.001..300 mm |
| Outline colour, fill colour | object's | |
| Colour blend | direct | direct, clockwise, counter-clockwise (HSB) |
| Corner type | Mitered | Mitered, Round, Bevel |

Shortcut: Ctrl+F9 opens the Contour docker. Interactive: drag outward
from the object for outside, inward for inside; the drag distance sets
`steps * offset`.

## Formulas

Step `i` (1-based) is the Minkowski offset of the source path by
`sign * i * offset` (polygon offset with the chosen join); for "to
centre" steps continue until the offset path is empty. Colours step
linearly from the object colour to the end colour over `steps`.

## Checks

- Given a 50 mm square, outside, 3 steps, 2 mm, then step 3 is a
  62 mm square (mitered) and its fill is the end colour.
- Given inside, to centre, offset 5 on a 20 mm square, then 2 steps
  are produced (10 mm and 0 mm).
