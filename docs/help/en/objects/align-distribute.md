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
   [[docker.fit_to_path_even]] to spread the objects over the whole path, or
   [[docker.fit_to_path_distance]] and type the [[docker.fit_to_path_distance_value]]
   between the centers of neighbouring objects.
3. [[docker.fit_to_path_offset]] moves the first object along the path from
   its start.
4. Tick [[docker.fit_to_path_rotate]] to turn each object with the direction
   of the path, and [[docker.fit_to_path_reverse]] to start from the other
   end.
5. The outlines on the canvas show where the objects will go. Click
   [[docker.apply]].

On a closed shape the objects go all the way round without the first and
last meeting. Fitting is one undo step.
