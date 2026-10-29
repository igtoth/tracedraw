# Package

File > Package and the Package choice of Save As put a drawing and what
it needs into one folder, ready to hand to someone else or to keep with
the job (#852). Desktop only: the browser version has no folders, so the
menu item is greyed and Save As does not offer the choice.

## What goes in

| Item | Where | Notes |
|---|---|---|
| The drawing | `<name>.tdraw` | With the fonts it already carries (text pasted from other drawings). |
| Linked pictures | `Pictures/` | Every bitmap linked to a file (an imported picture, Edit Bitmap), in groups, clip frames and symbols too; each file once; two files of one name become `logo.png` and `logo-2.png`. |
| Font files | `Fonts/` | One file per face the text uses: the installed file that draws it (the same lookup text uses: legacy names, a family named with its weight, the regular face for a missing style), or the font the drawing carries. Only fonts whose licence allows them to go with a document (OS/2 `fsType` not restricted and not bitmap only, as for embedding, `tracedraw_text::usable_in_documents`). |
| A PDF (optional) | `<name>.pdf` | Written with the Publish to PDF choices made last (text and fonts, bitmaps, bookmarks) and the drawing's bleed. |
| A report | `<name> report.txt` (named in the interface language) | The drawing, the PDF, each picture copied (from and to), pictures not found, each font copied, fonts left out because of their licence (with their file) and families not installed (with the font drawing their text). |

The copied drawing's links point to the copies as paths relative to the
drawing (`Pictures/logo.png`), so the folder can be moved or sent as a
whole. Relative links are read from the drawing's folder everywhere a
link is read: Update from Link and the watch that follows linked files
(`package::resolve_link`). A picture whose file is gone keeps its link
and is named in the report; its pixels are in the drawing anyway.

## File > Package

The Package Drawing dialog: Folder (a folder named after the drawing,
in the folder of its file, else the last folder used, else the home
folder; Browse picks the parent and keeps the name), File name (the
drawing's name, `.tdraw`), Include a PDF (off). OK writes the package
into the folder, making it when needed and replacing files of the same
names in it; the open drawing is not changed (its links still point to
the original pictures). The status line says how many pictures and fonts
went in and how many items could not.

## Save As > Package

Beside Embed fonts for the TraceDraw type, Package (off) and Include a
PDF. With Package ticked the drawing is saved as
`<folder>/<name>/<name>.tdraw` (the folder is made; an existing file
there asks before it is replaced, as any Save As does), its pictures are
copied beside it and its links are changed to the copies, one undo step
each, before it is written, so the open drawing is the package's from
then on. Fonts, the PDF and the report follow.

## Opening a package

A `.tdraw` drawing opened from a folder that has a `Fonts` folder beside
it gets the font files in that folder for the session, before its text
is laid out (`App::load_package_fonts`), so the package shows its own
fonts where they are not installed. Nothing is installed in the system.

## Checks

- Given a drawing with two linked pictures of one name, one picture
  whose file is gone, an installed font and a family that is not
  installed, when it is packaged with a PDF, then `Pictures/` holds both
  pictures under two names, the copy's links point to them, the open
  drawing is unchanged, the PDF and report are there and the report
  names the missing picture and font
  (`package::tests::a_package_holds_the_drawing_its_pictures_fonts_and_a_report`).
- Given a font whose licence is restricted, it is not copied and the
  report names it
  (`package::tests::fonts_that_may_not_go_along_are_reported_not_copied`).
- Given the Package Drawing dialog, Enter writes the package into the
  folder chosen (`ui::package_dialog::tests`).
- Given Save As with Package ticked in the dialog, the drawing is saved
  in a folder of its name with its picture in `Pictures/`, the link is
  relative and reads back
  (`ui::save_as::tests::save_as_package_puts_the_drawing_in_a_folder_with_its_pictures`).
- `tracedraw_text` finds the file a face came from
  (`tests::the_path_of_a_face_is_the_file_it_came_from`).

Not done: a ZIP of the folder.
