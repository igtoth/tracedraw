# Drop shadow

The Drop Shadow tool puts a shadow behind an object; the shadow stays
live (it follows the object) until it is broken apart or cleared.

| Control | Default | Range |
|---|---|---|
| Presets | | Soft, Tight, Lifted, Glow, On the floor (right, left), On the wall behind |
| Kind | flat | flat, or from the bottom, top, left or right edge (perspective) |
| Offset (flat) | 2 mm, -2 mm | any |
| Angle (perspective) | 20 degrees from the bottom edge | any |
| Stretch (perspective) | 60 % | 0 to 1000 % of the object's extent across the edge |
| Fade (perspective) | 50 % | 0 to 100 % |
| Opacity | 50 % | 0 to 100 % |
| Feathering | 1.5 mm | 0 to 50 mm |
| Feather direction | middle | inside, middle, outside, average |
| Feather edges | linear | linear, squared, inverse squared, flat |
| Merge mode | Multiply | the merge modes of transparency |
| Colour | black | any |

Shadows in files saved before merge modes existed keep the Normal merge
they were drawn with; they load as flat shadows feathered in the middle
with linear edges, as before.

## Drawing a shadow

- A drag from the middle of an object makes a flat shadow: its
  silhouette moved by the drag (offset = release point minus press
  point).
- A drag that starts in the outer fifth of the object next to an edge
  (the top and bottom edges first, then left and right) makes a
  perspective shadow standing on that edge: the edge stays where it is
  and the opposite side of the object falls on the pointer.
- One drag is one undo step.
- A group or a clip frame takes a shadow of its own, drawn beneath
  everything it holds. Its silhouette is what it shows: each object of
  a group, and a clip frame's filled frame with its contents cut to the
  frame (an empty part of an unfilled frame, or contents outside the
  frame, cast none). The shadow is measured on the frame's box (the
  group's box). Break Drop Shadow Apart makes the same silhouette (#873).

## Perspective geometry

With the edge's midpoint `A` (start handle), its inward unit normal
`n`, the object's extent across it `L` (height for top and bottom,
width for left and right), and the end point
`E = A + stretch L (cos angle, sin angle)`, a point `p` of the
silhouette goes to `p + s (D - L n)` with `D = E - A` and
`s = (p - A) . n / L` (`Shadow::silhouette_transform`). Points on the
edge stay, the far side lands on `E`. The opacity falls linearly along
`A` to `E` by the fade: `1 - fade * clamp(t, 0, 1)` where `t` is the
pixel's projection on `A E`.

## Feathering

The silhouette, filled with full coverage, is blurred with three box
passes of radius `feathering` (approximating a Gaussian); Average uses
one box pass of 1.7 times the radius (an even, straight ramp). The
blurred coverage `b` becomes `t`: middle and average `t = b`, outside
`t = min(1, 2 b)` (solid up to the outline, fading beyond), inside
`t = max(0, 2 b - 1)` (fading in from the outline). The edges shape it:
linear `t`, squared `t^2`, inverse squared `1 - (1 - t)^2`, flat `1`
wherever `t > 0.01`. The final alpha is `t * opacity * fade`, in the
shadow colour, merged with what lies beneath by the merge mode
(`Shadow::feather_value`; the renderer's `shadow_layer`).

## Handles

With the Drop Shadow tool the selected object's shadow shows a white
square at the start (the object's centre, or the edge's midpoint), a
square in the shadow colour at the end, a dashed line between them and
a bar on the line at the opacity (start = 0 %, end = 100 %).

- Dragging the end moves the offset, or the angle and stretch.
- Dragging the bar sets the opacity (whole per cent).
- Dragging the start to another edge stands the shadow on that edge;
  to the middle makes it flat. The end stays where it was.
- A palette colour dropped on the end square becomes the shadow colour
  (one undo step).

## Break Drop Shadow Apart

Object > Break Drop Shadow Apart (Ctrl+K, which takes the place of
Break Apart while a selected object has a shadow), or Break apart on the
property bar: each selected top-level object's shadow becomes a bitmap
of its own at 150 dpi, placed just behind the object, named Shadow,
holding the shadow's colour, feathering, opacity and fade. A merge mode
other than Normal is kept as an opaque transparency with that merge
mode. The object loses its shadow. Everything happens in one undo step,
and the new bitmaps come out selected.

## Checks

Checked by `shadow::tests` in the core (perspective geometry for the
bottom and top edges, flat offset, feather ramps, old files and preset
round trips, degenerate boxes), the renderer's
`tests::shadow_darkens_pixels_beside_the_object`,
`tests::shadow_merge_mode_multiplies_with_what_is_beneath`,
`tests::perspective_shadow_falls_on_the_floor_and_fades`,
`tests::clip_frames_and_groups_cast_their_shadows`, and
`shadow_tool::tests` in the application (drags, handles, colour drop,
presets, Break Apart with one undo step, a clip frame's shadow).
