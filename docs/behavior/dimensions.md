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

- Linear: extension lines from the measured points to 2 mm past the
  dimension line, the dimension line, filled arrowheads (3 mm long, 2 mm
  wide) at both ends. Angular: the two sides from the vertex to 2 mm past
  the arc, the arc, arrowheads along it at both ends.
- The label reads left to right: along the dimension line, turned to
  stay upright (a line pointing left or down reads the other way); for
  angles upright, out from the middle of the arc.
- Text position: Above the line (1 mm above it in reading direction; for
  angles outside the arc), In the line (centred on it, the line broken
  1 mm either side of the label), Below the line (for angles inside the
  arc).
- Lines are hairlines, arrowheads and label black, the label in the
  current font at the style's size.

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

## Dynamic dimensioning

When a dimension is moved, scaled, turned or skewed, its transform goes
into its measured points and it is drawn again in the same undo step: a
dimension scaled by 2 reads twice the length, its label stays upright,
undistorted text of its size, and the group keeps no transform.

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
- `ui::dimension_bar::tests::the_bar_and_the_preview_draw`
- `callouts::tests::a_callout_ends_at_its_text_and_follows_it`
- `callouts::tests::a_tip_moved_with_the_shape_tool_stays`
- `callouts::tests::text_left_of_the_elbow_is_right_aligned_and_shapes_fit`
- `callouts::tests::odd_callout_data_is_left_alone`
- `ui::callout_bar::tests::the_bar_and_the_preview_draw`
