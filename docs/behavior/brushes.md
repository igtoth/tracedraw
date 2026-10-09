# Shape brushes: Smear, Twirl, Smooth, Attract, Repel, Smudge, Roughen

Drag over a selected curve (objects that are not curves are converted
first) to deform the nodes within the nib radius. Defaults: nib radius
10 mm (property bar); Roughen amount 2 mm.

## Common behaviour

- Only the selected objects are affected; the nib is a circle of
  `brush_radius` around the pointer in page space, mapped into each
  object's local space.
- The effect falls off linearly from full at the centre to zero at the
  radius (`w = 1 - d / radius`).
- Each pointer sample is one command; they collapse into a single undo
  step per drag.

## Tools

- Smear (W): moves points by the pointer's movement scaled by the
  falloff, dragging the outline along.
- Twirl: rotates points around the pointer by an angle proportional to
  the pointer's speed (0.05 rad per mm of movement) times the falloff.
- Smooth: within the nib, the path is simplified (0.3 mm) and re-fitted,
  removing wobbles.
- Attract and Repel: points move toward the pointer (Repel: away, held
  with Shift) by 0.3 of the pointer's movement times the falloff.
- Smudge (V): a smooth (0.2 mm) followed by a smear, so the outline is
  pulled without kinks.
- Roughen (E): points within the nib are displaced perpendicular to
  the outline by a deterministic jitter up to the amount.

## Checks

- Given a 20 mm square converted to curves, when smeared at its right
  edge by (5, 0) with radius 10, then the right edge moves right and the
  left edge stays.
- Given the same square, when twirled at its centre, then its area is
  unchanged within 1 % (rotation only).
