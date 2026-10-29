# Mesh fill

A grid of nodes over an object, each node with a colour and an opacity;
the colours flow between the nodes and the object's outline clips them.
The grid lines between the nodes are straight for meshes made here, and
curves where a file gives the nodes handles (`MeshNode::handles`: towards
the next and the previous column, the next and the previous row); every
cell is then the Coons patch bounded by its four grid lines.

## Mesh Fill tool

- A click on an object gives it a mesh fill of the bar's rows and
  columns, spread over its bounds, every node in the colour its fill
  had (white when it had none); a click on an object that has a mesh
  selects it. A click on nothing clears the selection.
- A click on a node selects it; dragging a node moves it (one undo step
  for the drag).
- With a node selected, a click on a colour in the colour palette
  colours that node (the fill stays a mesh); with the Pick tool the same
  click fills the whole object as usual.
- Double-click inside the mesh puts a node where it was: a new column
  and a new row through that point, their nodes placed and coloured as
  the cell's patch is there (the point's place `u`, `v` in the cell
  found from its four corners, `a + (b - a) u + (d - a) v + (a - b - d +
  e) u v = p`); the new node is selected. Within 4 pixels of a grid line
  only the line across it is added, on a node nothing (#804; the line
  used to go halfway between the nearest nodes, away from the click).
- Delete with a node selected removes the grid lines through it that
  are not on the edge (its column, its row or both); a node on the edge
  keeps its lines and the status bar says so.
- The Shape tool drags and colours the nodes the same way.
- The grid is drawn over the selected object; the selection handles are
  not.

## Property bar

| Control | Default | Range |
|---|---|---|
| Rows, Columns | 2 x 2 (the selected mesh's when there is one) | 1 to 50 each |
| Smooth colour | off | |
| Transparency (the selected node) | 0 % | 0 to 100 % |
| Clear Mesh | | the selected objects lose their mesh fill (no fill) |

New meshes take the rows and columns, and a selected mesh changes to
them keeping its colours: the new nodes are spread evenly over the old
nodes' bounds and each takes the colour and opacity the old grid has
there (mixed from the four old nodes around it). The Properties panel's
fill page changes the grid the same way. The rows and columns are kept
in the preferences (`tools.mesh_rows`, `tools.mesh_cols`).

## Drawing

Each cell is the Coons patch of its four grid lines (cubic curves between
the nodes with their handles, straight thirds without): the point at
`(u, v)` is `(1 - v) top(u) + v bottom(u) + (1 - u) left(v) + u right(v)`
less the bilinear mix of the four corners. The patch is cut into a grid
of small triangles (`k x k` cells, `k` from the cell's size on screen:
one per 3 pixels of its longest side, 1 to 96), each corner given its
colour and opacity, and every pixel takes the mix of its triangle's
corners:

- Without Smooth colour, the mix of the cell's four nodes (bilinear in
  `u` and `v`).
- With Smooth colour, the Catmull-Rom mix of the 4 x 4 nodes around the
  cell (rows and columns past the edge repeat the edge), in RGB, so
  colours flow through the nodes without creases.

The picture is worked out in the screen's scale and turn without its
pan, its edge grown by one pixel, and kept (the renderer's procedural
fill cache, up to 256 pictures and 24 million pixels), so panning, a
selection change or another object dragged do not compute it again; it
is drawn through the object's path with bilinear sampling. A mesh of
more than 16 million pixels at the current zoom is worked out for the
part on the canvas only, and not kept.

Dragging a node moves its handles with it. A grid line added through a
curved cell cuts the curve at the new node (the neighbours' handles are
shortened to the cut, so the line keeps its shape) and gives the new
node handles across the line mixed from its neighbours'. Changing the
rows and columns from the property bar spreads a straight grid anew.

## Checks

- `tracedraw_core::style::tests::meshes_regrid_and_lose_lines_keeping_their_colours`
- `tracedraw_core::style::tests::mesh_handles_move_with_their_nodes_and_cut_at_new_lines`
- `tracedraw_render::tests::curved_mesh_cells_follow_their_handles`
- `tracedraw_core::style::tests::a_double_click_puts_the_new_mesh_node_where_it_was`
- `pick_drag_tests::mesh_nodes_land_where_they_are_clicked_and_dragged`
- `tracedraw_render::tests::mesh_fill_blends_corner_colours`
- `tracedraw_render::tests::smooth_mesh_colour_flows_through_the_nodes`
- `mesh_tool::tests::the_mesh_tool_builds_colours_and_clears_meshes`
- `ui::fill_tool_bars::tests::the_fill_tool_bars_draw`
