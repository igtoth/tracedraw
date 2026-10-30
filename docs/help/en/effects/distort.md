# Distortion

The [[tool.distort]] tool bends an object's outline in three ways: **push and
pull** (spikes or petals), **zipper** (zigzag teeth) and **twister** (a
swirl). It is a live effect: you can change, move or remove the distortion
at any time, and the object keeps its own outline.

## To distort an object {#create}

1. Choose the [[tool.distort]] tool from the Effects flyout.
2. Choose the kind on the property bar.
3. Drag on the object. The distortion is centered where you start.
   - **Push and pull**: drag right to push the nodes out (spiky shapes with
     hollow sides), left to pull them in (rounded petals).
   - **Zipper**: the further you drag, the taller the teeth.
   - **Twister**: drag around the starting point; full turns count.

The center of a selected distortion shows as a diamond: drag it to move the
center. Dragging again replaces the distortion; an object has at most one.
Groups and bitmaps are not distorted.

## Distortion settings {#settings}

The property bar shows and changes the selected distortion:

- the kind: **Push and pull**, **Zipper** or **Twister** (choosing another
  kind changes a selected distortion);
- for push and pull, the **amplitude** (-200 to 200, 20 at first);
- for zipper, the **amplitude** (0 to 100 mm, 2 mm at first) and the
  **frequency** (teeth per segment, 6 at first), with **Random** (uneven
  teeth), **Smooth** (rounded teeth) and **Local** (teeth shrink away from
  the center);
- for twister, **clockwise** or **counterclockwise**, the number of whole
  **rotations** and the extra **degrees**.

[[menu.object > menu.object.clear_effect]] removes the distortion.
