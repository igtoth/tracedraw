# Shape tool

Shortcut F10. Double-clicking the tool's toolbox button selects every
node of the selected curves.

A closed curve whose last segment ends back on its first node has that
node once: it shows both handles (the last segment's arriving one and the
first segment's leaving one), moves with the end lying on it, deletes by
joining the last and first segments, and Break opens the curve there.
Node counts in the status bar and the Objects panel count it once.

## How a curve shows

A curve selected with the Shape tool shows its outline in the node
colour (Options > Nodes and Handles, blue by default), its nodes as small
markers by type (squares for cusps by default) and the first node of each
subpath as a slightly larger triangle. A selected node's control
handles are dashed lines from the node, each ending in a solid arrow head
at its control point (the curve tools show the handles of the curve
being drawn the same way). The arrow showing the curve's
direction is off by default (Options > Nodes and Handles).

## Taking nodes and handles

A press takes the nearest node, or the nearest control handle of a
chosen node, within the pick distance of it: Options > Nodes and
Handles > Pick distance, 6 screen pixels by default (never less than
the node marker reaches). A segment of a selected curve is taken a
pixel further out. Distances are measured on the page, so a curve with
a scale of its own (text converted to curves, for one) is picked like
any other
(`pick_drag_tests::the_shape_tool_takes_the_nearest_node_of_a_scaled_curve`). A click that
takes nothing but lands within three pick distances of a node or a
handle of the selected curves (or a node of a selected rectangle,
ellipse or polygon) is a near miss: the selection stays. Only a click
further out on empty canvas deselects.

## Snapping

A dragged node snaps like a point placed by a drawing tool: to the
document grid, guidelines, the page and other objects (View > Snap To,
Options > Snapping), never to the curves being edited. When several
nodes move, the one under the pointer snaps and the move follows it;
Ctrl keeps the move horizontal or vertical before and after
snapping. A dragged control point, and a node of a rectangle, ellipse or
polygon, snaps the same way.

## The pointer

A solid black arrow (drawn by TraceDraw on a white halo, its tip on the
hot spot) with a mark at its lower right for what a press would take:
a small hollow square over a node (of a curve, or of a rectangle,
ellipse or polygon, or an envelope, perspective or mesh node), a dot
over a control point of a selected node, a wave over a segment of a
selected curve, nothing elsewhere. While a node or a control point is
dragged the mark stays. Over the handles framing the selected nodes
(Stretch and Scale or Rotate and Skew Nodes) the pointer is the Pick
tool's: resize arrows, or the turning arrow at the corners and the slant
arrows at the sides, also while they are dragged.

## Right click

A right click on a node chooses it (unless it is chosen already), on a
segment the node that segment ends at (so To Line and To Curve act on
that segment), on another object that object; then the node menu opens:
Add, Delete, Join, Break, To Line, To Curve, Cusp, Smooth, Symmetrical,
Reverse Direction, Extract Subpath, Close Curve, Elastic Mode,
Properties.

## Selecting nodes

| Action | Behaviour |
|---|---|
| Click node | selects it alone |
| Ctrl+click node | adds it to the node selection, or takes it out |
| Shift+click node | selects the run of nodes from the last selected one to it, along the subpath (the shorter way round on a closed one); Shift+clicking the run's end again selects the other way round; on another subpath it adds the node |
| Tab, Shift+Tab | the next or previous node of the curve, wrapping round |
| Marquee | Rectangular (default) or Freehand, picked in the property bar's Selection mode list; a freehand marquee selects the nodes inside the loop drawn (even-odd) |
| Ctrl+A, Select all nodes button | every node of the selected curves |

## Moving and editing

| Action | Behaviour |
|---|---|
| Drag node | move the selected nodes; Ctrl constrains |
| Click segment | chooses that segment: it is drawn over in the main colour with its handles, its end node is not shown as chosen (To Line and To Curve act on the segment through it), and the spot clicked gets a small round mark; Add Node (+ or the property bar button) puts the new node at that spot |
| Drag segment | bends it: the point grabbed follows the pointer exactly while both end nodes stay; a straight (or closing) segment becomes a curve first; both control points move, the nearer one more (by `k (1 - t) d` and `k t d` with `k = 1 / (3 t (1 - t) ((1 - t)² + t²))`, `t` the grabbed point's place along the segment, kept within 0.02 to 0.98, `d` the drag); a smooth or symmetrical end node stays so, its other handle turning along; one undo step (Bend Segment) |
| Drag handle | smooth nodes move the opposite handle to stay collinear; symmetrical also keep the length |
| Smooth, Symmetrical (property bar, node menu, C, S, Y) | a node between two curves gets its handles in line (symmetrical: of one length); between a straight segment and a curve, the curve's handle lines up with the straight segment, keeping its length, and the straight segment stays straight; between two straight segments both become curves (as To Curve makes them) and the node is smoothed, except a closing line that is only a close, which stays straight; an end node of an open curve stays as it is |
| Double-click segment | add a node |
| Double-click node | delete |
| Double-click a rectangle, ellipse or polygon | it becomes a curve; pictures, text, groups and clip frames stay as they are |
| Drag a picture's corner | the picture goes into a clip frame of its rectangle (Crop Bitmap) and the corner moves as a node of that frame; a clip frame's frame shows its nodes and edits like any curve, so a cropped picture can be reshaped further |
| Delete | with a segment chosen by a click: deletes that segment, its nodes staying (an open curve splits there or loses its end, a closed one opens there, a curve left with no segment is deleted; one undo step, Delete Segment); otherwise deletes the selected nodes; when a deleted node sat between segments and either side was a curve, one cubic is refitted from the segment before to the segment after (least squares with the old end tangents held, Graphics Gems), so the curve is kept rather than collapsing to the far handles; a node between two straight segments leaves a straight join |
| + / - | with nodes chosen: a node in the middle of each segment ending at them / delete them (without chosen nodes they zoom) |
| C, S, Y (with nodes chosen) | C turns cusp nodes smooth and the others cusp, S symmetrical nodes smooth and the others symmetrical, Y makes them symmetrical; while nodes are chosen these letters do not align objects or pick tools. The type chosen is kept with the curve (saved in `.tdraw`), so a cusp node may keep aligned handles and its handles still move apart; nodes never given a type read as their handles do (aligned: smooth; aligned and of equal length: symmetrical), and a curve that gains or loses nodes goes back to that |
| Home, End (with nodes chosen) | the first or last node of the curve |
| Arrow keys (with nodes chosen) | move the chosen nodes by the nudge distance (Shift: super nudge, Ctrl: micro nudge), one undo step per press |
| Break (property bar) | split the subpath at the node |
| Join (property bar) | join two selected end nodes into one node halfway between them, their handles moving along and no segment added: the two ends of one open subpath close it there, ends of two subpaths make one subpath, and ends on two curves make one curve (the first chosen keeps its place and look); one undo step (Join Nodes) |
| Drop an end node on another | dragging one end node and letting go within the pick distance of an end node of a selected curve joins the two as Join does; the curve dropped onto keeps its place and look |
| Extend Curve to Close | two selected end nodes are joined by a straight segment: the ends of one subpath close it, ends of two subpaths make one |
| Extract Subpath | the subpath of the selected node becomes its own curve |
| Reduce nodes | RDP simplification (0.3 screen pixels) of the curves with selected nodes, else of every selected curve |
| Curve smoothness (slider, 0 to 100) | drag it to remove nodes: the tolerance is (v / 100)² × 10% of the curve's larger side, worked from the curve as it was when the drag began, the selected nodes only when some are selected; one undo step per drag |
| Align Nodes | a dialog with Align horizontal and Align vertical: the selected nodes move onto a common line |
| Ctrl+C, Ctrl+X | with selected nodes, the segments between them are copied (cut: taken out of the curve, which opens) as new curves styled like their source |
| Ctrl+D | the same segments as new curves at the duplicate offset |

## Node transforms

The Stretch and Scale Nodes and Rotate and Skew Nodes buttons are
toggles. With one of them on and two or more nodes selected, eight
handles frame the selected nodes:

- Stretch and Scale: corner handles scale in proportion about the
  opposite corner, side handles stretch about the opposite side; Shift
  works about the frame's centre.
- Rotate and Skew: corner handles (curved arrows) rotate about the
  frame's centre by the angle the pointer turns; side handles (double
  arrows) skew along their side, the shear being the side's travel over
  half the frame.

Only the selected nodes and their handles move; each drag is one undo
step.

## Reflect and Elastic

- Reflect Nodes Horizontally and Vertically (toggles): moving a node
  moves the selected nodes on the other side of the selection's middle
  (vertical axis for horizontal reflection, horizontal axis for
  vertical) the mirrored way, so two sides of a shape widen or narrow
  together.
- Elastic mode (toggle): selected nodes move by
  `w = 1 - 0.5 × d / dmax` of the drag, `d` being a node's distance from
  the grabbed node and `dmax` the largest such distance; the grabbed node
  moves fully, the farthest one half as much.

## Rectangles, ellipses and polygons

| Action | Behaviour |
|---|---|
| Drag a rectangle corner node | every corner grows or shrinks by the same amount, in the current corner style; a square corner has one node, a cut one a node at each end |
| Ctrl+drag a corner node, or click it first | that corner only (the chosen corner's node is filled) |
| Drag an ellipse node | pie when the pointer is inside the ellipse, arc outside; a whole ellipse's node starts at the top and the way it turns picks which end it becomes |
| Drag a polygon node | its mirrored nodes follow: a vertex moves every vertex in or out (the points between them stay, and keep their turn); a point between two vertices moves all of those: in or out sharpens the star, around the centre turns them (a twisted star; within 1 degree of straight it stays untwisted; Ctrl keeps the turn they have) |

Rectangles, ellipses and polygons show these nodes and the property bar
shows their controls (corners, pie and arc, points) while the Shape tool
is active, after the object's kind ("Rectangle", "Ellipse", "Polygon";
a curve shows "Curve" before the node buttons). The status bar says so
too: "Rectangle: drag a corner node to round the corners (Ctrl+Q
converts to curves)", and the like for ellipses and polygons, where a
curve shows "Curve: n Nodes". The curve being edited is also drawn in
the node colour with a larger first node, which a rectangle, ellipse or
polygon never has. Each drag is one undo step. Other objects show a Convert to
Curves button.

