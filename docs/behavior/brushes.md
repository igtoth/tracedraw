# Shape brushes: Smooth, Smear, Twirl, Attract and Repel, Smudge, Roughen

Drag over a selected curve (objects that are not curves are converted
first) to reshape the outline within the nib. Only selected objects
change: with nothing selected a drag does nothing and the status bar
says "Select an object first, then drag over it with the brush"; a
click selects (see "Clicks with drawing tools" in `toolbox.md`;
`brushes::tests::brushes_change_only_the_selected_objects`). The whole
drag is one undo step; the next drag is another.

## Common behaviour

- The nib is a circle of the bar's Nib size (radius, 10 mm at first)
  around the pointer on the page. The brush works on the outline as it
  lies on the page (the object's transform applied) and the result is
  taken back into the object's own space, so the nib, the moves and the
  settings' millimetres are what is seen whatever the object's scale: a
  10 mm nib reshapes 10 mm of a curve scaled five times, not 50 mm.
- The effect falls off from full at the centre to nothing at the edge of
  the nib (`w = 1 - d / radius`).
- Only the stretch of a curve within the nib's reach changes: the pieces
  of segments within the nib and a quarter of its radius beyond (the
  points there do not move, so the new stretch joins the old curve
  smoothly) are measured into points (a sixth of the nib apart for Smear
  and Twirl, half a millimetre for Smooth, the spike spacing for
  Roughen), moved and fitted back into curves (Roughen: straight lines,
  so the spikes stay sharp), and the fitted curves are merged into as few
  as keep the stretch within 0.1 mm (`fit::fit_path`, whose splitting is bounded, so a long stroke never
  hangs, #1133; Smooth merges
  within its own tolerance), so a stroke leaves a handful of nodes, not
  one every few millimetres (`effects::brush_runs`, `fewer_curves`). A
  piece out of reach shorter than one and a half times the reach is
  rebuilt along with the piece in reach next to it, so a stroke that
  advances step by step does not leave a node, and a sliver of the old
  curve, at every step (`LEFTOVER_SHARE`). The pieces out of reach
  beyond that, and the subpaths out of reach, stay exactly as they were,
  nodes and handles included; open curves stay open, closed ones closed
  (a closed curve touched in part starts from a piece out of reach).
- Curves fitted through points (`geometry::smooth_path`, also the
  Freehand tool's) take the tangent at a point from its neighbours and
  handles a third of each segment long, so a long segment next to a
  short one neither overshoots nor flattens.

## Tools and their bars

Every bar starts with Nib size and ends with "Drag over a selected
object". The settings are kept in the preferences (`tools.brush_radius`,
`tools.roughen_amount`, `tools.brush`).

| Tool | Bar | What a step does |
|---|---|---|
| Smooth | Rate 1 to 100 % (20) | each point of the stretch in reach eased towards the average of the stretch around it (a quarter of the nib's radius each side, nearer points counting more) by three quarters of `w`, so jags and wobbles melt away a little with every step; then the points that matter within `0.05 + 1.45 (rate - 1) / 99` mm are kept (Ramer-Douglas-Peucker at half of it), a smooth curve fitted through them and merged into as few curves as stay within that tolerance |
| Smear | Pressure 1 to 100 % (100) | points move by the pointer's move times pressure times `w^2` |
| Twirl | Speed 1 to 100 % (50); Counterclockwise or Clockwise (Alt or Shift while dragging turns the other way) | points turn about the pointer by `speed / 1000` rad per mm the pointer moved, times `w`; held still, it goes on as if the pointer moved 10 mm a second |
| Attract and Repel | Speed 1 to 100 % (50); Attract or Repel (Alt or Shift while dragging swaps them) | nodes and control points move towards (away from) the pointer by `speed x 0.006` of the pointer's move times `w`, never past the middle; held still, it goes on as if the pointer moved 10 mm a second |
| Smudge | Pressure 1 to 100 % (100) | a smooth (0.2 mm), then a smear at the pressure, so the outline is pulled without kinks |
| Roughen | Frequency 1 to 10 spikes per 10 mm (5); Spike size (2 mm) | points `10 / frequency` mm apart within the nib pushed off the curve by up to the spike size times `w`, the push from the point's place |

## Checks

- `tracedraw_core::effects::tests::brushes_touch_only_the_subpaths_they_reach`
- `tracedraw_core::effects::tests::brushes_leave_the_segments_out_of_reach_exactly`
- `tracedraw_core::effects::tests::smooth_fits_jagged_lines_within_the_nib`
- `tracedraw_core::effects::tests::a_smooth_stroke_along_a_wobbly_spiral_leaves_few_nodes_on_it`
- `tracedraw_core::effects::tests::long_smudge_and_twirl_strokes_stay_light`
- `brushes::tests::a_stroke_is_one_step_and_follows_the_settings`
- `brushes::tests::attract_repel_and_twirl_follow_their_modes`
- `brushes::page_space_tests::the_nib_is_measured_on_the_page_whatever_the_object_scale`
- `brushes::page_space_tests::smooth_leaves_the_outline_outside_the_nib_alone`
- `ui::brush_bars::tests::brush_settings_scale_and_stay_in_range`
- `ui::brush_bars::tests::every_brush_bar_draws`
