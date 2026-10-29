# Aligning and distributing objects

Aligning lines objects up by their edges or centers; distributing spaces them
evenly. Use the [[docker.align_distribute]] panel
(<kbd>Ctrl+Shift+A</kbd>), the commands of
[[menu.object > menu.object.align_distribute]], or the keys below.

![The Align and Distribute panel](../../img/objects-align-distribute.png)

## Aligning {#align}

1. Select the objects. Select the one the others should line up with last.
2. In the [[docker.align]] part of the panel, choose [[docker.align_to]]:
   - [[docker.align_active]] (at first): the object selected last; with one
     object selected, the page;
   - [[docker.align_page_edge]]: the edges of the page;
   - [[docker.align_page_center]]: the center of the page;
   - [[docker.align_grid]]: each object to the nearest line of the document
     grid.
3. Click one of the six buttons: left edges, centers on a vertical line,
   right edges, top edges, centers on a horizontal line, or bottom edges.

[[docker.align_use_outline]]: with this on, the outer edges of the outlines
meet, instead of the paths themselves.

The keys always align to the object selected last:

| Key | Command |
|---|---|
| <kbd>L</kbd> | [[menu.object.align_left]] |
| <kbd>R</kbd> | [[menu.object.align_right]] |
| <kbd>T</kbd> | [[menu.object.align_top]] |
| <kbd>B</kbd> | [[menu.object.align_bottom]] |
| <kbd>E</kbd> | [[menu.object.align_centers_h]] |
| <kbd>C</kbd> | [[menu.object.align_centers_v]] |
| <kbd>P</kbd> | [[menu.object.center_to_page]] |

## Distributing {#distribute}

1. Select three or more objects (two or more to distribute across the page).
2. In the [[docker.distribute]] part, choose [[docker.distribute_to]]:
   - [[docker.distribute_selection]] (at first): the first and last objects
     stay where they are, and the others move between them;
   - [[docker.distribute_page]]: the objects spread across the whole page,
     the first touching one side and the last the other.
3. Click one of the eight buttons: left edges, centers, gaps or right edges
   across the page, or top edges, centers, gaps or bottom edges down it.
   Spreading the gaps gives equal space between the objects whatever their
   sizes.

With too few objects the buttons are pale and the panel says how many are
needed. Each align or distribute is one undo step.

To place copies at even distances in one step, see
[Step and repeat](step-and-repeat.md).

## Fit objects to a path {#fit-to-path}

The [[docker.fit_to_path]] panel places objects along a line, a curve or the
outline of a shape. Open it with
[[menu.object > menu.object.fit_objects_to_path]].

1. Select the objects to place, then the path last (Shift+click).
2. Under [[docker.fit_to_path_spacing]], choose
   [[docker.fit_to_path_even]] to spread the objects over the path, or
   [[docker.fit_to_path_distance]] and type the [[docker.fit_to_path_distance_value]]
   from one object to the next. [[docker.fit_to_path_measure]] measures it
   between the [[docker.fit_to_path_centres]] of neighbouring objects, or
   between their [[docker.fit_to_path_edges]], so that objects of different
   sizes get the same gap.
3. [[docker.fit_to_path_offset]] moves the first object along the path from
   its start; [[docker.fit_to_path_end_offset]] keeps the objects away from
   the end of an open path. With a fixed distance,
   [[docker.fit_to_path_align]] puts the row at the
   [[docker.fit_to_path_align_start]], in the
   [[docker.fit_to_path_align_middle]] or at the
   [[docker.fit_to_path_align_end]].
4. Under [[docker.fit_to_path_copies]], [[docker.fit_to_path_copies_count]]
   fills the path with that many objects: the ones you selected, then
   copies of them in turn. With a fixed distance,
   [[docker.fit_to_path_copies_fill]] makes as many copies as fit.
5. Under [[docker.fit_to_path_place]], put the objects
   [[docker.fit_to_path_on]], [[docker.fit_to_path_above]] or
   [[docker.fit_to_path_below]], with a [[docker.fit_to_path_side]].
6. Under [[docker.fit_to_path_rotation]], choose
   [[docker.fit_to_path_rotate]] to turn each object with the direction of
   the path, or [[docker.fit_to_path_fixed]] to turn them all alike; the
   [[docker.fit_to_path_angle]] is added in either case.
7. [[docker.fit_to_path_order]] sets which object goes first: as selected,
   reversed, from left to right or right to left, by size, or large and
   small in turn. [[docker.fit_to_path_reverse]] starts from the other end
   of the path.
8. Once you change a value, the outlines on the canvas show where the
   objects and copies will go. Click [[docker.apply]]. Copies go into
   the layer of the object they copy, and the objects placed are selected.

More choices on the panel:

- [[docker.fit_to_path_keep_originals]] leaves the selected objects where
  they are and places copies; [[docker.fit_to_path_group]] puts what was
  placed into one group.
- [[docker.fit_to_path_contiguous]] walks every part of a path made of
  several pieces, one after another; when it is off only the first piece
  is used.
- [[docker.fit_to_path_reference]] chooses the point of each object that
  goes on the path: its [[docker.fit_to_path_ref_pivot]] (moved with the
  Pick tool, if you moved it), or a point of its
  [[docker.fit_to_path_ref_box]] or [[docker.fit_to_path_ref_outline]],
  picked in the [[docker.fit_to_path_origin]] grid. The bottom middle point
  makes the objects stand on the path.
- [[docker.fit_to_path_ignore_rotation]] straightens objects that were
  turned before they are placed.
- Under [[docker.fit_to_path_modify_rotation]], a
  [[docker.fit_to_path_spin]] adds a turn to every object:
  [[docker.fit_to_path_spin_uniform]] turns them all by the
  [[docker.fit_to_path_start_angle]], [[docker.fit_to_path_spin_progressive]]
  turns each a little more than the one before (the
  [[docker.fit_to_path_spin_angle]] times the
  [[docker.fit_to_path_revolutions]] over the whole row),
  [[docker.fit_to_path_spin_jitter]] turns each by a different amount up to
  the [[docker.fit_to_path_range]], and
  [[docker.fit_to_path_spin_progressive_jitter]] does both. Tick
  [[docker.fit_to_path_clockwise]] to turn the other way. Only the fields
  the style uses can be changed.

The panel starts from its default values, with no outlines, each time you
select other objects (or work in another drawing); the values you enter
stay while the same objects are selected. On a closed shape the objects go all the way round without the first and
last meeting. Fitting is one undo step.
