# Align and Distribute

The Align and Distribute panel (no key of its own since Ctrl+Shift+A
became Copy Properties From, #1345; Options > Shortcuts can give it
one), the Object > Align and
Distribute menu and the keys L, R, T, B, E, C and P.

## Align

Six picture buttons: left edges, centres on a vertical line (the same
x), right edges; top edges, centres on a horizontal line (the same y),
bottom edges. Under them, what they align to:

| Align to | Target |
|---|---|
| Active object (default) | the last object selected; with one object selected, the page |
| Page edge | the page's edges (left edges to its left edge, centres to its middle, ...) |
| Page center | the page's centre point (left edges at its x, top edges at its y, ...) |
| Grid | each object on its own: the edge or centre goes to the nearest document grid line, counted from the ruler origin every grid spacing |

- Use Outline (off): the boxes aligned are the objects' bounds grown by
  half their outline width, so the outer edges of the outlines meet. A
  group counts its bounds only.
- The menu and the keys always align to the active object; P (Center to
  Page) always takes the page's centre.
- Each Align is one undo step ("Align"); objects already in place are
  left out of it.

## Distribute

Eight picture buttons: left edges, centres, gaps and right edges along
x; top edges, centres, gaps and bottom edges along y. Where the objects
are spread:

| Distribute to | Needs | Ends |
|---|---|---|
| Selection extent (default) | 3 objects | the first and last objects (ordered by the edge or centre distributed; by centre for gaps) stay; the others move so the edges or centres are evenly spaced, or the gaps are equal between the first's low edge and the last's high edge |
| Page extent | 2 objects | the first object's low edge (left, bottom) touches the page's, the last object's high edge (right, top) touches the page's; edges and centres in between are evenly spaced, or the gaps are equal across the page |

- With fewer objects the buttons are pale and a line says how many are
  needed.
- Use Outline applies here too.
- The menu distributes between the outermost objects.
- Each Distribute is one undo step ("Distribute").

## Checks

- `ui::align_docker::tests::align_follows_the_target_the_docker_shows`
- `ui::align_docker::tests::use_outline_aligns_the_outer_edges_of_the_outlines`
- `ui::align_docker::tests::distribute_spreads_edges_centres_and_gaps`
- `ui::align_docker::tests::the_docker_draws_with_and_without_a_selection`
