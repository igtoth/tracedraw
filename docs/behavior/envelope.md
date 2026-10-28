# Envelope

An envelope is a frame of nodes around an object; the object is mapped
into it and follows when the nodes move. It is a live effect: the object
keeps its own geometry and the envelope can be edited, replaced or
cleared at any time (Effects > Envelope opens the Envelope panel,
Ctrl+F7).

## The frame

- A new frame has eight nodes: the four corners of the object's bounding
  box and the middle of each side, joined by straight segments. Nodes run
  from the bottom-left corner along the bottom, up the right side, back
  along the top and down the left side.
- Each segment, from a node to the next, is a cubic curve with two
  handles. Corners cannot be removed; other nodes can, down to four.
- The frame is kept in the object's own space, so moving, scaling or
  turning the object takes the envelope along.
- Stored as `Effect::Envelope { nodes, handles, corners, keep_lines,
  mapping }`. Files written before handles existed have only the eight
  nodes: each side is then the single arc through its three nodes (the
  quadratic through corner, middle and corner), which is what they showed.
- On a group (or a clip frame) the frame is around the box of all its
  objects, and each object is mapped from that box into the frame,
  keeping its own fill and outline; perspective and distortions on a
  group work the same way. The group used to be drawn as one path with
  its own empty fill, so it vanished
  (`tracedraw_core::live::tests::an_envelope_on_a_group_keeps_each_object`).
