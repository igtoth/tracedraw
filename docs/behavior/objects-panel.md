# Objects panel

The Objects tab of the docker shows the current page as a tree: the page
with its layers, topmost first, then the Master Page with the master
layers, the guides and the document grid. A bar at the bottom holds New
Layer and New Master Layer at the left and Delete at the right.

| Row | Contents |
|---|---|
| Page, Master Page | open or close arrow, a sheet (cream for the Master Page), name in bold; a click opens or closes it |
| Layer (light grey), one step in | collapse arrow, eye (show or hide), printer (print and export or not), padlock (lock or unlock), layer icon (its top sheet blue for a page's layers, amber for master layers), name |
| Active layer | as a layer, on `#D4E6F7` with a 3 px accent bar at the left, name in bold |
| Master layer for odd or even pages | as a layer, followed by "Odd pages" or "Even pages" in small grey; dimmed on the pages it does not show on |
| Guides | eye (show or hide the guidelines), a pale printer (guidelines never print), padlock (locks or unlocks all of the page's guidelines; shut when they all are) |
| Document Grid | eye (show or hide the grid), a pale printer and a pale shut padlock (the grid never prints and cannot be edited) |
| Object, two steps in | swatch of its fill (a red diagonal for none) framed in its outline colour, then its name, or what it is when it has none ("Rectangle", "Curve with 8 nodes", "Paragraph Text: ...") |
| Group | as an object, with an arrow that opens it to its objects, one step further in (16 px) |

- Rows are 22 px high. The selected objects show in `#CCE8FF` with a
  `#99D1FF` frame; the row under the pointer in `#E5F3FF`. Hidden
  objects show their name dimmed.
- The active layer is where new objects go (drawn, pasted, duplicated,
  imported). Click a layer to make it active (the selection is cleared).
  Click an object to select it and make its layer active; Shift+click
  adds it to the selection.
- When the selection changes (picked on the page, Select All, an undo),
  the list scrolls just enough to show the first selected object's row,
  if its layer and groups are open; a selection that stays the same
  leaves the scroll where it is (#1044).
- A layer that is hidden or locked, or a master layer that does not show
  on the current page, cannot be active: new objects then go to the
  page's topmost visible, unlocked layer, as they do before any layer is
  picked. Each open drawing remembers its own pick.
- Objects on master layers show on every page the layer is for (all,
  odd or even pages) under the page's own layers, and can be clicked,
  marquee-selected, snapped to and selected with Select All or Tab from
  any of those pages while the layer is visible and unlocked.
- Double-click a layer to rename it; double-click an object to name it
  (Rename Object; an empty name goes back to the description).
- Double-click the page node, or pick Rename in its context menu, to
  rename the page (the Rename Page dialog, as in Layout > Rename Page).
- A layer's context menu: Rename, Move Up, Move Down, Delete Layer (a
  page keeps its last layer); a master layer's adds Show On (All pages,
  Odd pages, Even pages).
- The bottom bar:
  - New Layer adds "Layer n" on top of the page's layers (n is the
    number of layers plus one, or the next number not in use) and makes
    it active.
  - New Master Layer adds "Master Layer n" for all pages and makes it
    active; its context menu adds one for odd or for even pages.
  - Delete removes the selected objects or, with nothing selected, the
    active layer (not the page's last one). It is pale when there is
    nothing it can delete.
- Which layers are closed, which groups are open and whether the page
  and Master Page nodes are closed lasts for the session.

## Checks

- `ui::objects_docker::tests::objects_show_their_names_or_what_they_are`
- `ui::objects_docker::tests::the_docker_draws_layers_groups_and_selection`
- `ui::objects_docker::tests::new_layers_get_free_names_and_become_active`
- `ui::objects_docker::tests::master_layer_objects_are_picked_where_their_layer_shows`
- `ui::objects_docker::tests::delete_takes_the_selection_else_the_active_layer`
- `ui::objects_docker::tests::clicking_an_object_in_the_docker_makes_its_layer_active`
- `ui::objects_docker::tests::the_list_scrolls_to_an_object_selected_on_the_page`
- `command::tests::master_layers_reorder_and_delete_down_to_none` (core)
