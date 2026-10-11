# Brush Strokes (I)

Strokes drawn freehand (or applied to a selected curve from the Artistic
Media panel) become filled objects in one of five modes. Defaults:
Calligraphic mode, width 3 mm, angle 45 degrees, smoothing 25,
pressure 0.5, sprayer spacing 8 mm.

## Common

- The freehand points are simplified with a tolerance of
  `0.1 + smoothing / 100 x 1.5` mm (smoothing 0..100) before the stroke
  is built, so a higher smoothing gives fewer wobbles.
- The result takes the current fill colour (black when none) and no
  outline; the Properties panel edits it like any curve.
- Brush Strokes panel > Apply to curve replaces the selected curves by
  strokes of the current mode along their path.

## Modes

- Calligraphic: the outline swept by a flat nib of `width` at `angle`
  (degrees from +x) along the path; the thickness varies with the
  direction of travel, thinnest when moving along the nib.
- Preset: a variable-width stroke with a profile: Taper both ends
  (default), Taper start, Taper end, Bulge, Wave, Flat; width is the
  maximum thickness.
- Brush: the preset profile with a jittered edge (deterministic noise)
  for a dry-brush look.
- Expression: pen-pressure simulation; the speed between samples stands
  in for pressure (slow strokes are wide), scaled by the pressure
  setting (0.5 = half the width at full speed).
- Sprayer: objects (circles, squares, stars, hearts, leaves) placed
  from the start every `spacing` mm along the path (so a 40 mm path at
  8 mm gets 6 objects, at 0, 8, ... 40), each sized between 70 % and
  100 % of `width` in a fixed pseudo-random sequence.

## Checks

- Given a straight 50 mm stroke in Preset mode (taper both ends), then
  the outline is thinner at both ends than at the middle (test
  `tapered_stroke_is_thin_at_the_ends`).
- Given spacing 8 mm on a 40 mm path, then the sprayer places 6 objects
  (0, 8, 16, 24, 32, 40 mm).
