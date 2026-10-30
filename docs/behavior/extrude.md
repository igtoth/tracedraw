# Extrude

A body built from an object's outline, as a live effect
(`Effect::Extrude`): side faces between the outline and a copy of it,
and that copy as the far or the near face. The geometry is in
`tracedraw_core::extrude`, the tool in `extrude_tool.rs`, the bar and
the controls the Extrude panel shares in `ui/extrude_bar.rs`.

## Types

| Type | The copy | Where it is drawn |
|---|---|---|
| Small back | each point `p` to `p + (V - p) * d` | behind the object |
| Big back | `p + (p - V) * d` | behind the object |
| Small front | `p + (V - p) * d` | in front; the object behind |
| Big front | `p + (p - V) * d` | in front; the object behind |
| Back parallel | `p + v` | behind the object |
| Front parallel | `p + v` | in front; the object behind |

`V` is the vanishing point, `d` the depth over 100 (1 to 99), `v` the
offset of a parallel body. Older files without a type read as Back
parallel, or Small back when they have a vanishing point.

## Faces

- The outline is flattened (0.05 mm) subpath by subpath (at most 400
  points each); a subpath that ends where it started counts as closed.
  Every edge gives a quad to its copy.
- A face shows when its normal (pointing away from the filled side, so a
  hole's walls face into the hole) points along the way the body runs
  at that face (back types) or against it (front types). Open subpaths
  show both sides.
- Faces between corners (where the outline turns by more than 20
  degrees) make one surface, drawn as one object: the outline of its
  faces filled with their colour (a curved side, whose faces the light
  shades differently, with a linear fountain across it through their
  colours, at most a dozen stops) and outlined with the object's
  outline (at most 0.1 mm wide), or with a hairline of its own colour
  when the object has none. A square's body is its two sides, a
  circle's its one curved side: each side breaks apart and takes a
  colour or a fill as one part, not as a row of slats (#1281).
- Surfaces are drawn farthest first: along the offset for parallel
  bodies, by distance from the vanishing point for the others (nearer
  is farther for Small back and Big front, the other way for Big back
  and Small front).
- A back body draws its copy first (the far face, in the darkest
  colour), then the surfaces, then the object. Where the object does
  not cover the far face, a side does, so the far face is left out when
  the object is closed and opaque (a solid or opaque fountain fill, full
  opacity, no transparency or lens); it shows through an object without
  a fill and with open outlines (#1281); a front body draws the
  object, the surfaces, then the copy looking like the object (its fill
  and outline).

## Colour and light

- Object fill: the object's colour; Solid: one colour; Shade: a linear
  fountain on each face from the first colour at the object's side to
  the second at the copy's. The far face takes the object's or the
  solid colour half way to black, or the shade's second colour.
- Each face is darkened by the light (one light; angle 0 = from the
  right, counterclockwise; intensity `k` = 0 to 100 %):
  `0.55 + 0.45 * k * (n . l + 1) / 2 + 0.45 * (1 - k)`, at least 0.2.

## The tool

- A drag on an object (inside it counts, filled or not): a parallel
  body runs along the drag; the others take the point the drag ends on
  as the vanishing point. One undo step per drag.
- The selected extrusion's handle is a square with a cross, joined to
  the object's middle by a dashed line: the vanishing point, or the
  middle of a parallel body's copy. A drag that starts on it (within 6
  pixels) moves it.
- A click selects the object under the pointer (or nothing). Groups and
  bitmaps are not extruded.

## Property bar and panel

| Control | Default | Range |
|---|---|---|
| Type | Small back | the six types |
| Offset (parallel) | 6, 6 mm | document units |
| Depth | 20 | 1 to 99 |
| Vanishing point | up and to the right of the object (Apply), or where a drag ended | page coordinates, document units |
| Locked to | Locked to object | object (moves with it) or page |
| Colour | Object fill | Object fill, Solid, Shade (two colours) |
| Light angle | 135 degrees | 0 to 360 |
| Light intensity | 50 | 0 to 100 |

- The controls show the selected extrusion and edit the selected
  extrusions live (a run of changes is one undo step). Apply (shown when
  a selected object has none) adds the extrusion; Clear Extrude removes
  it; Break Apart turns the effects into objects, the faces grouped
  beside the object; Object > Break Extrude Group Apart (Ctrl+K, and
  the right-click menu) does the same (see `contour.md`).
- Effects > Extrude opens the panel. Five picture buttons at its top
  choose a page: Camera (a picture of the extrusion type, the type list,
  the vanishing point lock, Depth, X and Y, and Measure from: Page
  origin or Object center, which the vanishing point's X and Y count
  from; for parallel types X and Y are the offset and the vanishing
  point fields are greyed), Rotation (X, Y and Z turns, Reset),
  Lighting (angle, intensity, elevation and the further lights), Color
  (Object fill, Solid or Shade with their colours) and Bevel. Apply,
  Clear Extrude and Break Apart sit under every page. The values are
  kept in the preferences (`tools.extrude`).

## Turn, lights and bevel

The panel shows these under the other controls; the bar has them in its
3D menu. The geometry is in `tracedraw_core::extrude3d`.

| Control | Default | Range |
|---|---|---|
| Elevation (first light) | 0 | 0 to 90 degrees above the drawing |
| Turn x, y, z | 0, 0, 0 | -360 to 360 degrees; Reset puts them back |
| Light 2, Light 3 | off; 45 degrees, 30 up, 50 when switched on | angle, elevation, intensity |
| Bevel | 0 | a width in document units |

- The body is a prism: the outline at depth 0 and a copy at depth `L`,
  the offset's length (parallel bodies) or the depth share of the
  distance from the outline's middle to the vanishing point. A parallel
  body runs in space along its offset across and as far in depth, and is
  seen straight on, so untouched it is the flat body; the other types are
  seen from the vanishing point (Small: a perspective that puts the copy
  where the flat body has it, `s = t d / (t d + 1 - d)` of the way to the
  vanishing point at depth share `t`; Big: `d t` of the way away from it).
- The turn is about the middle of the body, by x (the top tips away),
  then y (the right side turns away), then z (counterclockwise). The
  object itself becomes the turned front face; turned away, the far face
  shows instead.
- A face shows when its outward side faces the viewer (its drawn corners
  run counterclockwise with that side up); faces are drawn farthest
  first.
- Light: the first light keeps the flat formula with its elevation,
  `0.55 + 0.45 (k (n . l + 1) / 2 + 1 - k)`; each further light adds
  `0.3 k max(0, n . l)`; the result stays within 0.2 to 1. A face's
  normal is worked out as if the body ran straight back, so an untouched
  body is lit as the flat one.
- Bevel: the front face shrinks by the width (each corner along its
  bisector, at most three widths) and a band slopes from it to the
  outline one width deep; at most 0.9 of the depth and of half the
  object's smaller side. The band takes the near colour; open subpaths
  have none.
- Untouched (no turn, the first light in the plane, no further light, no
  bevel), the body is the flat one above.

## Checks

- `extrude::tests::back_parallel_shows_the_faces_toward_the_vector`
- `extrude::tests::front_and_vanishing_bodies`
- `extrude::tests::every_subpath_extrudes_and_holes_show_their_far_walls`
- `extrude::tests::a_curved_side_is_one_outlined_surface`
- `extrude::tests::odd_values_give_no_faces`
- `extrude_tool::tests::a_drag_extrudes_toward_the_pointer`
- `extrude_tool::tests::the_bar_edits_the_selected_extrusion`
- `extrude_tool::tests::settings_read_older_effects_and_stay_in_range`
- `ui::extrude_bar::tests::the_bar_and_the_panel_draw_for_every_kind` (with the turn, lights and bevel, which go to the effect and back)
- `extrude3d::tests::untouched_with_a_raised_light_it_draws_as_the_flat_body`
- `extrude3d::tests::turning_shows_the_side_turned_toward_the_viewer`
- `extrude3d::tests::a_half_turn_hides_the_object_and_shows_the_far_cap`
- `extrude3d::tests::a_bevel_shrinks_the_near_cap_and_adds_a_band`
- `extrude3d::tests::further_lights_brighten_the_faces_they_reach`
- `extrude3d::tests::broken_values_never_panic`
- Given a 20 mm square, Back parallel, offset (10, 10), then the far
  face is the square at (10, 10) and only the top and right faces show,
  the top one lighter with the light from the top left.
- Given that square, Small back toward (60, 60) at depth 25, then the
  far face is 15 mm wide starting at (15, 15).
