# Shape tool

Shortcut F10. Double-clicking the tool's toolbox button selects every
node of the selected curves.

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
| Drag segment | bend the curve (the two surrounding handles move) |
| Drag handle | smooth nodes move the opposite handle to stay collinear; symmetrical also keep the length |
| Double-click segment | add a node |
| Double-click node | delete |
| Delete | delete selected nodes |
| + / - | add / delete node |
| C, S, Y | cusp, smooth, symmetrical (property bar buttons) |
| Break (property bar) | split the subpath at the node |
| Join (property bar) | join two end nodes into one |
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
| Drag a polygon node | its mirrored nodes follow: a vertex moves every vertex (the points between them stay), a point between two vertices moves all of those (a star); the drag's distance from the centre counts, not its angle |

Rectangles, ellipses and polygons show these nodes and the property bar
shows their controls (corners, pie and arc, points) while the Shape tool
is active. Each drag is one undo step. Other objects show a Convert to
Curves button.

## Property bar

Selection mode | Add Node, Delete Node | Join Nodes, Break Curve | Convert
to Line, Convert to Curve | Cusp, Smooth, Symmetrical | Reverse
Direction, Extend Curve to Close, Extract Subpath, Close Curve | Stretch
and Scale Nodes, Rotate and Skew Nodes, Align Nodes | Reflect
Horizontally, Reflect Vertically, Elastic Mode | Select All Nodes |
Reduce Nodes, Curve smoothness.

## Checks

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
