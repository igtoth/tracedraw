# Print merge

Print merge fills a design with data, to make many versions of it: name
badges, numbered tickets, certificates, labels with QR codes. Each record of
your data becomes a copy of the page, filled in.

## Setting up a merge {#setup}

1. Design the page. Where a value goes, type the field name in angle brackets
   in a text object, such as `<Name>`.
2. Choose [[menu.file > menu.file.print_merge > menu.file.print_merge_create]].
3. Add the data:
   - **Import file** reads a CSV file (separated by commas, semicolons or
     tabs, as Excel or any other program saves it) or an Excel workbook
     (`.xlsx`, its first sheet) whose first line names the fields. Fields named like a
     photo or a picture, or whose cells are picture files, start as
     pictures, fields named QR as QR codes; the types show at once to be
     changed;
   - or **Add column** to add fields by hand, then add records with **+**.
4. Click **Finish**. The [[docker.print_merge]] panel opens, where you put
   the fields on the page and make the merge.

![The Print Merge dialog box](../../img/printing-print-merge.png)

## The Print Merge panel {#docker}

The [[docker.print_merge]] panel (also under Window > Panels) works with the
data source you created or loaded:

- **[[merge.create]]** and **[[merge.edit]]** open the records.
- Choose the kind of field, **Text**, **Picture** or **QR code**, and the
  column under **[[merge.columns]]**.
- For a picture, **[[merge.pictures_folder]]** chooses the folder your photos
  are in, when the data names only their files (a name without an extension,
  such as `1024`, finds `1024.jpg` or `1024.png`). **[[merge.placeholder]]** chooses a picture that stands in
  for the merged ones while you design, and **[[merge.align_scale]]** sets
  where each picture goes in its frame: click one of the nine points to
  anchor it, then choose [[merge.scale_actual]], [[merge.scale_fit]],
  [[merge.scale_fill]] (the frame cuts what is outside) or
  [[merge.scale_stretch]].
- **[[merge.insert_field]]** puts the field in the text you are editing, or
  places a new text, picture frame or QR square in the middle of the view.
  Move and size it like any object.
- To put each photo inside a shape of your own (a circle, an oval, any
  outline, also inside a grouped card design), choose Image and the photo
  column, select the shape and click **[[merge.place_inside]]**. Every card's
  photo then goes inside that shape, placed by the alignment and scale.
- Select a field and click **[[merge.update_field]]** to change its column or
  its alignment and scale.
- **[[merge.create_document]]** makes the merged pages in a new drawing: the
  page you are on is copied once per record and each copy is filled in. Your
  design stays as it is. **[[merge.print_document]]** does the same and opens
  Print.

[[menu.file.print_merge_perform]] instead adds the filled copies to the
current drawing, right after the page, named after the page and the record
number.

## Kinds of field {#fields}

- **Text**: replaces `<Field>` in every text of the page.
- **Number**: a counter you do not type: a start, a step and a number of digits
  (start 7, step 3, digits 3 gives 007, 010, 013).
- **Picture**: the cell holds the path of a picture. A text holding only
  `<Field>` becomes the picture, as large as fits its box. Put that text
  inside a frame with [[menu.object.clip_frame_place_inside]] (a photo frame) and the
  picture fills the frame, cut by it. A rectangle, ellipse or curve named
  `<Field>` (in the Objects panel) takes the picture inside it the same way.

Fields work in groups and frames too, so a card can be grouped.
- **QR code**: a text holding only `<Field>` becomes a QR code of the cell's
  text.

An empty cell, or a picture that does not load, removes that object from the
record's page.

## Editing the records {#records}

**Configure columns** renames fields, changes their type and counter settings,
and removes fields. **View single record** shows one record as a form; use the
arrows or **Go to record** to move through them. The placeholder buttons
insert a field into the text you are editing. **Save data source** writes the
records as a CSV file.

To work on several cells at once, drag from one cell to another, or over the
record numbers or the field names to take whole records or fields. Then
**[[merge.sel_add]]** adds as many empty records after them, **[[merge.sel_delete]]**
removes their records and **[[merge.sel_clear]]** (or the Delete key) empties
the cells.

[[menu.file.print_merge_edit]] reopens the records.
