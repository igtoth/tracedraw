# Convert Outline to Object

Object > Convert Outline to Object (Ctrl+Shift+Q) replaces the outline
of each selected object by a new closed object filled with the outline's
colour, the shape the outline draws; the original keeps its fill and
loses its outline. One undo step, and the new objects are selected.

- A hairline counts as 0.25 mm wide.
- A dashed outline keeps its dashes: each dash becomes a closed shape of
  its own and the gaps stay empty, the pattern in widths as drawn
  (#490).
- A variable or calligraphic outline, or one inside or outside the path,
  becomes its band, fitted to curves like any other outline. A Variable
  Length band is built from the path flattened to 0.005 mm for it: the
  eased width becomes a few curves on each side and the butt ends lines
  (#627).
- A closed path, an ellipse included, is stroked all round, with no
  ends at its start.
- The result is clean: the stroke is computed on short lines, then
  fitted back to lines and cubic curves within 0.01 mm, with corners
  kept as corners and as few nodes as that allows. An ellipse's outline
  becomes two rings of a few curves each, not hundreds of short lines
  (#622). Each smooth run between corners is fitted by least squares,
  refined by Newton steps and split in the middle until it fits, so the
  nodes fall evenly, and neighbouring pieces that fit as one curve are
  joined again. Vertices within the tolerance of the first of their run are
  merged first (the tiny steps polygon operations leave; a run never
  spans more than twice the tolerance, so the fine round ends of a thin
  outline keep their shape), and a stretch that is straight
  but leaves at an angle stays a curve, so curves stay smooth; a smooth closed ring is split at its leftmost and
  rightmost points.
- Round ends and round corners are real curves: each round end is two
  curves meeting at its tip, with smooth nodes where it meets the sides,
  and a round corner one or two curves between nodes where it meets
  them; straight sides stay single lines (#623).

## Checks

- Given a line with a dashed outline, the result is one closed shape
  per dash with the gaps empty
  (`tools2::tests::a_dashed_outline_becomes_one_shape_per_dash`).
- Given an ellipse with a 4 mm outline, the result is a 4 mm ring of 8
  to 16 curves and no lines, its outer size 64 x 44 mm
  (`tools2::tests::an_ellipse_outline_becomes_a_few_curves`).
- Given a straight line with butt ends, the result is a rectangle of
  four lines (`tools2::tests::a_straight_outline_becomes_a_rectangle`).
- Given a curved open path with a thick butt-ended outline (an arc 87 mm
  across with a 5.8 mm outline, as in #622), the result is four cusp
  nodes at the butt corners, one curve along each long side and
  straight ends
  (`tools2::tests::a_thick_butt_ended_arc_becomes_four_nodes_and_two_curves`).
- Given a line with a corner, round ends and round corners, the result
  is four straight sides, two curves per end and one for the corner,
  with nodes at the tips and where the ends meet the sides
  (`tools2::tests::round_ends_and_corners_become_few_curves`); on a
  curved path the ends also meet the sides at nodes and the whole
  outline stays within 16 segments
  (`tools2::tests::round_ends_of_a_curve_meet_its_sides_at_nodes`).
- Given a 0.25 mm line with a corner, round ends and a round corner,
  and a straight hairline with round ends, the results reach as far as
  the strokes and have their areas within 0.5 %
  (`tools2::tests::a_thin_round_outline_keeps_its_shape`).
- Given a line with a Variable Length node of 10 mm in the middle of a
  2 mm outline, the result is two curves per side and two straight
  ends; along an arch, two straight ends and at most 16 curves, with the
  area of the band drawn within 0.2 %
  (`tools2::tests::a_variable_outline_becomes_a_band_of_few_curves`).
- The fitting: given nodes where a stadium's ends meet its sides, it
  becomes two lines and four curves with nodes at the tips
  (`tracedraw_core::fit::tests::given_nodes_split_a_stadium_where_its_ends_meet_its_sides`);
  a flattened circle becomes four curves within 0.02 mm,
  a rectangle four lines, collinear points one line, an open arc one
  curve keeping its ends, and degenerate input does not panic
  (`tracedraw_core::fit::tests::a_flattened_circle_becomes_four_curves`,
  `tracedraw_core::fit::tests::a_rectangle_keeps_four_corners_and_lines`,
  `tracedraw_core::fit::tests::collinear_points_become_one_line`,
  `tracedraw_core::fit::tests::open_runs_keep_their_ends`,
  `tracedraw_core::fit::tests::nothing_breaks_on_degenerate_input`).
