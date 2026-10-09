# Distort

Live effect (`Effect::Distort`) on the object's outline. Three modes on
the property bar; the current mode's amount and, for Zipper, the
frequency; Apply writes the bar values to the selection.

## Modes and formulas

The outline is resampled to 160 points `p_i` along its length; `c` is
the bounds centre, `r_max` half the longer side of the bounds.

- Push and Pull: `p + (p - c) * amount / 100`. Positive amounts push
  the nodes away from the centre, negative pull them in. Dragging:
  `amount = 2 * dx` (mm of horizontal drag).
- Zipper: `p + n * amplitude * sin(2 pi * frequency * i / 160)` with `n`
  the outline normal at `p`. Dragging: `amplitude = |drag| / 4`,
  frequency from the bar (default 12).
- Twister: rotate `p` about `c` by `angle * |p - c| / r_max`, so the
  centre stays and the edge turns by the full angle. Dragging:
  `angle = 3 * dx` degrees.
- Apply button: Push and Pull uses the amount directly, Zipper uses
  `|amount| / 10` mm with the bar frequency, Twister uses
  `amount * 3.6` degrees (amount 100 = one full turn).
- Defaults: mode Push and Pull, amount 20, frequency 12.

## Interaction

- Drag from an object: the object is selected and the effect follows
  the pointer; each move replaces the previous distortion of the same
  kind (one `SetEffects` per move, undone as a run).
- Click selects without changing the effect.
- The result is smoothed through the 160 samples and closed; the
  original nodes are kept in the object, so clearing the effect restores
  the shape exactly.

## Checks

- Given a 20 mm square with Push and Pull 100, then its bounds are 40
  mm.
- Given Twister 360 on a square, then the centre point is unchanged and
  a corner has turned by a full circle (back onto itself) while the
  midpoints of the edges have turned 180 degrees.
