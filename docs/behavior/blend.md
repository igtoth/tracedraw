# Blend

Intermediate objects between a start and an end object.

| Control | Default | Range |
|---|---|---|
| Steps | 20 | 1..999 |
| Rotation | 0 degrees | -3600..3600 (the panel: -360..360) |
| Acceleration (objects, colours) | 0 (linear) | -1..1 (the bar: -100..100 %) |
| Path | none | any curve (the panel's New path), "rotate all objects" |
| Spacing (along a path) | off, 5 mm | the steps follow from the path's length: `floor(length / spacing) - 1`, 1 to 999 |
| Loop | off | the rotation turns the steps about the middle between the two objects |
| Blend along full path | off | along a path, the first and last steps sit on the path's ends |
| Apply to sizing | off | the object acceleration changes the steps' sizes too |
| Color blend | Direct | Direct (straight through RGB), Clockwise (hue increasing), Counterclockwise (hue decreasing), through HSB |

A blend asking for more than 999 steps (from a file or a script) draws
999 (`tracedraw_core::live::tests::a_blend_draws_at_most_its_maximum_steps`, #980).

Not yet: mapping nodes, splitting and fusing.

## Panel (Effects > Blend)

Two picture buttons choose Steps or Spacing (Spacing only for a blend
on a path), then the Steps and Spacing fields (the one not chosen is
greyed), Rotation, Loop (with a rotation), Blend along full path and
Rotate all objects (with a path). Under "Acceleration" the Object and
Color sliders, Apply to sizing, Link acceleration (the colour slider
follows the object one and is greyed) and Reset (both back to even).
Under "Color Blend" three picture buttons: Direct, Clockwise and
Counterclockwise. With a blend selected every change applies live and
New path, Clear Blend and Break Apart follow; otherwise a full-width
Apply blends the two selected objects with these settings
(`tracedraw_core::live::tests::blend_options_change_the_steps`).

## Property bar (Blend tool)

Steps, Rotation, Object acceleration, Color acceleration; with nothing
blended selected they are what the next blend takes (Steps is kept in the
preferences), with a blend selected they change it live (a run of changes
is one undo step), and Clear Blend and Break Apart follow; otherwise a
line on dragging from one object to another.

Interactive: drag from the start object to the end object; the start
and end are the two selected objects' order on drop.

## On the canvas (Blend tool)

A click on a blended object (the start, the end or a step; text as any
other object) selects it and picks its blend; a click on empty space
lets go. The selected blend shows a white square on the start object's
centre, a crossed one on the end object's, the line between them, and
two sliders on that line: the object acceleration (triangle above the
line) and the colour acceleration (triangle below). Each sits where the
middle step lands, `0.5 ^ (1 + acc)` of the way from the start, so at
0 both are halfway. Dragging a slider along the line sets its
acceleration to the one that puts the middle step there (two decimals,
-1..1): toward the start bunches the steps (or the colours) near the
start, toward the end near the end. With Link acceleration both
follow either slider. Each drag is one undo step and the bar shows the
new values; a press on a slider never starts a new blend
(`blend_tool.rs`, `text_effect_tests::the_blend_tool_picks_a_text_blend_and_its_sliders_tune_it`).

Dragging the start or end square moves that object (one undo step for
the drag, the steps and the line following), so the blend's direction
changes; let go on another object of the page (not the blend's own two),
the move is taken back and that object becomes the new start or end
with the blend's settings, the old start staying a plain object and the
selection moving to the new start (#965,
`text_effect_tests::the_blend_tool_s_end_squares_move_and_pick_the_end_objects`).
Not yet on the canvas: dropping colours on the squares.

The steps lie between the two objects in the stacking order: with the
end object below the start one (a front object blended to one behind
it, the usual way to build depth), they are drawn below the start, the
step nearest the end lowest; with the end above, above the start. The
blend follows when either object is reordered
(`blend_link::tests::the_steps_lie_between_the_two_objects`).

## Start and end objects

Both objects stay objects of the drawing. The start object holds the
Blend effect, which names the end object and keeps a copy of it in page
space (its own effects left out) for drawing the steps; the end object
draws itself (`blend_link.rs`). After every change of the drawing, in
the same undo step:

- the end object moved, resized, turned or recoloured: the copy is
  brought up to date and the steps follow;
- the start object changed: the steps follow (they are drawn from it);
- the end object deleted: the blend goes with it (Undo brings both
  back).

Selecting either object picks the blend for the bar and the Blend
panel. Clear Blend removes the blend and leaves both objects; Break
Apart turns the steps into objects. Files from before this kept the end
only inside the effect; such a blend draws its end with the steps.

## Formulas

For step `k` of `n` (`t = (k) / (n + 1)` with acceleration `t' = t ^ (1 + 2*acc)`):
- Geometry: the end outline is wound the way the start outline is and
  started where it best fits the start outline's first node (the two
  outlines compared as 120 points each, about their own centres and at
  their own sizes, over every place the end outline could start), so
  equal shapes blend without twisting through the middle whatever their
  relative position. Then each outline gains a node where the other has
  one at the same share of its length (nodes less than half of one
  120th of the length apart pair up instead), and the steps are
  interpolated node by node, control points included: a step's side is
  straight where both paired sides are straight. So a square into a
  circle gives steps of eight nodes that round off evenly, and two
  rectangles give rectangles with their corners kept (#1020). Open
  outlines, and closed ones with more than 512 nodes together, are
  paired as 120 points spread evenly along their length, each step a
  polygon through them. Outlines of several parts (letters, holes)
  blend part by part, the n-th into the n-th, the same way.
  A blend read from a `.cdr` file (versions 9 and 13) pairs the two
  outlines node for node instead, each run from its first node as the
  file gives it: every node of a step lies on the straight line between
  its two paired nodes (control points too), so a step between a
  rectangle and a pentagon is a pentagon with sharp corners. When one
  outline has fewer nodes, its last segment (the one closing back to its
  first node) is cut into equal parts until the counts match. Open
  outlines, and outlines of more than 4096 nodes, are paired by arc
  length.
- Transform: interpolated as translation + rotation + scale (not matrix
  entries), so blends rotate the short way unless "rotation" adds turns.
- Colour: mixed directly (each colour's values from start to end).
- Along a path: step `k` is placed at arc length `t' * L` on the path,
  oriented to the tangent when "rotate all objects" is on.

## Checks

- `ui::effect_bars::tests::the_blend_bar_edits_the_selected_blend`
- `blend_link::tests::the_end_object_stays_and_the_steps_follow_it`

- Given two 10 mm circles 100 mm apart, 9 steps, then the 5th step is
  centred at 50 mm and has a 10 mm diameter.
- Given two equal ellipses, the second down and to the right, every step
  keeps the ellipse's width and height within 1 mm; reversing the second
  outline changes nothing.
- Given red to blue direct with 1 step, then the step is (128, 0, 128).
- Given two texts (or any outlines of several parts), part i of the start
  morphs into part i of the end; a part with no partner grows from or
  shrinks to a point at its centre, so the steps read as letters
  (`blends_of_several_parts_morph_part_by_part`,
  `the_blend_tool_blends_two_texts_and_a_text_with_a_shape`).

Break Apart (the docker's, or Object > Break Blend Group Apart, Ctrl+K
and the right-click menu, which take the place of Break Apart while a
selected object holds a blend) turns the steps into objects, grouped
when there are several (see `contour.md`, #424).
