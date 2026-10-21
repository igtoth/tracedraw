# Shape brushes: Smooth, Smear, Twirl, Attract and Repel, Smudge, Roughen

Drag over a selected object to reshape its outline within the nib. Only
selected objects change: with nothing selected a drag does nothing and
the status bar says "Select an object first, then drag over it with the
brush"; a click selects (see "Clicks with drawing tools" in
`toolbox.md`). An object that is not a curve becomes one when the nib
first reaches it; a selected group is reshaped through the objects
inside it, at any depth. The whole drag, conversions included, is one
undo step; the next drag is another.

## Common behaviour

- The nib is a circle of the bar's Nib size (radius, 10 mm at first)
  around the pointer, measured on the page, so it is the circle drawn on
  screen whatever the object's scale, stretch or turn.
- The effect falls off from full at the centre to nothing at the edge of
  the nib (`w = 1 - d / radius`).
- Only the outline within the nib changes. Each segment is tested by its
  real distance to the nib; the first and last segments of a run the nib
  reaches are cut where they cross the nib's edge (found on 64 samples
  and refined by halving). The run in between is measured into points on
  the page (a sixth of the nib apart for Smear and Twirl, a quarter for
  Smooth, an eighth of the current nib for Smudge, the spike spacing for
  Roughen), moved, and fitted back into as few curves as hold the points
  within 0.05 mm (Roughen: the moved points as straight lines, so the
  spikes stay sharp). Every other segment, node and corner, and every
  subpath the nib does not reach, stays exactly as it was; open curves
  stay open, closed ones closed.
- An object the nib reached earlier in the stroke keeps its change when
  the nib moves on to another one.

## Tools and their bars

Every bar starts with Nib size and ends with "Drag over a selected
object". The settings are kept in the preferences (`tools.brush_radius`,
`tools.roughen_amount`, `tools.brush`).

| Tool | Bar | What a step does |
|---|---|---|
| Smooth | Rate 1 to 100 % (20) | the run of points within the nib simplified within `0.05 + 1.45 (rate - 1) / 99` mm and re-fitted |
| Smear | Pressure 1 to 100 % (100) | points under the nib's last place move by the pointer's move times pressure times `w^2` |
| Twirl | Speed 1 to 100 % (50); Counterclockwise or Clockwise (Alt or Shift while dragging turns the other way) | points turn about the pointer by `speed / 1000` rad per mm the pointer moved, times `w`; held still, it goes on as if the pointer moved 10 mm a second |
| Attract and Repel | Speed 1 to 100 % (50); Attract or Repel (Alt or Shift while dragging swaps them) | nodes and control points move towards (away from) the pointer by `speed x 0.006` of the pointer's move times `w`, never past the middle; held still, it goes on as if the pointer moved 10 mm a second |
| Smudge | Pressure 1 to 100 % (100) | the outline within the nib around the pointer's last place moves by the pointer's move times `(1 - (d/r)^2)^2` (a soft, rounded falloff), so a drag that starts inside a closed shape and goes out pulls a rounded extension out of it and one that starts outside and goes in pushes a dent; the nib narrows along the stroke, `r = radius x max(0.6, 1 - pressure x travelled / (12 radius))` |
| Roughen | Frequency 1 to 10 spikes per 10 mm (5); Spike size (2 mm) | points `10 / frequency` mm apart within the nib pushed off the curve by up to the spike size times `w`, the push from the point's place |

## Checks

- `tracedraw_core::effects::tests::brushes_touch_only_the_subpaths_they_reach`
- `tracedraw_core::effects::tests::brushes_change_only_the_outline_under_the_nib`
- `tracedraw_core::effects::tests::the_nib_is_the_circle_on_the_page`
- `tracedraw_core::effects::tests::smudge_pulls_out_and_pushes_in`
- `brushes::tests::brushes_change_only_what_is_selected`
- `brushes::tests::one_undo_step_with_the_conversion_and_every_object_reached`
- `brushes::tests::a_brush_on_a_group_reshapes_its_objects`
- `brushes::tests::the_nib_matches_the_circle_on_screen_on_scaled_objects`
- `brushes::tests::smudge_pulls_extensions_out_and_dents_in`
- `brushes::tests::a_stroke_is_one_step_and_follows_the_settings`
- `brushes::tests::attract_repel_and_twirl_follow_their_modes`
- `ui::brush_bars::tests::brush_settings_scale_and_stay_in_range`
- `ui::brush_bars::tests::every_brush_bar_draws`