- Curves are flattened before the mapping with a tolerance of 1/50 000
  of the object's size (at most 0.05 units), so letters stay smooth; a
  fixed 0.05 left chords about 2 mm long on large text
  (`an_envelope_keeps_small_curves_smooth`, #455).

## Envelope tool (Effects group of the toolbox)

- A click on an object selects it and gives it an envelope if it has
  none; a click on nothing clears the selection.
- Dragging a node moves it the way the mode says; the whole drag is one
  undo step, measured from where it began.
- In the Unconstrained mode a click on a node selects it and shows its two
  handles (the end of the segment coming in and the start of the one
  going out); dragging a handle bends that segment alone.
- Double-click on the frame adds a node there (the curve keeps its
  shape); double-click on a node that is not a corner removes it.
- The Shape tool edits the frame of a selected enveloped object the same
  way. Other tools do not show it; the Envelope tool does not show the
  selection handles.
- Over a node or a handle the pointer is the Shape tool's arrow with the
  node or handle mark.

## Modes

How dragging a node reshapes the sides it is on (both sides for a
corner); the mode is a setting of the tool, not of the envelope, and it
is kept in the preferences (`tools.envelope_mode`).

| Mode | The sides the moved node is on become |
|---|---|
| Straight line | straight from node to node (handles a third and two thirds along each segment) |
| Single arc | one smooth arc through their nodes: tangents `(p[i+1] - p[i-1]) / 2` inside, `2 (p[1] - p[0]) - t[1]` and `2 (p[m] - p[m-1]) - t[m-1]` at the ends (three nodes make the quadratic through them) |
| Double arc | S curves: each segment leaves and reaches its nodes along the side (across for the bottom and the top, upright for the sides), handles a third of the way along |
| Unconstrained (first choice) | unchanged: the node takes its two handles along, and handles move on their own |

## Mapping

How the object's bounding box fills the frame; it belongs to the
envelope (new envelopes take the last one chosen, `tools.envelope_mapping`).

| Mapping | A point `(u, v)` of the box (0 to 1 across and up) goes to |
|---|---|
| Putty (first choice) | the Coons patch of the four sides, each side's segments taking equal parts of it: `(1-v) B(u) + v T(u) + (1-u) L(v) + u R(v) - [(1-u)(1-v) P00 + u(1-v) P10 + (1-u)v P01 + uv P11]` |
| Original | the same patch with each side measured by its length, so the object spreads evenly along the frame however its nodes are spaced |
| Horizontal | height `v` of the frame's height; across, from where that row meets the frame's outline on the left to where it leaves on the right |
| Vertical | `u` of the frame's width; up, from where that column meets the outline at the bottom to where it leaves at the top |

Keep Lines (`keep_lines`) maps only the nodes and control points of the
object, so its straight segments stay straight; without it the object
is divided finely enough to bend: segments of 1/64 of its larger side,
each halved again (up to six times) while the point its middle maps to
lies more than 1/100 000 of the object's size off the mapped chord, so a
tight bend is drawn smooth and a gentle one takes no more points
(`a_hard_bend_is_drawn_without_facets`, #881). Which segments to
measure is judged first from how far each mapped point lies off the
chord of its neighbours (a quarter of that is about how far a segment
strays), so a gentle warp costs no more work; a line shorter than 1/256
of the larger side (a curve's flattening) is not measured. Horizontal and Vertical
find where each of 1025 rows (or columns) meets the frame by solving
its cubic segments, not a polygon through them, so the object's sides
lie on the frame's curves (`the_rows_follow_the_frame_curves`, #881).
The frame itself is drawn in as many lines as keep it within a quarter
pixel of its curves at the current zoom (it was 16 lines a segment,
which showed as a row of lines zoomed in).

## Property bar (Envelope tool)

Presets list | Straight Line, Single Arc, Double Arc, Unconstrained | (in
the Unconstrained mode) Add Node, Delete Node | Add New Envelope,
Mapping list, Keep Lines | Copy Envelope Properties, Clear Envelope; with
nothing selected, how to start.

- Presets: Arch, Bulge, Squeeze, Flag, Perspective, Circle; one replaces
  each selected object's last envelope (or gives it one), fitted to the
  object as the envelope takes it, in one undo step.
- Add Node puts a node halfway along the segment after the selected node;
  Delete Node removes the selected node (pale on a corner).
- Add New Envelope puts a fresh frame around each selected object as it
  now looks; an envelope it already has stays under the new one.
- The mapping list and Keep Lines show the first selected envelope's;
  changing them changes the selected envelopes (one undo step) and what
  new envelopes take.
- Copy Envelope Properties (also Object > Copy Effect > Envelope From):
  the next click picks the object to copy from; the selected objects take
  its last envelope, stretched from its box to theirs, with its mapping
  and Keep Lines. A right click cancels. An object with no envelope gives
  the shape of its outline instead (its largest closed part, as drawn on
  the page), so text or any object is fitted into a drawn shape: the
  frame's nodes are the outline's, its corners the outline's points
  nearest the corners of its box (a node goes in there), stretched from
  that box to each object's, with the mapping and Keep Lines new
  envelopes take (#993, `tracedraw_core::envelope::tests::frames_are_made_from_outlines`,
  `text_effect_tests::text_takes_the_shape_of_an_object_as_its_envelope`).
  An open line, or an outline with more than 96 nodes, gives nothing and
  the status bar says so.
- Clear Envelope removes each selected object's last envelope.
- Convert to Curves (Ctrl+Q) and Break Apart (Ctrl+K) make the envelope
  permanent: the object becomes curves drawn as the envelope drew it, and
  the envelope is gone. Break Apart then breaks the warped curve into its
  parts, and enveloped artistic text into one curve per letter, each
  warped as the whole text was, with its holes kept (O, B, A). The
  warped outline is fitted with curves (`tracedraw_core::fit::fit_path`,
  corners kept, within a ten-thousandth of the outline's size, 0.005 to
  0.05 mm), so the result is smooth curves with about as many nodes as
  the outline had before, not the warp's dense lines ("ABC": 52 nodes
  for 56 unwarped, 826 before, #647,
  `text_effect_tests::baked_envelopes_are_fitted_with_few_curves`).
  Break Apart is offered by the canvas context menu for a warped object
  and for artistic text as well as by the Object menu and Ctrl+K
  (`pick_drag_tests::the_context_menu_breaks_an_enveloped_text_apart`). An
  envelope after a contour, blend or extrusion stays live, and a bitmap
  keeps its envelope. Break Apart leaves a group, clip frame, table or
  symbol instance with its envelope (there is nothing to break; Convert to
  Curves bakes it). See `convert-to-curves.md`.

Text with characters of their own colour or outline keeps them under an
envelope: it is drawn as one warped curve for each look, all warped
together in the box of the whole text, so the letters sit exactly where
the text in one colour has them (#912,
`text_effect_tests::an_envelope_keeps_the_colours_of_the_letters`).

## Panel

A full-width Add New (which also takes the Envelope tool); "Choose a
preset" over a white grid of 24 preset shapes, four to a row, drawn as
their outlines (square, arched top, circle, pill, parallelogram,
trapezoid, hexagons, octagons, flag, heart, pinched square, four-point
star, barrel, rounded rectangle, cushion, diamond, lens, badge, wavy
rectangle, cross, cloud and speech bubble); a click puts that shape on
each selected object as its envelope (in place of its last one), one
undo step. Under a line the four modes as pictures with the mapping
list at the right; under another line Keep Lines, Copy Envelope
Properties and Clear Envelope as pictures. The shapes are outlines in
the unit square whose corners map the object's box
(`envelope_shapes::tests::every_preset_makes_a_frame_with_four_increasing_corners`).

## Selection and picking

The selection box, the property bar's position and size, alignment, a
click and a marquee all go by what an object with live effects draws
(`Shape::visual_bounds`): a click on the arch above an enveloped
rectangle selects it, and the selection box reaches the top of the arch.

## Checks

- `tracedraw_core::envelope::tests::a_fresh_frame_changes_nothing`
- `tracedraw_core::envelope::tests::older_envelopes_are_single_arcs`
- `tracedraw_core::envelope::tests::the_modes_shape_the_sides_a_node_moves`
- `tracedraw_core::envelope::tests::nodes_are_added_and_removed`
- `tracedraw_core::envelope::tests::the_mappings_fill_the_frame_differently`
- `tracedraw_core::live::tests::visual_bounds_follow_the_live_effects`
- `envelope_edit::tests::dragging_a_node_is_one_step_in_the_chosen_mode`
- `envelope_edit::tests::handles_and_nodes_in_the_unconstrained_mode`
- `envelope_edit::tests::enveloped_objects_are_hit_and_boxed_as_drawn`
- `envelope_edit::tests::presets_settings_clear_and_copy`
- `envelope_edit::tests::convert_to_curves_bakes_the_envelope`
- `envelope_edit::tests::break_apart_bakes_an_enveloped_curve`
- `envelope_edit::tests::an_enveloped_group_converts_object_by_object`
- `text_effect_tests::break_apart_bakes_the_envelope_into_letter_curves`
- `text_effect_tests::break_apart_keeps_the_hole_of_a_single_warped_letter`
- `text_effect_tests::breaking_several_warped_texts_keeps_the_stacking_order`
- `text_effect_tests::an_envelope_keeps_the_colours_of_the_letters`
- `envelope_edit::tests::break_apart_leaves_an_enveloped_group`
- `tracedraw_core::live::tests::a_baked_group_object_loses_its_spline`
- `tracedraw_core::live::tests::outline_warp_warps_parts_as_the_whole`
- `ui::envelope_ui::tests::the_bar_and_the_docker_draw`
