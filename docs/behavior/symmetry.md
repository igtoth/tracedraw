# Symmetry

Live mirrored copies of an object across one or more mirror lines
(Object > Symmetry).

## Model

`Effect::Symmetry { center, angle, lines }` on the object. `center` is a
page point, `angle` the direction of the first mirror line in degrees
(0 = horizontal, counter-clockwise positive), `lines` the number of
mirror lines (1 to 12), spaced evenly around `center`.

## Result

With `n` lines the copies are the dihedral orbit of the object: a
reflection across each line plus the rotations those reflections
generate, `2n - 1` copies in all. The copies are drawn above the original
(`evaluate()` puts them in `above`), share its fill, outline and other
effects, and follow every edit of the original, because they are
recomputed from it.

## Defaults

- Create (Object > Symmetry > Create New Symmetry, Alt+S, or the object
  context menu): one vertical line (90 degrees) through the right edge of
  the selection's bounds, at its vertical centre, so the copy appears next
  to the original instead of over it.
- Edit (Object > Symmetry > Edit Symmetry): mirror lines 1..12, angle
  0..180, centre x/y in page millimetres. Changes collapse into one undo
  step labelled "Symmetry".
- Remove Symmetry drops the effect; the copies disappear.
- Break apart (in the dialog) bakes the copies into real objects.

## Display

Mirror lines are drawn as dashed selection-colour lines across the whole
canvas while an object with the effect is selected.

## Checks

- Given a rectangle at x 10..20, when Symmetry is created with the
  defaults, then the evaluated `above` contains one copy spanning x 20..30.
- Given two lines at 0 degrees through the origin, then there are three
  copies, one per other quadrant (core test `symmetry_two_lines`).
- Given a symmetry object, when the original is moved, then the copies
  move with it (they are recomputed, nothing is stored).
