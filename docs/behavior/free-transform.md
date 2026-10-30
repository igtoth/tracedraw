# Pick flyout: Freehand Pick and Free Transform

## Freehand Pick

- Works as the Pick tool (clicks, Shift, Ctrl and Alt clicks, the
  handles and rotate mode, moving, the pointer, the property bar) except
  that a drag from empty space draws a lasso (a dashed blue line) instead
  of a frame.
- When the button is released, every object on an unlocked, visible
  layer lying wholly inside the lasso (every point of its outline; for
  text, bitmaps, tables and clip frames their box) is added; with Alt
  held at the end, every object the lasso touches (the same rules as the
  Pick tool's Alt marquee). The lasso is closed from its last point to
  its first and tested even-odd; fewer than three points select nothing.
- With Alt held at the press the lasso starts also over an object that
  is not selected, as with the Pick tool's frame (#1350).
- With Ctrl held, the lasso is a rectangle from where the drag began to
  the pointer.
- Check: `pick_drag_tests::the_freehand_pick_lassoes_and_has_the_pick_tools_handles`.

## Free Transform

- A drag transforms the selection about the press point. If the press
  is on an unselected object, that object becomes the selection first.
- Property bar: four modes, Apply to Duplicate, then the Pick tool's
  object fields (X, Y, W, H, scale, angle, mirror).
  - Free Rotation (default): rotate by the change of the pointer angle
    around the press point.
  - Free Angle Reflection: mirror across the line from the press point
    through the pointer. The first step reflects; moving the pointer
    then turns the mirror line, which is a rotation by twice the angle
    change, so the result is always one reflection across the current
    line.
  - Free Scale: scale uniformly about the press point by the ratio of
    the pointer distances to it, step by step.
  - Free Skew: horizontal pointer motion skews along x, vertical along
    y, by `distance in mm / 50` per step, about the press point. The
    step is a shear along x followed by a shear along y, so it keeps the
    area and never mirrors (one skew by both amounts turned the object
    over once their product passed 1, as a quick diagonal stroke did to
    text, #1342).
- Modifier keys override the mode for one drag: Alt scales, Ctrl skews,
  Shift reflects.
- Apply to Duplicate: the press duplicates the selection in place (no
  duplicate offset) and the drag transforms the copy; the original is
  untouched.
- Every step is one `TransformShapes` command, all of them in one undo
  group (`Engine::begin_group` at the press, `end_group` at the
  release): one Undo puts the selection back as it was before the drag,
  and with Apply to Duplicate it also takes the copy away (#1342).

## Checks

- Given a rectangle, when Free Angle Reflection is dragged from (10, 10)
  to the right, then the point (20, 15) lands on (20, 5); turning the
  line to 45 degrees moves it to (15, 20), which equals one reflection
  across the 45 degree line.
- Given any mode, the press point stays fixed under the step.
- Given Apply to Duplicate, after the drag the document has one more
  object and the original bounds are unchanged.
- Given any drag, one Undo restores the object exactly; given a Ctrl
  drag in one large diagonal step on a text, the text is not mirrored
  (`pick_drag_tests::a_free_transform_drag_is_one_undo_step`,
  `pick_drag_tests::free_skew_never_mirrors_text`).