## Text

With a text object selected the Shape tool shows a node per character
and the two spacing arrows; the property bar shows the chosen
characters' horizontal and vertical offsets and angle. See `text.md`,
Shape tool on text.

## Property bar

Selection mode | Add Node, Delete Node | Join Nodes, Break Curve | Convert
to Line, Convert to Curve | Cusp, Smooth, Symmetrical | Reverse
Direction, Extend Curve to Close, Extract Subpath, Close Curve | Stretch
and Scale Nodes, Rotate and Skew Nodes, Align Nodes | Reflect
Horizontally, Reflect Vertically, Elastic Mode | Select All Nodes |
Reduce Nodes, Curve smoothness.

## Checks

- `pick_drag_tests::nodes_are_taken_within_the_pick_distance_and_a_near_miss_keeps_the_selection`
- `pick_drag_tests::the_shape_tool_says_a_rectangle_is_still_a_rectangle`
- `pick_drag_tests::a_right_click_in_the_shape_tool_takes_the_node_or_segment_under_it`
- `pick_drag_tests::dragging_a_segment_bends_it_and_a_click_marks_where_a_node_goes`
- `nodes::tests::bending_a_segment_moves_its_point_and_keeps_the_ends` (core)
- `pick_drag_tests::c_s_and_y_set_the_type_of_the_chosen_nodes`
- `pick_drag_tests::the_pointer_shows_what_a_press_would_do` (marks, node
  transform handles)
- `node_edit::tests::ctrl_toggles_and_shift_selects_a_run`
- `node_edit::tests::handles_scale_rotate_and_skew_the_chosen_nodes`
- `node_edit::tests::reflect_and_elastic_moves`
- `node_edit::tests::segments_copy_cut_and_duplicate`
- `node_edit::tests::a_freehand_marquee_picks_the_nodes_inside`
- `node_edit::tests::extend_to_close_and_smoothness`
- `kind_nodes::tests::dragging_a_corner_rounds_all_corners_or_one`
- `kind_nodes::tests::rounded_corners_grow_by_the_same_amount`
- `kind_nodes::tests::corners_drag_in_page_units_on_scaled_rectangles`
- `kind_nodes::tests::the_ellipse_node_opens_a_pie_inside_and_an_arc_outside`
- `kind_nodes::tests::polygon_nodes_move_together`
- `kind_nodes::tests::points_between_the_vertices_twist_the_star`
- `geometry::tests::twisting_turns_the_points_between_the_vertices` (core): the
  vertices stay, the points between them keep their distance and turn by
  the twist; a plain polygon turned gains points at its sides' middles.
