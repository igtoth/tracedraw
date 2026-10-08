# Mesh fill

A grid of Bezier patches over an object, each node with a colour.

| Control | Default | Range |
|---|---|---|
| Grid | 2 x 2 | 1..50 x 1..50 |
| Node colour | object fill | any colour, transparency per node 0..100 |
| Smooth colour | off | |

Shortcut: M (Mesh Fill tool). Double-click adds a grid line; Delete
removes the node's lines; dragging a palette colour onto a node colours
it; Shift+drag a node moves without curving.

## Formulas

The mesh is `(m+1) x (n+1)` nodes with Bezier handles on each edge;
each cell is a Coons patch (see envelope) rendered by subdividing into
`k x k` quads (`k = ceil(size_px / 4)`, min 4) with bilinear colour
interpolation of the four corner colours (Gouraud), or bicubic when
"smooth colour" is on. The mesh is clipped by the object's path.

## Checks

- Given a 2x2 mesh with corners red, green, blue, white, then the
  centre pixel is the average (128, 128, 128) within 8/255.
- Given a moved interior node, then the colour boundary moves with it.
