# Curve tools

Freehand, 2-point line, Bezier, Pen, B-spline, Polyline, 3-point curve,
Shape recognition and Sketch. All produce open (or closed, when the end
meets the start) `Path` objects with the toolbox defaults.

## Freehand (F5)

- The pointer is sampled every 0.2 mm of movement; on release the points
  are simplified with a tolerance of `0.2 + s / 100 x 2.2` screen pixels
  (`s` the Freehand smoothing, 0 to 100, 50 by default: 1.3 pixels; at
  least 0.02 mm), so a zoomed-in drawing keeps more detail, and fitted
  with smooth cubic segments (`geometry::smooth_path`). The smoothing is
  on the property bar and in Options > Tools > Curve tools.
- A click, then another click: a straight line between them, previewed
  to the pointer after the first click; Ctrl keeps it to steps of the
  constrain angle (15 degrees by default); Esc drops the first click; a
  drag in between draws freehand instead.
- While dragging, the stroke shows as a black hairline on a white line
  twice as wide, so it shows over any colour, a black fill too (#672).

## Bezier and Pen

- Click-drag places a smooth node and pulls its handle out
  symmetrically (the handle follows the pointer while the button is
  down). C while pulling makes it a cusp: the incoming handle stays where
  it is and only the outgoing one follows. C acts while it is held (also
  when it went down before the press) and the node stays a cusp after C
  is let go; pressing it again in the same pull changes nothing more.
  While a curve is being drawn C never aligns the selection. Alt while
  pulling moves the node itself, its handles with it.
  A pull carried out of the drawing window (over a docker or a toolbar)
  goes on following the pointer and ends where the button goes up, and a
  closing drag from the first node closes the curve there; the preview
  then follows the pointer back on the drawing and the next click places
  a node (#1014;
  `pick_drag_tests::a_pen_pull_let_go_outside_the_canvas_ends_and_the_next_click_places_a_node`).
- A plain click places a node without a handle: the segment to it is
  the one the preview showed, curved by the handle of the node before
  (straight when that node has none) and arriving straight at the new
  node; the segment leaving it is straight unless a handle is pulled
  (`pick_drag_tests::a_pen_click_after_a_pulled_node_follows_its_handle`).
  Clicking the last node again (within the join distance) makes it a
  cusp without adding a node.
- Double-click or Enter finishes; with the Bezier tool Space finishes
  too and then, being the Pick tool's key, switches to Pick (checked by
  `pick_drag_tests::space_finishes_a_bezier_curve_then_picks`); Esc
  finishes the curve as it stands too, the segment to the pointer
  dropped and the tool kept (a lone first node is dropped;
  `pick_drag_tests::esc_finishes_the_curve_being_drawn_and_keeps_it`).
  While it is drawn, Ctrl+Z takes back
  the last press that changed it (a node placed, a handle pulled, a
  click on a node), one at a time until the curve is gone, and then the
  document's own history; Ctrl+Shift+Z puts the steps back one by one.
  A finished curve of its own keeps those steps in the document's
  history, closed or not, selected or not (#360): Ctrl+Z opens a closed
  curve first, then takes the anchors back one at a time until the
  curve is gone; redo builds it again the same way (each step "Curve
  Node"; the first one creates the curve with the fill and outline the
  finished curve takes). A curve that continues a selected curve, a
  B-spline and a curve drawn while a symmetry is edited stay one step
  (`pick_drag_tests::ctrl_z_takes_back_pen_nodes_one_by_one`,
  `pick_drag_tests::a_finished_pen_curve_undoes_anchor_by_anchor`).
  Undoing or redoing a step of a curve drawn with the Pen or Bezier
  tool (one of those anchor steps, the close, or a line that continued
  a curve) goes back to drawing it (#631): that tool is chosen again,
  the curve selected, and a line goes on from the node drawing stood
  at before that step (the last anchor left, the node a continued line
  started from, or the first node once the first step is undone),
  wherever the pointer is, with the handle that node had then, so the
  dashed preview curves as before. A click goes on drawing and joins
  the same curve, so the curve can be undone anchor by anchor and drawn
  on at once; a click on the curve's other end closes it again. A
  redone close ends the line, the curve closed and selected. Redo leaves
  a curve being drawn alone, and while nodes are drawn on from a curve
  taken up this way there is nothing to redo
  (`pick_drag_tests::undoing_pen_steps_goes_back_to_drawing_the_curve`,
  `pick_drag_tests::redo_keeps_the_curve_being_drawn`,
  `pick_drag_tests::undo_goes_back_to_the_end_drawing_went_on_from`).
  A press on the first node (within the join distance, 5 screen pixels
  by default, once the curve has two nodes with the Bezier and Pen
  tools, three with the others: two curved nodes close into a leaf or a
  lens, #356; with the Pen and Auto add/delete on, within the larger of
  the join and pick distances, so the first node is never deleted
  instead) closes the curve (`CurveInProgress::closes_at_first`): a
  click closes it with the segment the preview showed, leaving the last
  node along its handle (straight when it has none) and arriving at the
  first node with no handle (#359); a drag from the first node pulls the
  closing segment's handle at the first node, which follows the pointer
  (drag left, the handle lies left, #358), and the first node's own
  outgoing handle stays as it was, with or without C held; the constrain
  key keeps that handle to the constrain angle about the first node, as
  on every other node (#473,
  `pick_drag_tests::ctrl_keeps_the_closing_handle_to_the_constrain_angle`).
- With no curve being drawn and one curve selected, the Pen and Bezier
  tools edit it: a click on a segment (within the pick distance) adds a
  node there (the Pen only with Auto add/delete on), a press on a node
  that is not an end node drags it as the Shape tool does (end nodes
  continue the curve, see below); with the Pen and Auto add/delete on a
  click on such a node deletes it (one undo step, Delete Node; a curve
  keeps at least two nodes).
- Auto add/delete (Pen, on by default): also on the curve being drawn, a
  click on a segment (within the pick distance) adds a node there,
  splitting the segment so the curve keeps its shape, and a click on a
  node deletes it, its neighbours joined with their handles. The last
  node (a click makes it a cusp) and the first once the curve has three
  nodes (a click closes it) keep their meaning. A drag from a node or a
  segment places a node as usual. The Bezier tool has no Auto
  add/delete.
- The pointer: the cross hair with the tool's sign to start a new
  curve; with a node and a line going on from it over an end node of the
  selected curve (continue it); a ring over the first node (close); a
  plus over a segment where a click adds a node; a minus over a node a
  click deletes (Pen, Auto add/delete); a hollow node over a node of the
  selected curve otherwise, or over the last node of the curve being
  drawn (edit the node, or make it a cusp). The Pen tool shows a pen
  nib instead of the cross hair (#362): black on a white halo, leaning
  up and to the right, with a dot at its tip; the tip is the hot spot,
  where the node goes. The same signs show below the nib.
- While drawing (code: `pen_look.rs`), the segments placed so far are a
  hairline in the colour of the outline new objects get (black without
  one), as the curve will look, on a white line twice as wide so a black
  outline shows over a black fill (#672); nodes placed before are small hollow
  squares and the last node a filled blue one; the last node's handles
  are blue dashed lines ending in arrow tips (a cusp shows its incoming
  handle as it was frozen). While the first node is dragged to close,
  the closing segment shows dashed in blue with its handle.
- Preview mode (Pen, on by default): while the button is up a dashed blue
  segment (5 px dashes, 2 px gaps, over a white line as thin as the dashes
  that shows in the gaps, so it reads over any colour, #489; plain line
  segments, so the preview costs nothing noticeable in a frame:
  `frame_times_of_the_pen_preview`, #560) runs from the last node to the pointer (as the next node would
  take it, snapped and with Ctrl's steps), curved by the last node's
  outgoing handle, so the next segment shows before the click. The
  Bezier tool and the Pen with Preview mode off show nothing past the
  last node.
- Both options are on the Pen's property bar and in Options > Tools >
  Curve tools (kept in the preferences, `curves.pen_preview` and
  `curves.pen_auto_add_delete`).
- Snapping applies to every placed node and to a handle being pulled;
  the snap mark and its screen tip (Options > Snapping) show beside the
  pointer while nodes are placed and handles pulled, including "grid"
  and "guideline" when the pointer lands on a grid line or a guideline.
- Ctrl: the next node goes in steps of the constrain angle from the last
  one (keeping how far the pointer reaches along that direction), and a
  dragged handle in such steps around its node; the preview follows.
  The key is Ctrl at first; Options > Tools > Curve tools > Constrain
  angle key makes it Shift or Alt instead (`curves.constrain_key`), for
  every curve tool and the Freehand tool's two-click lines
  (`pick_drag_tests::the_constrain_key_can_be_shift_or_alt`).

## B-spline

- Each click (or the start of a drag) places a control point; there are
  no handles. The curve is the cubic B-spline of the control points: it
  starts at the first and ends at the last, tangent there to the control
  polygon, and stays inside the points' hull (quadratic with three
  points, a line with two). While drawing, the control polygon shows
  dashed with the curve over it, both following the pointer.
- Double-click, Enter or Esc finishes (Esc with one point drops it);
  clicking the first point (once there are three) closes it into the periodic spline,
  smooth all round. The result is a curve object (its nodes are the
  Bezier segments, one a knot span) that keeps its control points
  (`Shape::spline`): the Shape tool shows and moves them, with the
  dashed control polygon, instead of the nodes (see
  [shape-tool.md](shape-tool.md), "B-splines"). A B-spline that goes on
  from the end node of the selected curve joins that curve and keeps no
  control points.
- Checked by `geometry::tests::bsplines_start_and_end_on_their_ends_and_stay_inside`,
  `pick_drag_tests::bspline_clicks_place_control_points_of_a_smooth_curve`
  and `pick_drag_tests::a_bspline_keeps_its_control_points_for_the_shape_tool`.

## Polyline and 2-point line

- Polyline: clicks add straight segments (no smooth handles);
  double-click or Enter finishes. 2-point line: the second click
  finishes the segment at once. Ctrl keeps each segment to steps of the
  constrain angle.
- While drawing, the polyline, line or B-spline (as with the 3-point
  curve) shows as a black hairline on a white line twice as wide, the
  segment to the pointer included, and its node squares have a white
  edge, so they show over a black fill (#672).

## Continuing the selected curve

With one open curve selected (lying on a layer), a line drawn with
Freehand (dragged, or click and click), Bezier, Pen, B-spline, Polyline
or 2-point line that starts or ends within the join distance (Options >
Tools > Curve tools, 5 screen pixels by default) of one of the curve's
end nodes becomes part of that curve (code: `curve_join.rs`):

- it runs on from the end node itself (a line ending there is turned
  round first; a line from the curve's start turns the curve round);
- when its other end lies on the curve's other end, the curve closes;
- when its other end lies on an end of another of the curve's
  subpaths, the two subpaths become one;
- the curve keeps its look and stays selected; the change is one undo
  step (Set kind) and the curve's own transform is kept.

Otherwise the line is a curve of its own. Closed curves, several
selected objects and objects inside groups are never continued.

The Pen and Bezier tools also go on from an open curve that is not
selected: a first press on an end node of the topmost visible, unlocked
open curve there (the selected curve's ends come first) selects that
curve and starts the new line on the end node, as if drawing had never
stopped. The next segment leaves along the mirror of the handle the
curve arrives at that end with, so the dashed preview curves and the
join is smooth; after a straight segment it leaves straight. A drag
from the end node pulls a new handle instead (#630).

The pointer shows a node with a line going on from it over such an end
node, and a ring over the first node of a curve being drawn when a
click would close it.

## Property bar

The selection's position, size, scale, angle and mirror buttons; with a
selection also its outline width, line style and start and end
arrowheads, and Close Curve (enabled for an open curve); for Freehand
the smoothing (0 to 100); with nothing selected a reminder of how to
finish a curve.

## 3-point curve

- Drag the chord, then click the point the curve passes through; the
  curve is the quadratic through the three points (the click at t = 0.5),
  written as a cubic.
- Ctrl (the curve tools' constrain key) while dragging the chord keeps
  it to steps of the constrain angle from its start, so a bow drawn with
  Ctrl stands level or upright (#934,
  `pick_drag_tests::ctrl_keeps_the_three_point_base_to_the_constrain_angle`).
- The same key held at the third click (and while the pointer moves
  before it, in the preview) makes the curve a bow: the arc of the circle
  through the three points, as cubic pieces of at most a quarter turn,
  standing evenly on the chord wherever the point is; points in a line
  or nearly give the quadratic (#934,
  `pick_drag_tests::ctrl_at_the_third_click_draws_a_bow`).
- A chord starting on an end node of the selected open curve (within
  the join distance) continues it, and the curve ending on its other
  end node closes it, as the other curve tools do (`curve_join.rs`;
  `pick_drag_tests::a_three_point_curve_continues_and_closes_the_selected_curve`).

## Shape recognition (Shift+S)

A freehand stroke becomes the shape it looks like, as drawn: tilted
shapes stay tilted (`recognize.rs`). The stroke is closed when its ends
are within a fifth of its size (the diagonal of its box).

- Open: a straight line from end to end when no point is farther from
  that line than a share of its length; else a smoothed curve.
- Closed, the corners found by simplifying the stroke (from its point
  farthest from the middle) by a share of its size:
  - four corners all near right angles: a rectangle, its sides the means
    of opposite sides, turned as the first side runs (within 45 degrees;
    nearly level ones come out level);
  - otherwise, when the stroke's area is within a share of the ellipse
    of the same spread (its second moments, so any tilt): an ellipse,
    level when nearly level, never tilted when nearly round;
  - three corners: a triangle, and four other corners: a four-sided
    shape, through the corners as drawn;
  - anything else: a smoothed closed curve.
- The new object takes the default fill and outline (open curves no
  fill), is selected; one undo step.

Property bar: Shape recognition and Smoothing, each None, Lowest, Low,
Medium (first), High, Highest; kept in the preferences
(`tools.recognize`).

| Level | Corners (share of size) | Straight line | Right angles | Ellipse area | Smoothing (mm) |
|---|---|---|---|---|---|
| None | no shapes | no shapes | | | 0.05 |
| Lowest | 0.03 | 0.025 | 8 degrees | 5 % | 0.15 |
| Low | 0.045 | 0.04 | 12 degrees | 8 % | 0.3 |
| Medium | 0.06 | 0.06 | 16 degrees | 12 % | 0.6 |
| High | 0.08 | 0.08 | 20 degrees | 16 % | 1.0 |
| Highest | 0.11 | 0.11 | 25 degrees | 20 % | 1.6 |

## Sketch (S)

- The stroke is simplified by the bar's smoothing level (as Shape
  recognition's) and smoothed. With Join on, when an open curve is
  selected and the new stroke starts within the join distance (5 mm) of
  its end, the stroke is appended to it; otherwise a new curve is created
  and selected, so consecutive strokes chain.
- Property bar: Join to the selected curve, the distance, Smoothing; kept
  in the preferences (`tools.sketch`).

## Checks

- Given a rectangle with a black fill, while a freehand stroke is
  dragged, a polyline is drawn or a Bezier segment is placed over it,
  they are drawn on a white line
  (`pick_drag_tests::lines_being_drawn_show_on_a_black_fill`).
- `pick_drag_tests::a_pen_click_after_a_pulled_node_follows_its_handle`
- `pick_drag_tests::clicking_the_last_node_again_makes_it_a_cusp`
- `pick_drag_tests::dragging_from_the_first_node_shapes_only_the_closing_segment`
- `pick_drag_tests::the_pen_adds_a_node_on_a_segment_and_drags_a_node_of_the_selected_curve`
- `pick_drag_tests::the_pen_deletes_a_node_of_the_selected_curve_and_the_toggle_turns_it_off`
- `pick_drag_tests::the_pen_adds_and_deletes_nodes_on_the_curve_being_drawn`
- `pick_drag_tests::c_held_while_pulling_makes_a_cusp_that_stays_and_does_not_align`
- `pick_drag_tests::the_snap_tip_shows_while_placing_and_pulling_with_the_pen`
- `pen_look::tests` (preview geometry, colours and node marks, the
  closing preview, splitting and deleting nodes of the curve being
  drawn, the defaults)
- `pick_drag_tests::the_pen_continues_an_unselected_curve_with_the_mirrored_handle`
  (#630)
- `pick_drag_tests::undoing_pen_steps_goes_back_to_drawing_the_curve`
  (#631)
- `pick_drag_tests::redo_keeps_the_curve_being_drawn` (#631)
- `pick_drag_tests::undo_goes_back_to_the_end_drawing_went_on_from`
  (#631)
- `curve_join::tests` (from either end, closing, two subpaths made one,
  reversing curves, in the curve's own space with one undo step)
- `ui::cursors::tests::tools_get_their_cursors` (join and close signs)

- Given a freehand drag of 100 jittered points along a line, then the
  result has at most a handful of nodes and spans the same bounds.
- `recognize::tests::strokes_become_the_shapes_they_look_like` (level and
  tilted rectangles, a circle and a tilted ellipse, a triangle, a line,
  recognition off)
- `ui::drawing_bars::tests::both_bars_draw`
