# Moving, scaling, rotating and skewing

You can transform objects by dragging them with the [[tool.pick]] tool (see
[Selecting objects](selecting.md#transform-with-pick)), by typing values on
the property bar, with the [[tool.free_transform]] tool, or precisely with
the [[docker.transformations]] panel.

## The Transformations panel {#transformations-docker}

Open the panel with [[menu.object > menu.object.transformations]], or with
<kbd>Alt+F7</kbd> ([[docker.position]]), <kbd>Alt+F8</kbd>
([[docker.rotate]]), <kbd>Alt+F9</kbd> ([[docker.scale_and_mirror]]) and
<kbd>Alt+F10</kbd> ([[docker.size]]). The buttons at the top choose one of
five pages.

![The Transformations panel](../../img/objects-transforming-transformations-docker.png)

Every page ends with the **reference point**, a 3 by 3 grid that chooses the
point of the selection that is placed, turned about or kept still (the
center at first), [[dialog.copies]] and [[docker.apply]]. Distances are in
the drawing units, angles in degrees (counterclockwise), scales in percent.

To transform the selection:

1. Choose a page and type the values.
2. Choose the reference point.
3. Set [[dialog.copies]]: 0 changes the selection itself; a number above 0
   leaves the selection as it is and adds that many copies, each transformed
   once more than the one before. The copies are one undo step, and the last
   copy becomes the selection, so another [[docker.apply]] goes on from there.
4. Click [[docker.apply]].

### Position {#position}

[[docker.x]] and [[docker.y]] move the selection. With
[[docker.relative_position]] on (at first), they are a distance to move by;
off, they are where the reference point goes, counted from the ruler origin.
With [[docker.relative_position]] on, the grid picks a direction: a side or a
corner sets the distance to the selection's size, so each copy lands right
next to the last one.

### Rotate {#rotate}

Type the angle, and the [[docker.center_colon]] to turn about. With
[[docker.relative_center]] on (at first), the center is counted from the
reference point; off, from the ruler origin.

### Scale and Mirror {#scale-and-mirror}

[[docker.x]] and [[docker.y]] scale in percent (100% at first).
[[toolbar.mirror_h]] and [[toolbar.mirror_v]] flip the selection.
[[docker.proportional]] keeps both scales the same.

### Size {#size}

[[docker.w]] and [[docker.h]] set the width and height of the selection.
[[docker.proportional]] keeps the proportions.

### Skew {#skew}

[[docker.x]] and [[docker.y]] slant the selection by angles. With
[[docker.use_anchor_point]] on, the reference point stays still; off, the
center does.

## The Free Transform tool {#free-transform}

The [[tool.free_transform]] tool, in the Pick flyout, transforms the selection
about the point where you press. Choose a mode on the property bar:

- **Free rotation** (at first): turns the selection around the press point.
- **Free angle reflection**: mirrors the selection across the line from the
  press point to the pointer; moving the pointer turns that line.
- **Free scale**: scales about the press point.
- **Free skew**: horizontal movement skews horizontally, vertical movement
  vertically.

Hold <kbd>Alt</kbd> to scale, <kbd>Ctrl</kbd> to skew or <kbd>Shift</kbd> to
reflect for one drag, whatever the mode. With **Apply to duplicate** on, the
press makes a copy and the drag transforms the copy, leaving the original as
it was. The property bar also has the position, size, scale, angle and mirror
fields. One drag is one step: <kbd>Ctrl+Z</kbd> puts the selection back as it
was before the drag (and takes the copy away).

## Undoing transformations {#clear}

[[menu.object > menu.object.clear_transformations]] takes the selected objects
back to how they were before they were scaled, rotated or skewed.

## The Coordinates panel {#coordinates}

The [[docker.coordinates]] panel draws an object from typed coordinates, or
reshapes the selected one, with a dashed preview on the drawing. Open it from
[[menu.window > menu.window.dockers]].

![The Coordinates panel](../../img/objects-transforming-coordinates.png)

1. Choose the kind of object with the buttons at the top: rectangle or
   square, ellipse or circle, polygon or regular polygon, star, complex star,
   2-point line or multipoint curve.
2. Type its values:
   - rectangles and ellipses: the position of the origin point (chosen in a
     3 by 3 grid), the width and height, the angle of rotation, and the
     corners of the bounding box while the angle is 0;
   - squares and circles: the position, the side or diameter and the angle;
   - polygons and stars: the number of points or sides, the sharpness, the
     angle and the circle the points lie on;
   - regular polygons: the number of sides, the length of a side, the angle
     and the center;
   - 2-point lines: the start point and end point, or the length and angle;
   - multipoint curves: the list of points, with buttons to add and delete
     points and to close the curve.
3. Instead of typing, click a button with a pointer and target, then click or
   drag on the drawing to set that value. A right click cancels.
4. Click **Create object** to add it, or **Replace object** to reshape the
   selected rectangle, ellipse, polygon or curve while keeping its fill,
   outline, name and place.

Positions are counted from the ruler origin, in the drawing units.
