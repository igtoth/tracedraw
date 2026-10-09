# Pick flyout: Freehand Pick and Free Transform

## Freehand Pick

- Drag a lasso on the canvas. When the button is released, every object
  on an unlocked, visible layer whose bounds centre lies inside the
  lasso polygon is selected (point-in-polygon test, even-odd). Fewer
  than three points select nothing.
- A click selects the object under the pointer like the Pick tool; the
  property bar is the Pick tool's.

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
    y, by `distance in mm / 50` per step, about the press point.
- Modifier keys override the mode for one drag: Alt scales, Ctrl skews,
  Shift reflects.
- Apply to Duplicate: the press duplicates the selection in place (no
  duplicate offset) and the drag transforms the copy; the original is
  untouched.
- Every step is one `TransformShapes` command; the drag is undone as a
  sequence.

## Checks

- Given a rectangle, when Free Angle Reflection is dragged from (10, 10)
  to the right, then the point (20, 15) lands on (20, 5); turning the
  line to 45 degrees moves it to (15, 20), which equals one reflection
  across the 45 degree line.
- Given any mode, the press point stays fixed under the step.
- Given Apply to Duplicate, after the drag the document has one more
  object and the original bounds are unchanged.
