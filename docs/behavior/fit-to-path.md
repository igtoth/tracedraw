# Fit Objects to Path

Object > Fit Objects to Path opens the Fit Objects to Path panel. Select
the objects to place, then the path last (Shift+click). The objects go
along the path's first subpath; a closed path is walked all the way
round, its closing side included. The code is in
`apps/tracedraw/src/fit_path.rs`.

The panel starts from the defaults below, with no preview, each time
other objects are selected or another drawing is active (#1136). Once a
value changes, dashed outlines on the canvas show where every object and
copy will go. Apply is one undo step ("Fit Objects to Path").

## Settings and defaults

| Setting | Choices | Default |
|---|---|---|
| Spacing | Evenly along the path, Fixed distance (Distance) | Evenly, 10 mm |
| Measure between | Centers, Edges | Centers |
| Start offset | any length, negative included | 0 |
| End offset | any length (open paths only) | 0 |
| Place the row at (fixed distance) | Start, Middle, End | Start |
| Copies | Each object once, Number of objects (1 to 1000), Fill the path (fixed distance only) | Each object once, 10 |
| Placement | On the path, Above the path, Below the path, with a distance from the path | On the path, 0 |
| Rotation | Rotate objects to follow the path, Fixed angle; Angle in degrees counterclockwise | Fixed angle, 0 |
| Order | As selected, Reversed, Left to right, Right to left, Largest first, Smallest first, Large and small in turn | As selected |
| Reverse direction | start from the path's last node | off |

## Formulas

Lengths are in millimetres along the path, measured on the path
flattened to 0.05 mm. With Reverse direction the path is walked from its
last node. `s0` is the start offset; on an open path of length `L`,
`s1 = L - end offset`; on a closed path `s1 = s0 + L` and the end offset
is not used.

**Order.** The objects are sorted first: Left to right and Right to left
by the x of their centres, Largest and Smallest first by the area of
their bounds; Large and small in turn takes the largest, the smallest,
the next largest, the next smallest and so on.

**Count.** `n` objects are selected. Each object once places `n`. A
number `N` places `max(N, n)`: the selected objects, then copies of them
in the same order (object `i mod n` at place `i`). Fill the path places
as many as fit between `s0` and `s1` (round a closed path): `floor(room
/ distance) + 1` between centres on an open path, `floor(L / distance)`
on a closed one; between edges, objects are added while the next one,
with its gap, still fits (on a closed path the gap back to the first
counts too). Never fewer than `n` and never more than 1000 in all.

**Half lengths.** For an object whose bounds are `w` by `h` and which
is turned by `a` relative to the path's direction (the Angle when it
follows the path; the Angle minus the path's direction `phi` when the
angle is fixed), its half length along the path is
`w/2 |cos a| + h/2 |sin a|` and across it `w/2 |sin a| + h/2 |cos a|`.
With a fixed angle these depend on where the object lands, so the layout
is worked out three times, each with the directions found by the last.

**Places.** `d[i]` is the distance along the path of object `i`'s
centre, `al[i]` its half length along the path, `m` the number placed.

| Spacing | Between | `d[i]` |
|---|---|---|
| Evenly, open | centers | `s0 + (s1 - s0) i / (m - 1)` (one object: `(s0 + s1) / 2`) |
| Evenly, closed | centers | `s0 + L i / m` |
| Evenly, open | edges | `d[0] = s0 + al[0]`, then `d[i] = d[i-1] + al[i-1] + g + al[i]`, `g = (s1 - s0 - sum 2 al) / (m - 1)` |
| Evenly, closed | edges | `d[0] = s0`, the same step, `g = (L - sum 2 al) / m` |
| Fixed distance | centers | `base + distance i` |
| Fixed distance | edges | `d[0] = base + al[0]`, then the step with `g = distance` |

With a fixed distance the run goes from `lo = 0` to `hi` (the last
centre, or the last far edge between edges) and `base` is `s0` (Start),
`(s0 + s1) / 2 - hi / 2` (Middle) or `s1 - hi` (End). On an open path a
place past an end stays at that end; on a closed path it goes round.

**Transform.** At `d[i]` the path has the point `p` and the unit
direction `t`; its left normal is `(-t.y, t.x)` (the page's y axis
points up, so this is above a path drawn from left to right). Above moves
the object along the normal by its half length across the path plus the
distance from the path; Below moves it the other way. The object turns
about its centre by `phi + Angle` (follow) or by the Angle (fixed), and
its centre goes to the moved point.

## Copies

Copies are made with fresh ids in their original's layer, above the
objects already there, with the original's fill, outline and effects.
After an Apply that made copies, the objects placed and the copies are
selected (the path is not), so the panel starts again from its defaults.
Without copies the selection stays.

## Preview

The preview is worked out again only when the settings, the selection or
the drawing change; the frames between draw the outlines kept. Above 200
objects each object previews as its bounding box. Selections of more than
200 objects show no preview. `frame_times_of_the_fit_to_path_panel` in
`pick_drag_tests.rs` measures a frame with the panel open.
