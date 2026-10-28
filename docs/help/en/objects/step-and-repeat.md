# Making copies with Step and Repeat

The [[docker.step_and_repeat]] panel makes several copies of the selection,
each moved by the same distance from the one before: a row of buttons, a
column of lines, a grid of tiles. Open it with
[[menu.edit > menu.edit.step_and_repeat]] (<kbd>Ctrl+Shift+D</kbd>).

![The Step and Repeat panel](../../img/objects-step-and-repeat.png)

## To make copies {#copies}

1. Select the objects to copy.
2. Set the number of [[docker.copies]] (1 to 999).
3. Under [[docker.horizontal_settings]], choose how the copies are spaced
   across:
   - [[docker.offset]]: the distance from one copy to the next, measured from
     the same point of each;
   - [[docker.spacing]]: the gap between one copy and the next, so the copies
     never overlap whatever their size.

   Then type the distance. 0 keeps the copies in line.
4. Do the same under [[docker.vertical_settings]].
5. Click [[docker.apply]].

The copies are added to the active layer and selected. Distances are in the
drawing units.

The values are for the objects selected when they were typed. Selecting
other objects, or the copies an Apply just made, puts the number of copies,
the distances and the modes back to their defaults, so the next Apply never
reuses a count meant for another object. With nothing selected the fields
wait at their defaults.

To copy with a rotation or a scale at each step, use the [[dialog.copies]]
field of the [[docker.transformations]] panel; see
[Moving, scaling, rotating and skewing](transforming.md#transformations-docker).
