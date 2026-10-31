# Objects panel

The Objects tab of the docker shows the drawing as a tree: every page
with its layers, topmost first (or only the current page), then the
Master Page with the master layers, the guides and the document grid
(#1477). A bar at the bottom holds New Layer, New Master Layer and Show
All Pages at the left, the thumbnail size in the middle and Delete at
the right.

| Row | Contents |
|---|---|
| Page, Master Page | open or close arrow, a sheet (cream for the Master Page), name (in bold for the current page and the Master Page); a click opens or closes it |
| Layer (light grey), one step in | collapse arrow, eye (show or hide), printer (print and export or not), padlock (lock or unlock), layer icon (its top sheet blue for a page's layers, amber for master layers), name |
| Active layer | as a layer, on `#D4E6F7` with a 3 px accent bar at the left, name in bold |
| Master layer for odd or even pages | as a layer, followed by "Odd pages" or "Even pages" in small grey; dimmed on the pages it does not show on |
| Master layer in front of the page's layers | as a layer, followed by "In front" in small grey (after "Odd pages" or "Even pages" if it has one); it stays listed under the Master Page (#1132) |
| Guides | eye (show or hide the guidelines), a pale printer (guidelines never print), padlock (locks or unlocks all of the page's guidelines; shut when they all are) |
| Document Grid | eye (show or hide the grid), a pale printer and a pale shut padlock (the grid never prints and cannot be edited) |
| Object, two steps in | thumbnail: the object itself drawn small (its shape, fill, outline and effects) on a white tile, then its name, or what it is when it has none ("Rectangle", "Curve with 8 nodes", "Paragraph Text: ...") |
| Group | as an object, with an arrow that opens it to its objects, one step further in (16 px) |

- Layer and node rows are 22 px high; an object row is the thumbnail
  size plus 6 px, 22 px at least. The selected objects show in `#CCE8FF`
  with a `#99D1FF` frame; the row under the pointer in `#E5F3FF`. Hidden
  objects show their name and their thumbnail dimmed.
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
  odd or even pages) under the page's own layers, or over them when the
  layer is in front (its menu's "In front of the page's layers", or To
  Front of Page on one of its objects; To Back of Page puts it behind
  again, one undo step; #1132), and can be clicked,
  marquee-selected, snapped to and selected with Select All or Tab from
  any of those pages while the layer is visible and unlocked.
- Double-click a layer to rename it; double-click an object to name it
  (Rename Object; an empty name goes back to the description).
- Double-click the page node, or pick Rename in its context menu, to
  rename the page (the Rename Page dialog, as in Layout > Rename Page);
  the context menu of another page also has Go to Page.
- A click on a layer or an object of another page makes that page the
  current one first (Layout's page change), then picks the layer or
  selects the object.
- A layer's context menu: Rename; New Layer (on its page) or New Master
  Layer (on the same pages); Select the Layer's Objects (the ones shown
  and not locked); ticks for Show/hide, Print and export the layer and
  Lock layer; Move Up, Move Down; a page's layer has Move to Master Page
  (all pages): the layer becomes a master layer with its name, look and
  objects (a page keeps its last layer, emptied); a master layer has
  Show On (All pages, Odd pages, Even pages), In front of the page's
  layers (a tick) and Move to the Current Page: it becomes the page's
  top layer with its objects, which then show on that page only; then
  Delete Layer. Each is one undo step (#1477).
- An object's context menu: Rename; Move to Layer, a list of every
  page's layers and the master layers under their page's name: the
  object (with the rest of the selection when it is selected) goes to
  the top of that layer, keeping its place on the page, in one undo
  step, and stays selected; a layer of another page makes that page the
  current one. So an object of a master layer is moved onto one page's
  layer, or a page's object onto the master page (#1477). Delete.
- The bottom bar:
  - New Layer adds "Layer n" on top of the page's layers (n is the
    number of layers plus one, or the next number not in use) and makes
    it active.
  - New Master Layer adds "Master Layer n" for all pages and makes it
    active; its context menu adds one for odd or for even pages, and
    under "In front of the page's layers" has a tick for each kind (All
    pages, Odd pages, Even pages): ticking it puts the drawing's master
    layers of that kind in front of the page's own layers (one undo
    step), unticking puts them behind, and new master layers of that
    kind are made in front or behind accordingly. The ticks are kept in
    the preferences (`master_in_front`, off at first) and are also in
    Options > Page, beside New master (all, odd, even), where each
    master layer also has an In front tick (#1132, #1477).
  - Show All Pages (two sheets, shown pressed when on, the default)
    lists every page; off, only the current one
    (`objects_all_pages` in the preferences).
  - Delete removes the selected objects or, with nothing selected, the
    active layer (not the page's last one). It is pale when there is
    nothing it can delete.
- Which layers are closed, which groups are open and which page nodes
  and the Master Page node are closed lasts for the session.

## Thumbnails

- Each object row starts with a thumbnail: the object drawn alone by the
  same renderer as the page, fitted into a square tile with a small
  margin and kept in its proportions (a long thin object gets a strip,
  at least an eighth of the tile thick). A group shows all of its
  objects, a symbol instance its symbol, a hidden object what it would
  draw. An object that draws nothing (an empty group) shows the empty
  tile.
- The size goes from 16 to 96 points in steps of 4, 20 at first, and is
  kept in the preferences (`object_thumb_size`). It changes with the
  slider in the bottom bar (between a small and a large tile, shown when
  the docker is wide enough), the wheel over that slider, or
  Ctrl+wheel (Cmd+wheel on a Mac) anywhere over the list; each notch is
  one step.
- Thumbnails are drawn at the screen's pixel density and cached per
  drawing and object. A cached thumbnail is checked only when the
  drawing changed (its revision moved on), and drawn again only when the
  object itself changed (compared with the copy it was drawn from), a
  symbol it shows changed, or the size changed. Rows scrolled out of
  sight draw nothing and ask for no thumbnail.
- New thumbnails are drawn for up to 6 ms per frame (at least one); the
  rest wait for the next frames, showing their old picture or, the first
  time, the swatch of their fill (a red diagonal for none) framed in
  their outline colour.
- The cache keeps at most 24 MB of pixels and 4,000 thumbnails; past
  that the thumbnails shown longest ago are dropped (never one shown in
  the current frame) and drawn again if their rows come back into view.
  With 3,000 objects the docker keeps only the thumbnails of the rows on
  screen and of those scrolled past.
- Measured with `objects_docker_frame_times` (3,000 objects, a 600 px
  docker, debug build): a frame with nothing changed takes 2.1 ms and a
  scrolling frame 3.0 ms, against 55 ms and 56 ms before thumbnails,
  when every row was drawn whether in sight or not; after scrolling the
  whole list the cache held 191 thumbnails in 191 KB of pixels.

## Checks

- `ui::objects_docker::tests::objects_show_their_names_or_what_they_are`
- `ui::objects_docker::tests::the_docker_draws_layers_groups_and_selection`
- `ui::objects_docker::tests::new_layers_get_free_names_and_become_active`
- `ui::objects_docker::tests::master_layer_objects_are_picked_where_their_layer_shows`
- `ui::objects_docker::tests::delete_takes_the_selection_else_the_active_layer`
- `ui::objects_docker::tests::clicking_an_object_in_the_docker_makes_its_layer_active`
- `ui::objects_docker::tests::the_list_scrolls_to_an_object_selected_on_the_page`
- `ui::objects_docker::tests::rows_show_thumbnails_drawn_once_per_change`
- `ui::objects_docker::tests::ctrl_wheel_and_the_bar_change_the_thumbnail_size`
- `ui::objects_docker::tests::a_long_list_draws_only_the_rows_in_sight` (and the
  ignored benchmark `objects_docker_frame_times`)
- `object_thumbs::tests::a_thumbnail_shows_the_object_in_its_colours`
- `object_thumbs::tests::pictures_are_drawn_again_only_when_the_object_changes`
- `object_thumbs::tests::the_least_recently_shown_pictures_go_first`
- `object_thumbs::tests::sizes_are_clamped`
- `command::tests::master_layers_reorder_and_delete_down_to_none` (core)
