# Extrude

A 3D-looking body built from an object's outline.

| Control | Default | Range |
|---|---|---|
| Type | Back parallel | Small back, Small front, Big back, Big front, Back parallel, Front parallel |
| Depth | 20 | 1..99 (vanishing point types) |
| Vanishing point | page centre | any point, locked to object or page |
| Rotation | none | 3D rotation of the body |
| Lighting | off | up to 3 lights, intensity 0..100 |
| Colour | use object fill | object fill, solid, shade (from, to) |
| Bevel | off | depth, angle |

Shortcut: none by default; Effects > Extrude opens the panel.

## Formulas

- Parallel types: the back face is the front face translated by the
  extrusion vector `v` (drag vector). Side faces are quads between
  each pair of consecutive nodes on the front and back faces.
- Vanishing point types: back face = front face scaled about the
  vanishing point `V` by `s = 1 - depth/100` (small back), `1 + depth/100`
  (big back); front types swap which face is the original.
- Face order: sort side faces by the depth of their midpoint along `v`
  (painter's algorithm), draw back face, sides back to front, front face.
- Shading: face colour = fill colour scaled by `0.6 + 0.4 * max(0, n . l)`
  per light, `n` the face normal in the pseudo-3D frame, `l` the light
  direction; the "shade" colour mode interpolates from the from-colour
  (front) to the to-colour (back).

## Checks

- Given a 20 mm square, back parallel, vector (10, -10), then the body
  has 4 side quads and the back face is the square at (10, -10).
- Given a light from the top-left, then the top and left faces are
  lighter than the bottom and right faces.
