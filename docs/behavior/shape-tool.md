# Shape tool

Shortcut F10.

| Action | Behaviour |
|---|---|
| Click node | select; Shift adds; marquee selects several |
| Drag node | move; Ctrl constrains |
| Drag segment | bend the curve (the two surrounding handles move) |
| Drag handle | smooth nodes move the opposite handle to stay collinear; symmetrical also keep the length |
| Double-click segment | add a node |
| Double-click node | delete |
| Delete | delete selected nodes |
| + / - | add / delete node |
| C, S, Y | cusp, smooth, symmetrical (property bar buttons) |
| Break (property bar) | split the subpath at the node |
| Join (property bar) | join two end nodes |
| Reduce nodes | RDP simplification with the tolerance slider |
| Elastic mode | moving one node moves the others proportionally |
| Drag a rectangle corner node | every corner grows or shrinks by the same amount, in the current corner style; a square corner has one node, a cut one a node at each end |
| Ctrl+drag a corner node, or click it first | that corner only (the chosen corner's node is filled) |
| Drag an ellipse node | pie when the pointer is inside the ellipse, arc outside; a whole ellipse's node starts at the top and the way it turns picks which end it becomes |
| Drag a polygon node | its mirrored nodes follow: a vertex moves every vertex (the points between them stay), a point between two vertices moves all of those (a star); the drag's distance from the centre counts, not its angle |

Rectangles, ellipses and polygons show these nodes and the property bar
shows their controls (corners, pie and arc, points) while the Shape tool
is active. Each drag is one undo step.

## Checks

- `kind_nodes::tests::dragging_a_corner_rounds_all_corners_or_one`
- `kind_nodes::tests::rounded_corners_grow_by_the_same_amount`
- `kind_nodes::tests::corners_drag_in_page_units_on_scaled_rectangles`
- `kind_nodes::tests::the_ellipse_node_opens_a_pie_inside_and_an_arc_outside`
- `kind_nodes::tests::polygon_nodes_move_together`
