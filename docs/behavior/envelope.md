# Envelope

Deforms an object into a bounding frame with 8 or more nodes.

| Control | Default | Range |
|---|---|---|
| Mode | Putty | Straight line, Single arc, Double arc, Unconstrained, Putty |
| Mapping | Putty | Horizontal, Original, Putty, Vertical |
| Presets | none | circle, arch, heart... |
| Keep lines | off | |

Shortcut: Ctrl+F7.

## Formulas

The envelope is a cubic Coons patch defined by four Bezier edges
(8 nodes: 4 corners, 4 edge midpoints, each with handles). A point
`(u, v)` of the source bounding box in 0..1 maps to
`P(u, v) = (1-v) B(u) + v T(u) + (1-u) L(v) + u R(v)
 - [(1-u)(1-v) P00 + u(1-v) P10 + (1-u)v P01 + uv P11]`.
Every node and handle of the source path is mapped; with "keep lines"
straight segments stay straight (only endpoints are mapped).

## Checks

- Given an unchanged envelope, then the object is identical.
- Given the top edge midpoint moved up by 10 mm (single arc), then the
  top-centre node of a rectangle moves up by 10 mm and corners stay.
