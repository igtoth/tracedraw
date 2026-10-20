# Transformations panel

Five pages, chosen with the picture buttons at the top of the panel:
Position, Rotate, Scale and Mirror, Size and Skew. Alt+F7, Alt+F8,
Alt+F9 and Alt+F10 open the panel on Position, Rotate, Scale and Mirror
and Size (pressed again on the page shown, they close the docker
column); Object > Transformations opens any of the five.

Each page is laid out the same way: a line under the
picture buttons, the page's name with a colon (Rotate shows a turning
arrow before its angle instead), the fields as spin boxes (up and down
arrows at their right, a click steps by a whole millimetre, pixel,
degree or percent, or a tenth of an inch) with the unit after them,
some room, the check box, the 3 x 3 reference point under it (for a
relative position also the direction picture, a small grid with four
arrows, beside it), a line, Copies in a spin box and Apply across the
panel. Distances are in the ruler unit
(through the drawing scale), angles in degrees (counter-clockwise
positive), scales in percent.

| Page | Fields (defaults) | What Apply does, `b` the selection's bounds, `a` its reference point |
|---|---|---|
| Position | X, Y (0, 0); Relative position (on) | relative: moves by (X, Y); otherwise moves `a` to (X, Y) counted from the ruler origin |
| Rotate | Angle (0); Center X, Y (0, 0); Relative center (on) | turns by Angle about `c + (X, Y)`, or about (X, Y) from the ruler origin when Relative center is off; `c` is the centre of rotation moved on the canvas (Pick tool, rotate mode) when there is one, else `a` |
| Scale and Mirror | X, Y (100 %); Mirror horizontally, Mirror vertically (off); Proportional (on) | scales by X/100 and Y/100 about `a`, each negated when its mirror button is down; Proportional copies a typed X into Y and Y into X |
| Size | W, H (the selection's size); Proportional (on) | scales about `a` by W / width(b) and H / height(b) (a side of zero size stays); Proportional keeps H / W at the selection's ratio |
| Skew | X, Y (0); Use anchor point (off) | `x' = x + tan(X) (y - p.y)`, `y' = y + tan(Y) (x - p.x)` about `p`: `a` with Use anchor point, else the centre of `b`; the reference point is pale without it |

- The reference point defaults to the centre. Corners and edge middles
  pick the point of `b`; the top row is the top of the selection.
- On the Position page with Relative position on, the 3 x 3 boxes pick a
  direction, not a point (the centre shows as a ring): a side or a
  corner sets X to `-width(b)`, 0 or `width(b)` and Y to `height(b)`, 0
  or `-height(b)` that way, so Apply moves the selection, or puts each
  copy, right next to the last one; the centre sets (0, 0). The offset
  follows the selection's size (a new or another selected object, after
  Apply) until X or Y is typed into or Relative position is turned off
  and on again.
- The absolute fields follow the selection: Position with Relative
  position off shows where `a` is, Rotate with Relative center off shows
  `c` (the centre moved on the canvas, else `a`), and Size shows the selection's width and height, each time the
  selection or its bounds change (also after Apply). Turning a Relative
  box on sets its fields to 0.
- Copies (0 to 500, default 0): with 0, Apply transforms the selection
  (paragraph text frames take a scale as a new frame, as everywhere).
  With n copies the selection stays as it is and n copies are added to
  the active layer, copy k transformed k times; the last copies become
  the selection, so another Apply goes on from there. The copies are one
  undo step named after the page.
- Rotate keeps its centre: after Apply (with or without copies) the
  centre of rotation of the new selection is the point it turned about,
  so Apply again goes on round the same point (copies make a wheel
  round a centre moved off the object). Picking a reference point on
  the Rotate page lets a centre moved on the canvas go, and the turn is
  about that point again.
- With nothing selected the fields, the boxes, Copies and Apply are pale;
  the page buttons stay usable.

## Checks

- `ui::transform_docker::tests::position_moves_by_an_offset_or_to_a_point`
- `ui::transform_docker::tests::rotate_scale_size_and_skew_keep_the_reference_point_still`
- `ui::transform_docker::tests::copies_repeat_the_transformation_in_one_step`
- `ui::transform_docker::tests::rotated_copies_turn_about_the_centre_moved_on_the_canvas`
- `ui::transform_docker::tests::absolute_fields_follow_the_selection`
- `ui::transform_docker::tests::relative_position_boxes_pick_a_direction_the_size_of_the_selection`
- `ui::transform_docker::tests::the_docker_draws_with_nothing_selected`
- `ui::transform_docker::tests::the_shortcuts_open_a_page_and_close_it_again`
- `ui::transform_docker::tests::the_docker_draws_every_page`
