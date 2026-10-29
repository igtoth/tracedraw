# Shelf

A docker that keeps objects apart from any drawing, to place them in
this drawing or another one, in this session or after TraceDraw
restarts. Code: `shelf.rs` (items, file, placing, pictures) and
`ui/shelf_docker.rs` (the docker).

## Opening it

- Window > Dockers > Shelf, the "..." under the docker tabs, or Add to
  Shelf on an object's menu (right click on the selection), which adds
  the selection and opens the docker.

## Items

- Add Selection (enabled with a selection) makes one item of copies of
  the selected objects and chooses it. The status bar says "Added to the
  Shelf: name".
- The name: one object's own name (Object Properties), else the kind the
  Objects docker shows ("Rectangle", "Curve with 5 nodes"); several
  objects are "n objects". Names are one line of at most 120 characters;
  control characters become spaces; an empty name is refused.
- An item stands on its own:
  - every symbol instance in it (inside groups and clip frames too)
    becomes the symbol's objects at the instance's place; instances of a
    missing symbol, symbols nested more than 16 deep and groups left
    empty are left out;
  - the object styles its objects link to go along with it (only those);
  - bitmaps are kept whole (their PNG data is in the item).
- At most 500 items. Adding to a full shelf does nothing and says so.

## Colours

- Add Colours (enabled with a drawing open) makes an item of the
  drawing's document palette and colour styles, named "n colours", shown
  as a grid of its colours (at most 64, eight a row). With neither, the
  status bar says so and nothing is added.
- Placing a colour item (Place, a double click, or dragging it onto the
  drawing) adds its colours to the document palette and its colour
  styles to the drawing's, leaving out colours already there and styles
  whose name is already used; one undo step ("Place from Shelf").

## Placing

- Place (button or the item's menu) and a double click put copies of the
  item's objects on the active layer, centred in the middle of the view.
- Dragging an item onto the drawing puts the copies centred under the
  pointer (in page millimetres, `View::to_page` of the pointer). While
  dragging, a faded picture follows the pointer; over the drawing the
  pointer shows a copy sign.
- The copies get new ids, are selected and make one undo step ("Place
  from Shelf"). A copy keeps its link to an object style only when the
  drawing has a style with the same number and the same contents;
  otherwise it keeps the look and loses the link (as Paste does).
- Nothing is placed while the Welcome Screen shows ("Open or create a
  drawing to place items from the Shelf") or when every layer is locked.

## Arranging

- A click chooses an item (also the right click and the start of a
  drag). The chosen item has the selection colour.
- The item's menu: Place, Rename (the name becomes a field: Enter or
  leaving it keeps the name, Esc keeps the old one), Remove.
- Remove (button or menu) takes the chosen item off the shelf; it cannot
  be undone, so it asks first ("Remove name from the Shelf?", Remove or
  Cancel) unless Options > Global > Warnings, or "Do not ask again" in
  that question, says not to. The status bar says "Removed from the
  Shelf: name".
- Dragging an item onto another item moves it to that place.

## Pictures

- Each item shows a 64 point picture of its objects on white with a grey
  border, the long side filling it, with a margin of 4 % (at least
  0.2 mm) so outlines on the edge show whole. The picture is drawn with
  the drawing renderer at the screen's pixel density, once, and again
  when the density changes. At most six new pictures are drawn per frame,
  so a large shelf fills in over a few frames.

## Where it is kept

- Desktop: `shelf.json` in the preferences folder, rewritten after every
  change through a temporary file (`shelf.json.tmp`, then renamed), so a
  failed write leaves the previous shelf.
- Browser: the page's local storage (`tracedraw-shelf`); browsers limit
  it to a few megabytes, so large bitmaps may not be kept there.
- The file: `{"kind": "tracedraw-shelf", "version": 1, "items": [{"name",
  "shapes", "styles"}]}`; shapes are written as in `.tdraw` files.
- Reading: a missing file, a damaged one, one of another kind or one over
  64 MB gives an empty shelf (a warning in the log); items without
  objects are left out and names cleaned; a file from a newer version is
  read for what this version knows.
- The shelf is read the first time the docker shows or something is
  added, not while TraceDraw starts.

## Checks

- Given two rectangles, when they are added, then the item has both and
  is named "2 objects"; when it is placed at (100, 200), then the copies
  are centred there, selected, and one Undo removes them
  (`selections_go_on_the_shelf_and_come_back_anywhere`).
- Given an item, when it is placed in a new drawing, then it lands in the
  middle of the view; while the Welcome Screen shows, nothing is placed.
- Given a symbol instance and an object linked to a style, when both are
  added, then the item has the symbol's rectangle at the instance's place
  and that style only; placed in the same drawing the link stays, in a
  new drawing it goes (`symbols_become_objects_and_styles_travel_along`).
- Given a symbol that holds an instance of itself, or an instance of a
  missing symbol, then nothing of it is kept
  (`missing_or_self_nested_symbols_are_dropped`).
- Given a saved shelf, when it is read again, then the items and names
  are the same; damaged, foreign, empty and missing files give an empty
  shelf (`the_shelf_is_kept_between_sessions`).
- Names are cleaned and cut to 120 characters; the 501st item is refused;
  a file with more items is cut to 500
  (`names_are_cleaned_and_the_shelf_has_a_limit`).
- Given a 40 x 20 mm red rectangle, then its picture is 64 pixels wide,
  red in the middle (`thumbnails_show_the_objects`).
- The docker draws the pictures, places a dropped item under the pointer,
  moves an item dropped on another one and keeps a name typed with Enter
  (`the_docker_lists_items_and_places_them`).
