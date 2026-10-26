# Print Merge

File > Print Merge > Create/Load Print Merge opens the dialog; Edit Print
Merge reopens the records kept with the session, Perform Merge runs them
again. `print_merge.rs` holds the model and the merge.

## Records

The dialog: "Add columns or records manually or import data from a file"
with Import File; under "Add or edit records" Add Column, Configure
Columns and View Single Record; the records; + and - (add a record, delete
the current one) and the navigation (first, previous, next, last, Go to
Record); Save Data Source, Finish and Cancel.

- Import File reads a CSV whose first line names the fields (comma,
  semicolon or tab, whichever the first line has most of; UTF-8 with or
  without its byte order mark, UTF-16 with its mark, else code page 1252,
  as Excel saves CSV on Windows: that one failed to read as UTF-8, #495,
  `export::tests::data_files_in_every_encoding_read`), or an Excel workbook
  (`.xlsx`): its first sheet, shared, inline and formula strings, numbers
  as plain text (whole numbers without a decimal point), TRUE and FALSE,
  empty rows and trailing empty cells left out (`tracedraw_io::xlsx`,
  #377). The field types are guessed: a name holding photo, picture,
  image, foto, imagem, img or logo, or cells that all name picture files,
  gives Picture; a name holding QR gives QR code; the rest Text. Configure
  Columns opens after an import to check them
  (`field_types_are_guessed_from_the_data`). Save Data Source
  writes the records back as CSV, counters written out.
- Add Column asks for a name and a type and adds the field to every
  record (and a first record when there is none). For a Number it also
  asks for the counter: digits (zeros in front), the starting value,
  "Continually increment value" with its step (off: every record the
  same number) and, when ticked, an ending value. With an ending value
  the records from the start to it are made at once (15 to 25 in three
  digits: eleven records, 015 to 025), counting down when the step is
  negative; records already there are kept, and at most 100,000 are
  made. An end the counter never reaches makes none (#791,
  `print_merge::tests::a_number_column_with_an_end_value_makes_its_records`,
  `pick_drag_tests::add_column_offers_a_number_range`).
- Configure Columns shows each field's name (edited in place), type,
  counter settings and the x that removes it.
- View Single Record shows the current record as a form; View All
  Records goes back to the grid, where a click on a record's number makes
  it the current one.
- The grid selects a block of cells (#501): a drag from one cell to
  another (the text editing stops once the drag reaches another cell), a
  drag over record numbers (whole records) or field names (whole fields),
  a click on one of those, or Shift+click on a cell from the block's first
  cell; a block of more than one cell is tinted. Beside + and -: Add puts
  as many empty records as the block spans after it (and selects them),
  Delete removes the block's records, Clear (or Delete with no field
  being edited) empties its cells, counter fields keeping their numbers
  (`ui::dialogs::tests::the_merge_grid_selects_a_block_of_cells_by_dragging`,
  `print_merge::tests::clearing_cells_leaves_counters`).
- Types:
  - Text: put in place of `<Field>` in every text object of the page.
  - Number: a counter, not typed in: Start, then Step more for each
    record, padded with zeros to Digits digits (start 7, step 3, digits 3
    gives 007, 010, 013).
  - Picture: the cell is the path of a picture (relative paths start at
    the data file's folder). A text object holding only `<Field>` becomes
    the picture, as large as fits in the object's box (the frame of
    paragraph text) keeping its proportions, rotated with it.
  - QR code: a text object holding only `<Field>` becomes a black QR
    code of the cell's text, square, as large as the box allows.
  An empty cell, or a picture that does not load, removes the field's
  object from that record's page.
- Fields are found in groups and in clip frames too. A picture
  placeholder inside a clip frame becomes the picture covering the
  frame's box (as small as covers it, centred, cut by the frame); a
  rectangle, ellipse or curve named `<Field>` of a picture field (on the
  page, not in a group) takes the picture inside it, covering it, as a
  clip frame (#378, `fields_in_groups_and_frames_are_filled`). The page
  copies get new ids all through, clip frames included, so filling a
  copy never changes the template.
- The placeholder buttons insert `<Field>` into the text being edited.

## The Print Merge docker

Finish keeps the records and opens the Print Merge docker (also Window >
Panels > Print Merge), as in the reference program; it no longer merges
at once (#437). From top to bottom:

- Create (greyed while records are loaded) opens the dialog on new
  records, Edit on the ones kept.
- Text, Image or QR code: the kind of field Insert Field makes.
- Data source columns: the field to insert.
- For Image: Pictures folder (#460): the folder picture cells that are
  relative paths are looked for in first, before the data file's folder
  (the folder button picks it, the cross forgets it); a cell naming a
  picture without an extension ("1024") finds it with .jpg, .jpeg, .png,
  .webp, .bmp, .gif, .tif or .tiff
  (`each_record_gets_its_photo_from_the_pictures_folder`). Then
  Placeholder image (a picture that stands in for the merged
  ones on the page; the folder button picks it) and Align and scale: a
  grid of nine anchor points (the point of the placeholder the picture
  is placed by) and Actual size (its own size: its resolution, else
  96 dpi), Fit proportionally (the default), Fill proportionally (covers
  the placeholder, cut by it) or Stretch to fill.
- Insert Field: Text puts `<Field>` at the caret of the text being
  edited, else makes a text holding it in the middle of the view. Image
  makes a frame named `<Field>` in the middle of the view, 40 mm square,
  or the placeholder picture's shape at most 80 mm long with the picture
  inside it as a clip frame; it keeps the Align and scale in its object
  data (`PrintMerge.Scale`: actual, fit, fill or stretch;
  `PrintMerge.Anchor`: 0 top left to 8 bottom right, row by row). QR
  code makes a 40 mm square named `<Field>`. One undo step each.
- Place Inside Frame (Image only): each
  selected object that is not a text or a group (a circle, an oval, any
  outline, a clip frame), on the page or inside a group, is named
  `<Field>` of the chosen column and keeps the Align and scale shown; on
  the page, with a placeholder picture chosen, the picture goes inside
  it. One undo step "Place Inside Frame". Nothing selected: the button
  waits for a click on the shape (it shows pressed, the status bar asks
  for the shape and the pointer is the Place Inside arrow, as for Place
  Inside Frame on the Object menu); the click takes the object under it,
  also one inside a group, and makes it the frame; Esc, a right click or
  a click on nothing cancels
  (`print_merge_docker::tests::place_inside_frame_uses_the_users_shapes_also_in_a_group`,
  `pick_drag_tests::place_inside_frame_waits_for_a_click_with_its_arrow`,
  #460, #577).
- On every page (for every kind of field, Text, Image and QR code, off
  at first): Insert Field puts the field in the same place on every
  other page of the drawing too, a copy with ids of its own in each
  page's first shown, unlocked layer, joining the undo step of the
  insert; a field typed into text being edited goes in that text only
  (#793,
  `print_merge_docker::tests::on_every_page_a_text_field_goes_on_every_page_and_merges_there`).
  For a picture field Place Inside Frame also puts the placeholder picture in the same place on
  every other page of the drawing, as part of the same undo step: in the
  shape there of the same kind and bounds (within 0.01 mm), which takes
  the field's name and placement, else in a copy of the frame added to
  that page's first unlocked layer. Only frames on the page itself (not
  inside a group) are repeated (#598,
  `print_merge_docker::tests::the_placeholder_goes_in_the_same_place_on_every_page`).
  Without a placeholder picture and with a pictures folder, each page
  takes its own picture of the folder instead: the folder's pictures in
  name order (ignoring case, numbers by value: 2 before 10), the first
  on the drawing's first page; pages past the last picture are left as
  they are (#618,
  `print_merge_docker::tests::on_every_page_takes_the_pictures_folder_page_by_page`).
  With it on, Create Merged Document and Print Merged Document fill the
  open drawing's own pages instead of making a new drawing: record 1 on
  the first page, record 2 on the second, and so on, as one undo step
  "Print Merge"; pages past the last record keep their fields, records
  past the last page are left out, and a picture frame whose cell is
  empty or whose picture does not load keeps the picture it holds (#618,
  `print_merge_docker::tests::on_every_page_merges_onto_the_drawings_own_pages`).
  So a drawing already merged, whose pictures folder is gone, is left as
  it is. The status bar says "Print merge filled {n} page(s) of this
  drawing" (and how many records had no page left), "Pages already
  filled; no new drawing made" when nothing was left to fill, or "These
  pages hold no merge fields; nothing to fill" when they have none, with
  no undo step (#618,
  `print_merge_docker::tests::on_every_page_a_merge_keeps_the_pictures_already_placed`).
  Place Inside Frame with On every page leaves a frame on another page
  that already holds something as it is, as on the page itself.
- Update Field (when a placeholder is selected: a text holding only
  `<Field>`, or an object named `<Field>`): the selected placeholders
  take the chosen column, and with Image the Align and scale shown.
- Create Merged Document: the current page merged into a new drawing
  (one page per record, named as below); the drawing it came from stays
  as it was. With On every page on, the drawing's own pages are filled
  instead (above). Print Merged Document... does the same, then opens Print.

Clip frames keep the object data of the object they were made from, as
they keep its name.

## Merge

File > Print Merge > Perform Merge copies the current page once per
record, right after it, in the records' order, names the copies
"<page> 1", "<page> 2", ... and fills them. It is two undo steps (the
copies, then the fills).

An object named `<Field>` of a picture field takes the record's picture
inside it as a clip frame, placed by its Align and scale (placeholders
without it fill it from the centre, as before); a clip frame's old
contents, such as the placeholder picture, give way. An empty cell or a
picture that does not load leaves the frame empty, so a record without a
picture never shows the stand-in; only a merge onto the drawing's own
pages (On every page) keeps the frame's current contents then (#618,
`print_merge_docker::tests::a_record_without_a_picture_gets_no_stand_in`). On the page the
picture is placed inside the object; inside a group the object becomes,
in place, a clip frame of its own outline holding the picture (#460). An object named `<Field>` of a QR field becomes the code, as
large as its box allows.

## Checks

- `print_merge::tests::counters_and_short_records_are_filled_in`
- `print_merge::tests::fields_and_records_are_added_and_removed`
- `print_merge::tests::csv_round_trips_with_counters_written_out`
- `print_merge::tests::merge_fills_text_qr_and_counters`
- `print_merge::tests::pictures_are_placed_by_scale_and_anchor`
- `ui::print_merge_docker::tests::the_docker_inserts_fields_and_makes_a_merged_document`
- `ui::print_merge_docker::tests::update_field_changes_the_column_and_the_placement`
- `ui::print_merge_docker::tests::the_docker_draws_with_and_without_data`
- `ui::print_merge_docker::tests::on_every_page_takes_the_pictures_folder_page_by_page`
- `ui::print_merge_docker::tests::on_every_page_merges_onto_the_drawings_own_pages`
