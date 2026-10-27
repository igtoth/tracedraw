# Distort

Push and Pull, Zipper and Twister as a live effect (`Effect::Distort`):
the object keeps its own outline and draws the distorted one, so the
distortion can be changed, moved or cleared at any time. The formulas
are in `tracedraw_core::distort`; the tool in `distort_tool.rs`, its bar
in `ui/distort_bar.rs`.

## Formulas

`c` is the distortion's centre (in the object's own space; the middle of
its outline's bounds when it has none). The distortions work on the
outline's own nodes and segments: every subpath stays a subpath, an open
curve stays open.

- Push and Pull, `amount` from -200 to 200: each node `p` goes to
  `c + (p - c) * (1 + s)` and each control point `q` of a segment to
  `c + (q - c) * (1 - s)`, with `s = amount / 100`. Positive amounts
  push the nodes out and bow the segments in (spikes with hollow
  sides); negative amounts pull the nodes in and bow the segments out
  (petals). Straight segments become curves with their control points
  at a third and two thirds; quadratic ones are raised to cubic.
- Zipper, `amplitude` (mm, 0 to 100) and `frequency` (0 to 100): each
  segment is cut into `2 * frequency` equal steps of its parameter; the
  inner points move off the segment along its normal, alternately
  `+amplitude` and `-amplitude`; the nodes stay. Random scales each
  tooth by a fixed pseudo-random share of its height (the same every
  time it is drawn); Local scales it by `1 - d / d_max`, `d` its
  distance from the centre and `d_max` the farthest node's; Smooth
  draws a smooth curve through the points instead of straight teeth.
- Twister, `angle` (degrees; whole turns times 360 plus degrees,
  clockwise negative): the outline is cut into steps of at most 1/64 of
  its larger side, every point `p` is turned about `c` by
  `angle * |p - c| / d_max` and a smooth curve goes through them, so the
  centre stays and the farthest point turns by the whole angle.
- A centre that is not a number or lies over 100 m from the outline is
  replaced by the middle; values out of range are brought in; a result
  that is not finite leaves the outline as it was.

## The tool

- Drag on an object: the distortion is centred where the drag started
  and follows the pointer, one undo step for the whole drag. Push and
  Pull: `amount = 100 * dx / r` (`dx` the drag to the right, `r` half
  the object's larger side; to the left pulls), rounded to whole
  percents. Zipper: `amplitude = |drag| / 4`, the bar's frequency and
  options. Twister: the angle the pointer turned about the start, whole
  turns counted.
- The centre of each selected distortion is drawn as a diamond; a drag
  that starts on it (within 6 pixels) moves the centre.
- A drag replaces the object's distortion (an object has at most one;
  it stays in its place among the object's other effects).
- A click selects the object under the pointer (or nothing).
- Groups and bitmaps are not distorted.

## Property bar

| Control | Default | Range |
|---|---|---|
| Push and Pull, Zipper, Twister | Push and Pull | with a distorted object selected, turn its distortion into that kind with the bar's values for it |
| Amplitude (Push and Pull) | 20 | -200 to 200 |
| Amplitude (Zipper) | 2 mm | 0 to 100 mm |
| Frequency | 6 | 0 to 100 teeth per segment |
| Random, Smooth, Local | off | toggles |
| Clockwise, Counterclockwise | Clockwise | |
| Rotations | 0 | 0 to 9 |
| Degrees | 90 | 0 to 359 |

- The bar shows the selected object's distortion (its kind and values)
  and edits the selected distortions; each change is one undo step (a
  run of edits of the same objects, typing on, is one). A value changed
  on the bar also distorts the selected objects that have no distortion
  yet, about their middles, so the change shows at once (#1030,
  `distort_tool::tests::a_bar_edit_previews_on_an_undistorted_selection`);
  a kind button alone only turns existing distortions into that kind.
- Center Distortion: the selected distortions about their objects'
  middles again. Clear Distortion removes them. Convert to Curves
  (Ctrl+Q) makes the distorted outline the object's own (the object
  becomes a curve, also when it already was one), and Break Apart breaks
  the distorted curve, or a distorted text into one curve per letter,
  each distorted as the whole text was.
- Apply (shown when a selected object has no distortion): the bar's
  distortion on those objects, about their middles.
- The values are kept in the preferences (`tools.distort`).

## Checks

- `distort::tests::push_and_pull_move_nodes_and_bow_the_sides`
- `distort::tests::zipper_teeth_on_every_segment`
- `distort::tests::twister_turns_the_far_points_by_the_angle`
- `distort::tests::older_files_and_odd_values_read`
- `distort_tool::tests::a_drag_distorts_about_where_it_started`
- `distort_tool::tests::the_bar_edits_the_selected_distortion`
- `distort_tool::tests::settings_round_trip_and_stay_in_range`
- `distort_tool::tests::a_bar_edit_previews_on_an_undistorted_selection`
- `ui::distort_bar::tests::the_bar_draws_for_every_kind_and_shows_the_selection`
- Given a 20 mm square about its middle and Push and Pull 50, then a
  corner node is 1.5 times as far from the middle and the middle of a
  side is nearer the middle than 10 mm.
- Given Twister 90 on a 20 mm square about its middle, then the corner
  (10, 10) is at (-10, 10).
