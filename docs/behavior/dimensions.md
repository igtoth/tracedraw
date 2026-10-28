# Dimension and callout tools

Parallel, Horizontal or Vertical, Angular and Segment dimensions, and
the 3-point callout. A dimension is a group of its lines, its arrowheads
and its label, with what it measures and its style in the group's object
data (`dimension`), so it is drawn again when its style changes or it is
moved, scaled, turned or skewed (`dimensions.rs`).

## Placing them

- Parallel and Horizontal or Vertical: drag from the first point to the
  second (or click both), then click where the dimension line goes (its
  distance from the measured points along their normal). Horizontal or
  Vertical measures the larger of the horizontal and the vertical span.
- Segment: a click on a segment of any object takes its two ends (a
  click away from every segment is the first point, as with Parallel);
  then click where the dimension line goes.
- Angular: the vertex, a point on the first side, a point on the second
  side, then where the arc goes (its radius; at least 1 mm). The angle is
  measured counterclockwise from the first side.
- While the last point is wanted, the dimension follows the pointer,
  drawn thin in blue; before that, a line from the last point. The
  property bar says what to click next; Escape starts again. Points snap
  as drawing does. With Ctrl held, the second point of a linear
  dimension (clicked, or where a drag from the first point ends) and
  each side of an angular one keep to steps of the constrain angle
  (Options > Edit, 15 degrees by default) from the first point or the
  vertex, as the curve tools do; the preview follows
  (`pick_drag_tests::ctrl_keeps_a_dimension_leg_to_the_constrain_angle`).
- The new dimension is selected; one undo step.

## Drawing

- Linear: extension lines from the measured points (or the extension
  gap off them) to 2 mm (the overshoot) past the dimension line, the
  dimension line, filled arrowheads (3 mm long, 2 mm wide) at both
  ends. Angular: the two sides from the
  vertex (or the gap off it) to the overshoot past the arc, the arc,
  arrowheads along it at both ends.
- The label reads left to right: along the dimension line, turned to
  stay upright (a line pointing left or down reads the other way), or
  upright with Horizontal text, its middle off the line's middle by its
  reach across the line; for angles upright, out from the middle of the
  arc.
- Text position: Above the line (1 mm above it in reading direction; for
  angles outside the arc), In the line (centred on it, the line broken
  1 mm either side of the label), Below the line (for angles inside the
  arc).
- Lines, arrowheads and label take the style's look (#475): at first
  hairlines, black filled arrowheads and a black label in the font of new
  text at the style's size
  (`dimensions::tests::the_style_sets_the_look_of_lines_heads_and_label`).
  Open arrowheads and ticks are strokes drawn with the lines; a dot is a
  filled circle at each end.

## Property bar

Shown with the dimension tools and when the Pick tool's selection is all
dimensions (after the position and size fields). Changes apply to the
selected dimensions (one undo step) and to the ones made next; the style
is kept in the preferences (`tools.dimension_style`).

