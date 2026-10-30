# Fitting objects to a path

The [[docker.fit_to_path]] panel places objects along a line, a curve or the
outline of a shape: beads on a string, letters or stars round a seal, a row
of fence posts along a road, dashes of your own along a border. It can make
copies to fill the path, space them evenly or at a set distance, stand them
on the path or turn them with it.

## Placing objects on a path {#place}

1. Select the objects to place, then the path last (Shift+click). The panel
   shows how many objects are selected and what kind of object the path is.
2. Choose [[menu.object > menu.object.fit_objects_to_path]] to open the
   panel.
3. Change the values you need (see below). Once a value changes, dashed
   outlines on the canvas show where every object and copy will go.
4. Click [[docker.apply]].

Fitting undoes in one step. Copies go into the layer of the object they copy,
with its fill, outline and effects. After an Apply that made copies, the
objects placed and the copies are selected (not the path).

The panel starts from the default values, with no outlines, each time you
select other objects or work in another drawing; the values you enter stay
while the same objects are selected.

## The settings {#settings}

The panel's groups, from top to bottom:

### Objects and path {#objects}

- [[docker.fit_to_path_keep_originals]] (off): the selected objects stay
  where they are and copies of them are placed.
- [[docker.fit_to_path_group]] (off): what was placed goes into one group,
  which is selected.
- [[docker.fit_to_path_contiguous]] (on): a path made of several pieces is
  walked piece after piece, as one path. Off, only the first piece is used.

### Spacing {#spacing}

- [[docker.fit_to_path_spacing]]: [[docker.fit_to_path_even]] (the default)
  spreads the objects over the whole path; [[docker.fit_to_path_distance]]
  puts them the [[docker.fit_to_path_distance_value]] apart (10 mm at first).
- [[docker.fit_to_path_measure]]: between the
  [[docker.fit_to_path_centres]] of neighbouring objects (the default) or
  between their [[docker.fit_to_path_edges]], so objects of different sizes
  get the same gap.
- [[docker.fit_to_path_offset]] (0): moves the first object along the path
  from its start. A negative value is allowed.
- [[docker.fit_to_path_end_offset]] (0, open paths only): keeps the objects
  away from the end of the path.
- [[docker.fit_to_path_align]] (fixed distance only): puts the row at the
  [[docker.fit_to_path_align_start]] (the default), in the
  [[docker.fit_to_path_align_middle]] or at the
  [[docker.fit_to_path_align_end]] of the path.

### Copies {#copies}

- [[docker.fit_to_path_copies_none]] (the default): each selected object is
  placed once.
- [[docker.fit_to_path_copies_count]] (1 to 1000, 10 at first): that many
  objects in all: the selected ones, then copies of them in the same order.
- [[docker.fit_to_path_copies_fill]] (fixed distance only): as many as fit
  on the path.

There are never more than 1000 objects in all.

### Placement {#placement}

- [[docker.fit_to_path_place]]: [[docker.fit_to_path_on]] (the default),
  [[docker.fit_to_path_above]] or [[docker.fit_to_path_below]], with a
  [[docker.fit_to_path_side]] (0 at first) between the objects and the path.
  "Above" is on the left of the path's direction.
- [[docker.fit_to_path_reference]]: the point of each object that goes on
  the path: its [[docker.fit_to_path_ref_pivot]] (the default; the centre,
  or where you moved it with the Pick tool), or a point of its
  [[docker.fit_to_path_ref_box]] or [[docker.fit_to_path_ref_outline]].
- [[docker.fit_to_path_origin]] (box references only): one of nine points of
  the box, picked in the grid (the centre at first). The bottom middle point
  makes the objects stand on the path.

### Rotation {#rotation}

- [[docker.fit_to_path_rotation]]: [[docker.fit_to_path_rotate]] turns each
  object with the direction of the path; [[docker.fit_to_path_fixed]] (the
  default) turns them all alike. The [[docker.fit_to_path_angle]] (0, in
  degrees counterclockwise) is added in either case.
- [[docker.fit_to_path_ignore_rotation]] (off): straightens objects that
  were turned before they are placed.
- Under [[docker.fit_to_path_modify_rotation]], the
  [[docker.fit_to_path_spin]] adds a turn to every object:
  - [[docker.fit_to_path_spin_none]] (the default): no extra turn.
  - [[docker.fit_to_path_spin_uniform]]: every object turns by the
    [[docker.fit_to_path_start_angle]].
  - [[docker.fit_to_path_spin_progressive]]: each object turns a little more
    than the one before, the [[docker.fit_to_path_spin_angle]] (360 at
    first) times the [[docker.fit_to_path_revolutions]] (1) over the whole
    row.
  - [[docker.fit_to_path_spin_jitter]]: each object turns by a different
    amount, up to the [[docker.fit_to_path_range]] (15 degrees) either way.
    The same object turns the same way every time, in the preview and after
    Apply.
  - [[docker.fit_to_path_spin_progressive_jitter]]: both.

  Tick [[docker.fit_to_path_clockwise]] to turn the other way. Only the
  fields the style uses can be changed.

### Order {#order}

- [[docker.fit_to_path_order]]: which object goes first:
  [[docker.fit_to_path_order_selected]] (the default),
  [[docker.fit_to_path_order_reversed]], [[docker.fit_to_path_order_ltr]],
  [[docker.fit_to_path_order_rtl]], [[docker.fit_to_path_order_largest]],
  [[docker.fit_to_path_order_smallest]],
  [[docker.fit_to_path_order_alternate]], [[docker.fit_to_path_order_ttb]],
  [[docker.fit_to_path_order_btt]], [[docker.fit_to_path_order_tall]],
  [[docker.fit_to_path_order_short]], [[docker.fit_to_path_order_wide]],
  [[docker.fit_to_path_order_narrow]] or [[docker.fit_to_path_order_random]]
  (shuffled the same way each time for the same objects).
- [[docker.fit_to_path_reverse]] (off): starts from the other end of the
  path.

## Tips and limits {#tips}

- On a closed shape the objects go all the way round, the closing side
  included, without the first and last meeting.
- On an open path, an object that would land past an end stays at that end.
- With more than 200 objects each one previews as its box; with more than
  200 selected there is no preview, but Apply works.
- To place text along a path instead, see
  [Adding and editing text](../text/adding-text.md). To spread objects in a
  straight row, see [Aligning and distributing](align-distribute.md).
