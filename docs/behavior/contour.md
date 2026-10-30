# Contour

Concentric copies of an object's outline, inside, outside or to the
centre, as a live effect (`Effect::Contour`): the object keeps its own
outline and the steps are drawn from it, so the contour can be changed or
cleared at any time. The tool is `contour_tool.rs`; its bar and the
Contour panel share the controls in `ui/contour_bar.rs`.

| Control | Default | Range |
|---|---|---|
| To centre, Inside, Outside | Outside | |
| Steps | 3 | 1 to 999 (to centre: as many as fit) |
| Offset | 2 mm | 0.001 to 1000 mm, between steps |
| Corners | Mitred | Mitred, Round, Bevelled |
| Colour direction | Direct | Direct, Clockwise, Counterclockwise (through the hues) |
| Last step's fill and outline colours | white, black | |
| Object acceleration, colour acceleration | 0 % | -100 to 100 % |

The two colour buttons of the bar open the colour palette's swatches
and, under Other, the colour's swatch: a click on it closes the list
and opens the colour picker beside the button (viewer, Model list,
eyedropper, sliders), which stays open while it is used and closes on a
click elsewhere or Esc. The picker used to sit inside the list, so the
first click closed both and the Model list took the list away with it
(#1262, `ui::fill_tool_bars::color_pick`, shared by the colour buttons
of the Extrude and Smart Fill bars;
`ui::fill_tool_bars::tests::other_opens_a_picker_that_stays` and
`the_first_click_on_other_opens_a_picker_that_works`).

Ctrl+F9 opens the Contour panel.

## Formulas

- With `n` steps, step `i` (1 to `n`) is the object's outline offset by
  `n * offset * accelerate(i / n, objects)`: outward for Outside, inward
  for Inside. To centre: inward by `i * offset` until nothing is left.
- Distances are measured on the page: the outline is offset with the
  object's scale, stretch and rotation applied and the steps are taken
  back into the object's own coordinates, so an 80 mm inside step of a
  scaled polygon lies 80 mm from its edges (#754).
- `accelerate(t, a) = t ^ (4 ^ a)`: 0 spaces the steps evenly, positive
  values pack them toward the object, negative ones toward the last
  step.
- Corners: the offset outline's corners are mitred (sharp; corners under
  about 9 degrees are cut off), round, or bevelled. Contours from older
  files, which have no corner setting, are round.
- Colours: step `i` of the `n` drawn takes `accelerate(i / n, colours)`
  of the way from the object's fill (and outline) colour to the last
  step's colours, mixed directly or through the hues clockwise or
  counterclockwise. An object without fill gives unfilled steps.
- Outside steps are drawn behind the object, farthest first; inside
  steps in front of it, outermost first.

## The tool

- A drag starting on an object (inside it counts, filled or not): away
  from the object's middle gives an outside contour, toward it an
  inside one (To centre stays to centre); the distance from the start
  is `steps * offset` (the steps kept, the offset following). The
  contour is drawn live while dragging; one undo step.
- A click selects the object under the pointer (or nothing). Groups and
  bitmaps get no contour.

## Property bar and panel

- The controls show the selected object's contour and edit the selected
  contours live (a run of changes is one undo step). Apply (shown when
  a selected object has no contour) adds the contour; Clear Contour
  removes it; Break Apart turns the steps into objects, grouped beside
  the object when there are several (`App::break_effect_groups`).
- Object > Break Contour Group Apart (Ctrl+K, and the right-click
  menu), which takes the place of Break Apart while a selected object
  has a contour, does the same (#424;
  `contour_tool::tests::break_apart_breaks_a_contour_group`). A blend
  gives Break Blend Group Apart and an extrusion Break Extrude Group
  Apart; a drop shadow's Break Drop Shadow Apart comes first.
- The values are kept in the preferences (`tools.contour`).

## Checks

- `live::tests::contour_outside_three_steps`
- `live::tests::contour_offset_is_measured_on_the_page`
- `shaping::tests::offset_corners_are_mitred_round_or_bevelled`
- `contour_tool::tests::a_drag_draws_a_live_contour`
- `contour_tool::tests::the_bar_edits_the_selected_contour`
- `ui::contour_bar::tests::the_bar_and_the_panel_draw`
- Given a 20 mm square, outside, 3 steps of 2 mm, mitred, then the
  outermost step is a 32 mm square and its fill is the end colour.
- Given a 10 mm square offset by 2 mm, then the area is 196 mm2 mitred,
  `180 + 4 pi` round and 188 bevelled.
- Given an octagon scaled 1.7 times with its sides level, inside, one
  step of 20 mm, then the step lies 20 mm inside each side on the page;
  given a 20 mm square stretched 3 times across, one 4 mm inside step,
  then it lies 4 mm inside on all four sides.