| Control | Default | Range |
|---|---|---|
| Precision | 1.23 (2 decimals) | 0 to 6 decimals |
| Units | Drawing units (the document's) | any unit of the Units list; lengths go through the drawing scale (at 1:100 a 10 mm line reads 1000 mm) |
| Show units | on | the unit's sign after the number; for angles, the degree sign |
| Prefix, Suffix | empty | text before and after (applied when the field is left or Enter is pressed) |
| Text | Above the line | Above the line, In the line, Below the line |
| Label size | 10 pt | 1 to 500 pt |
| Font | the font of new text | any font (the label keeps it; empty in styles saved before) |
| Label colour | black | any colour (the swatch opens the colour picker) |
| Line colour | black | the lines and arrowheads |
| Line width | hairline (0) | 0 to 50 mm, shown in the outline units |
| Arrowheads | Arrow (filled) | Arrow, Open arrow, Dot, Tick, None |
| Round ends | off | round caps and joins on the lines |
| Options > Extension line gap | 0 | 0 to 100 mm between the measured points and the extension lines (at most the lines' length) |
| Options > Extension past the line | 2 mm | 0 to 100 mm |
| Options > Horizontal text | off | a linear dimension's label stays horizontal |
| Options > Join lines at corners | off | the extension lines run into the dimension line (or arc) as one curve with corners, nothing past it (#641) |
| Options > Corner radius | 0 | 0 to 100 mm, rounds those corners (each cut at most half an inner leg, an end leg wholly); off unless the lines are joined |

The Options menu stays open while its fields are typed in or stepped;
a click outside it closes it
(`pick_drag_tests::the_dimension_options_stay_open_while_typed_in`).
Styles saved before the options read with their defaults; lengths that
are not finite numbers read as the defaults
(`dimensions::tests::more_options_shape_the_dimension`).

Joined, a linear dimension is one open curve from the first measured
point (or the gap off it) up to the dimension line, along it and down to
the second; a centred label breaks it in two, each with one corner. An
angular one runs from the vertex (or the gap off it) out along the first
side, round the arc and back along the second side. A rounded corner is
a quarter-circle cubic from the cut on one leg to the cut on the other
(on the arc the same length along it), so it stays smooth for Convert to
Curves, Break Apart and the Shape tool. Both options are kept in the
dimension, so in a saved `.tdraw` file, and in the bar's style for the
next ones; styles saved before read with both off and a radius that is
not a finite number reads as 0
(`dimensions::tests::joined_lines_meet_in_rounded_corners`,
`dimensions::tests::joined_corners_are_kept_in_the_file`).

## Editing the label

The label is text and is edited in place (#644): a double click on it
with the Pick tool, or a click on it with the Text tool, edits it with
the caret where the pointer is (the Pick tool switches to the Text
tool); a drag inside it selects characters. After every change of the
drawing, in the same undo step, the dimension reads what was done to its
label back and draws itself again:

- typed text that still holds the measured value as it was drawn (as a
  whole number: no digit or decimal sign right before it, no digit or
  decimal part right after, so `40.5` where the value was `40` is
  another number) keeps it live: the text before it becomes the prefix, the text
  after it the suffix, and a new size changes the number;
- typed text without it (`140.00 mm` where the value was `40.00 mm`
  counts as without) shows as typed whatever the size, until the
  label's text is deleted, which brings the measured value back;
- the label's font, size and colour become the style's (the first
  span's font and size: bold, italic and the looks of other characters
  are not kept); a fill picked for a selected dimension (a palette
  swatch) so colours its label only, the lines and arrowheads keeping
  the line colour, and an outline colour picked for it becomes the line
  colour (`dimensions::tests::a_dragged_fill_on_a_dimension_is_one_step`:
  a colour dragged in a picker stays one undo step while it is read
  back);
- a label emptied while it is typed stays empty; when its editing ends
  it is deleted, which brings the measured one back in that step (a part
  of the group deleted, or a label left empty otherwise, is drawn again
  too).

The follow-up joins the step just recorded and is never made after an
undo, a redo or opening a drawing, so undo and redo go through every
step (`dimensions::tests::undo_after_an_emptied_label_goes_all_the_way_back`).

Edits are compared with how the label was last drawn (noted in the
object data), so new drawing units or a new font for new text do not
make a measured label a typed one. A dimension saved before labels were
noted reads its label as drawn until it is drawn again (moved, scaled or
restyled), so opening an older file changes nothing; an edit of its
label begun in place is read back against the label as it was then
(noted for that edit only: it goes when the editing ends or the drawing
is left for another tab, `dimensions::tests::noted_labels_go_with_the_edit`,
`dimensions::tests::decimals_older_dimensions_and_fills_read_back`,
`dimensions::tests::a_label_edited_in_place_stays_linked`,
`dimensions::tests::new_units_or_default_font_keep_labels_measured`,
`pick_drag_tests::a_double_click_edits_a_dimension_label_in_place`).

## Break Apart

Break Apart (Ctrl+K) on a dimension frees its lines, its arrowheads and
its label as objects of their own, selected, in one undo step; open,
tick or no arrowheads (drawn with the lines) leave no empty object. The
pieces are no longer a dimension. A dimension inside a group is left as
it is
(`dimensions::tests::break_apart_frees_the_lines_heads_and_label`).

## Dynamic dimensioning

When a dimension is moved, scaled, turned or skewed, its transform goes
into its measured points and it is drawn again in the same undo step: a
dimension scaled by 2 reads twice the length, its label stays upright,
undistorted text of its size, and the group keeps no transform.

## Held to the object it measures

A measured point placed within 3 pixels of an object's node, or else of
a point along one of its segments (an edge, its midpoint or any point on
it), holds to it (#838, #1178). Each anchor notes the object, and either
the node's place among its nodes or the segment's place among the
outline's segments with the point's share of its length (0 at the
segment's start, 1 at its end); a node wins over a segment, so a corner
stays a corner. The anchors are kept in the object data, so they
survive the file; older files hold node anchors only.

When the object is moved, resized, turned or reshaped (its outline
replaced, a node dragged), the step is followed up in the same undo
step: each held point goes to its node or to the same share of its
segment, the dimension line or arc moves as far as the held points
moved on average, and the label reads the new value. A point on a
stretched edge keeps its share of the edge, so a dimension between two
sides of a rectangle grows with it. A dimension moved by itself lets go
of the object, as does one whose object, node or segment is gone
(deleted, or the outline now has fewer segments).

## Convert to Curves

Convert to Curves (Ctrl+Q) on a dimension makes it a group of plain
curves (#640): its lines (one open curve, the outline kept), its filled
arrowheads (none for open, tick or no arrowheads) and its label as
closed curves with its fill. The group is no longer a dimension: it is
not drawn again when moved or scaled, and each curve can be edited with
the Shape tool. One undo step. An angle's arc is drawn as a few clean
curves (at most one per quarter turn), not as many short lines
(`dimensions::tests::convert_to_curves_makes_a_dimension_plain_curves`).

## 3-point callout

A callout is a group of its leader (an open path from the tip through
the elbow to the text, with a head at the tip), the shape round its text
when its style has one, and its text, with the tip, the elbow and the
style in the group's object data (`callout`; `callouts.rs`).

### Placing one

- A drag from the tip (what the callout points at) to the elbow, or a
  click on each, then a click where the text goes. While the next point
  is wanted, the leader so far follows the pointer, drawn thin in blue;
  the property bar says what to click next and Escape starts again.
  Points snap as drawing does.
- The text goes on the side of the elbow the third click is on: to its
  right, left aligned, or to its left, right aligned, so it grows away
  from the leader. It is placed so the leader ends at the third click,
  in the current font, size, bold and italic.
- The text opens for typing at once with its sample text ("Callout
  text") selected, so typing replaces it. One undo step makes the
  callout; the typing is another.

### Fitting

After every change of the drawing, in the same undo step:

- the leader ends at the middle of the text's side, `gap` away; with an
  underline it runs on under the text, `gap` below it and `gap` past
  both ends; with a rectangle, a rounded rectangle (corner radius 30 %
  of the shorter side, at most 3 mm) or an ellipse (through the corners
  of the text's box grown by `gap`), it ends at the middle of the
  shape's side;
- typed text, a new size or the text moved on its own (Ctrl+click it)
  take the leader's end and the shape along;
- a tip or an elbow moved with the Shape tool stays where it was put
  (they are read back from the leader's first two nodes); the leader's
  end always goes back to the text.

The group can be moved, scaled and turned as any group. The text's box
is its lines, from the left of the first character to the right of the
last and from the lowest descent to the highest ascent.

### Property bar

With the 3-point callout tool, when the Pick tool's selection is all
callouts (after the position and size fields), and while a new
callout's text is typed. Changes apply to the selected callouts (one
undo step; only what changed, so a leader's colour stays) and to the
ones made next; the style is kept in the preferences
(`tools.callout_style`).

| Control | Default | Range |
|---|---|---|
| Outline width | Hairline | Hairline, 0.1 to 3 mm (leader and shape) |
| Line style | solid | the outline dash presets |
| Head at the tip | arrow | the arrowhead presets (None for a plain line) |
| Callout shape | None | None, Underline, Rectangle, Rounded rectangle, Ellipse |
| Gap | 1 mm | 0 to 100 mm |

## Checks

- `dimensions::tests::labels_follow_the_style` (25.4 mm: `25.40 mm`,
  `W 1.000 " max` in inches with three decimals, a prefix and a suffix,
  `25` bare; a right angle `90.0°`)
- `dimensions::tests::dimensions_redraw_for_their_style_and_their_size`
- `dimensions::tests::previews_and_segments`
- `dimensions::tests::a_dimension_follows_the_object_it_measures` (two
  corners of a rectangle; moved by itself it lets go)
- `dimensions::tests::a_dimension_between_two_edges_follows_the_object`
  (two sides of a rectangle: moved, scaled, given a new size, saved)
- `dimensions::tests::a_dimension_lets_go_of_a_deleted_object`
- `dimensions::tests::convert_to_curves_makes_a_dimension_plain_curves`
- `dimensions::tests::break_apart_frees_the_lines_heads_and_label`
- `dimensions::tests::a_label_edited_in_place_stays_linked`
- `dimensions::tests::new_units_or_default_font_keep_labels_measured`
- `dimensions::tests::more_options_shape_the_dimension`
- `dimensions::tests::joined_lines_meet_in_rounded_corners`
- `dimensions::tests::joined_corners_are_kept_in_the_file`
- `dimensions::tests::undo_after_an_emptied_label_goes_all_the_way_back`
- `dimensions::tests::decimals_older_dimensions_and_fills_read_back`
- `dimensions::tests::a_dragged_fill_on_a_dimension_is_one_step`
- `dimensions::tests::noted_labels_go_with_the_edit`
- `pick_drag_tests::the_dimension_options_stay_open_while_typed_in`
- `pick_drag_tests::a_double_click_edits_a_dimension_label_in_place`
- `ui::dimension_bar::tests::the_bar_and_the_preview_draw`
- `callouts::tests::a_callout_ends_at_its_text_and_follows_it`
- `callouts::tests::a_tip_moved_with_the_shape_tool_stays`
- `callouts::tests::text_left_of_the_elbow_is_right_aligned_and_shapes_fit`
- `callouts::tests::odd_callout_data_is_left_alone`
- `ui::callout_bar::tests::the_bar_and_the_preview_draw`
