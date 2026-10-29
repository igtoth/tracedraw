# Fit Objects to Path

Object > Fit Objects to Path opens the Fit Objects to Path panel. Select
the objects to place, then the path last (Shift+click). The objects go
along all the path's subpaths one after another (Treat subpaths as
contiguous) or along its first subpath only; a closed path is walked
all the way round, its closing side included. The code is in
`apps/tracedraw/src/fit_path.rs`.

The panel starts from the defaults below, with no preview, each time
other objects are selected or another drawing is active (#1136). Once a
value changes, dashed outlines on the canvas show where every object and
copy will go. Apply is one undo step ("Fit Objects to Path"). The panel
shows how many objects are selected (all but the path) and what kind of
object the path is.

## Settings and defaults

| Setting | Choices | Default |
|---|---|---|
| Keep originals (Objects) | the selected objects stay where they are and copies are placed | off |
| Group all objects (Objects) | the objects placed go into one group | off |
| Treat subpaths as contiguous (Path) | all subpaths one after another, or the first only | on |
| Spacing | Evenly along the path, Fixed distance (Distance) | Evenly, 10 mm |
| Measure between | Centers, Edges | Centers |
| Start offset | any length, negative included | 0 |
| End offset | any length (open paths only) | 0 |
| Place the row at (fixed distance) | Start, Middle, End | Start |
| Copies | Each object once, Number of objects (1 to 1000), Fill the path (fixed distance only) | Each object once, 10 |
| Placement | On the path, Above the path, Below the path, with a distance from the path | On the path, 0 |
| Reference | Center of rotation, Bounding box, Bounding box with outline | Center of rotation |
| Origin | nine points of the box, three by three (Bounding box references only) | the centre |
| Rotation | Rotate objects to follow the path, Fixed angle; Angle in degrees counterclockwise | Fixed angle, 0 |
| Ignore initial rotation | take the turn the objects have off them first | off |
| Modify rotation: Style | None, Uniform, Progressive, Jitter, Progressive jitter | None |
| Start angle, Spin angle, Revolutions, Range (±), Clockwise | degrees, degrees, a number, degrees, a switch; each enabled only for the styles that use it (below) | 0, 360, 1, 15, off |
| Order | As selected, Reversed, Left to right, Right to left, Largest first, Smallest first, Large and small in turn | As selected |
| Reverse direction | start from the path's last node | off |

## Formulas

Lengths are in millimetres along the path, measured on the path
flattened to 0.05 mm. With Reverse direction the path is walked from its
last node (with several subpaths: the last subpath first). With
several subpaths walked as one, the move from one to the next is not
counted, and the path counts as closed only when every subpath is.
`s0` is the start offset; on an open path of length `L`,
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

**The box and the reference point.** Each object's box is its bounds;
with Ignore initial rotation, the object is first turned back by the
angle of its transform (`atan2(b, a)` of its matrix) about its centre and
the box is that of the turned-back outline; Bounding box with outline
grows the box by half the outline's width. The reference point is the
point that goes on the path: the object's centre of rotation (the box's
centre, or where it was moved with the Pick tool) or, for the two box
references, the Origin point: column `u` and row `r` (0 to 2 from the
top left) give `(x0 + u w / 2, y1 - r h / 2)`. `v` is the reference point
less the box's centre.

**Modify rotation.** A turn `e[i]` is added to object `i` of `m` (degrees
counterclockwise, all negated with Clockwise): None `0`; Uniform the
Start angle; Progressive `Start + Spin angle * Revolutions * i / k` with
`k = m` on a closed path and `m - 1` on an open one; Jitter `Start +
Range * j(i)`, where `j(i)` is a number from -1 to 1 worked out from `i`
alone (SplitMix64), so the preview and Apply agree and the same object
turns the same way each time; Progressive jitter both. Start angle and
Clockwise are enabled for every style but None, Spin angle and
Revolutions for the two progressive ones, Range for the two jitter ones.

**Half lengths.** For an object whose box is `w` by `h` and which is
turned by `a` relative to the path's direction (`Angle + e[i]` when it
follows the path; `Angle + e[i] - phi`, with `phi` the path's direction,
when the angle is fixed), its half length along the path is
`hl = w/2 |cos a| + h/2 |sin a|`, across it `hc = w/2 |sin a| + h/2 |cos a|`,
and `v` turned by `a` is `v'`. Its length before its reference point
along the path is `b = hl + v'.x` and after it `f = hl - v'.x`. With a
fixed angle (or a reference point off the centre) these depend on where
the object lands, so the layout is worked out three times, each with the
directions found by the last.

**Places.** `d[i]` is the distance along the path of object `i`'s
reference point and `m` the number placed.

| Spacing | Between | `d[i]` |
|---|---|---|
| Evenly, open | centers | `s0 + (s1 - s0) i / (m - 1)` (one object: `(s0 + s1) / 2`) |
| Evenly, closed | centers | `s0 + L i / m` |
| Evenly, open | edges | `d[0] = s0 + b[0]`, then `d[i] = d[i-1] + f[i-1] + g + b[i]`, `g = (s1 - s0 - sum (b + f)) / (m - 1)` |
| Evenly, closed | edges | `d[0] = s0`, the same step, `g = (L - sum (b + f)) / m` |
| Fixed distance | centers | `base + distance i` |
| Fixed distance | edges | `d[0] = base + b[0]`, then the step with `g = distance` |

With a fixed distance the run goes from `lo = 0` to `hi` (the last
reference point, or the last far edge between edges) and `base` is `s0` (Start),
`(s0 + s1) / 2 - hi / 2` (Middle) or `s1 - hi` (End). On an open path a
place past an end stays at that end; on a closed path it goes round.

**Transform.** At `d[i]` the path has the point `p` and the unit
direction `t`; its left normal is `(-t.y, t.x)` (the page's y axis
points up, so this is above a path drawn from left to right). On the
path the reference point goes to `p`. Above keeps the reference point's
place along the path and moves the object along the normal so that its
centre is `hc` plus the distance from the path off the path (the
reference point goes to `p + (hc + distance + v'.y)` along the normal);
Below the other way. The object (turned back first with Ignore initial
rotation) turns about its reference point by `phi + Angle + e[i]`
(follow) or `Angle + e[i]` (fixed).

## Copies and groups

Copies are made with fresh ids in their original's layer, above the
objects already there, with the original's fill, outline and effects.
With Keep originals every object placed is a copy and the originals
stay where they were. After an Apply that made copies, the objects
placed and the copies are selected (the path is not), so the panel
starts again from its defaults. Without copies the selection stays.

With Group all objects the objects placed (copies, and originals not
already inside a group) are grouped in the same undo step, when there
are at least two; the group is selected.

## Preview

The preview is worked out again only when the settings, the selection or
the drawing change; the frames between draw the outlines kept. Above 200
objects each object previews as its bounding box. Selections of more than
200 objects show no preview. `frame_times_of_the_fit_to_path_panel` in
`pick_drag_tests.rs` measures a frame with the panel open.
