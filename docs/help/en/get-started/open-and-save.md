# Opening and saving drawings

TraceDraw saves drawings in its own format, `.tdraw`, which keeps
everything, and can also save `.cdr` files. It opens many other formats; see
[Supported file formats](../file-formats/supported-formats.md).

## Opening a drawing {#open}

1. Choose [[menu.file > menu.file.open]] (<kbd>Ctrl+O</kbd>), or click Open
   on the standard toolbar.
2. Choose the file and confirm.

The drawing opens in a new tab. To open a drawing you used recently, use
[[menu.file > menu.file.open_recent]], the arrow of the Open button, or the
list on the Welcome Screen; [[menu.file.clear_recent]] empties the list. To
take one drawing off the Welcome Screen list, point at it and click the x at
its right, or right-click it and choose [[welcome.remove_recent]]; the file
itself stays where it is.
[[menu.file > menu.file.import]] (<kbd>Ctrl+I</kbd>) instead adds the
content of a file to the drawing that is open; see
[Importing files](../file-formats/import.md).

### Notes after opening {#open-notes}

When some objects of an opened file could not be read, or the drawing uses
fonts that are not installed on your computer, a message says so after the
file opens:

- For objects that could not be read, it gives their number. A
  **Report** button opens a new issue for the file on the project's issue
  tracker, so the problem can be fixed.
- For missing fonts, it names them. The text shows in a similar font and
  keeps its font name, so it comes back as it was on a computer that has
  the font.

Tick **Do not show this message again** to stop these messages. You can
turn them on again on the [[options.warnings]] page of
[[menu.tools > menu.tools.options > menu.tools.options_global]].

## Saving a drawing {#save}

Choose [[menu.file > menu.file.save]] (<kbd>Ctrl+S</kbd>) to write the
drawing back to its file. A new drawing, or one opened from a format
TraceDraw only reads, has no file yet, so Save opens the
[[dialog.save_drawing]] dialog box.

## Save As {#save-as}

[[menu.file > menu.file.save_as]] (<kbd>Ctrl+Shift+S</kbd>) saves the drawing
under another name, in another folder or in another format.

![The Save Drawing dialog box](../../img/get-started-open-and-save-save-as.png)

To save a drawing under a new name:

1. Choose the folder. The list on the left has shortcuts to
   [[dialog.place_home]], [[dialog.place_desktop]],
   [[dialog.place_documents]], [[dialog.place_downloads]] and
   [[dialog.place_pictures]], and to the drives. [[dialog.up_folder]] goes
   to the folder above; you can also type a folder in the path bar and press
   <kbd>Enter</kbd>. [[dialog.new_folder]] makes a folder and opens it.
   Double-click a folder of the list to open it; click a file to take its
   name.
2. Type the [[dialog.file_name]]. The extension of the chosen type is added.
3. Choose [[dialog.save_as_type]]: TraceDraw (`*.tdraw`) or
   [[file.cdr_files]].
4. Choose the [[dialog.cdr_version]]. For TraceDraw files it is the version
   of the format, newest first. For `.cdr` files it is the version of the
   file, from 27 down to 14; an older program can only open a version as old
   as itself or older. The version you choose stays with the drawing, so
   Save writes it again without asking.
5. For `.cdr` files from version 16, [[dialog.embed_color_profile_none]]
   stores the drawing's CMYK profile in the file, and
   [[dialog.embed_fonts]] stores the fonts the text uses when their license
   allows it. Both choices are kept for the next time.
6. Optionally fill in [[dialog.title]], [[dialog.subject]] and a
   [[dialog.rating]] (click the lit last star again to take it off). They
   are the drawing's properties, also shown in
   [Document properties](../pages-layout/document-properties.md).
7. Click [[dialog.save]] or press <kbd>Enter</kbd>. If the file exists,
   TraceDraw asks whether to [[dialog.replace]] it.

The dialog box starts in the drawing's folder, else in the folder you saved
to last, else in your documents folder.

> **Note:** In the browser version every save is a download: the dialog box
> keeps the name, type, version and properties, and the browser puts the file
> in its downloads folder.

Other ways to save: [[menu.file > menu.file.save_as_template]] saves a
template (see [Templates](../templates-styles/templates.md)),
[[menu.file.export]] writes other formats (see
[Exporting](../file-formats/export.md)), [[menu.file.publish_to_pdf]] writes
a PDF (see [Publishing to PDF](../file-formats/publish-pdf.md)),
[[menu.file.send_to]] saves a PDF to your desktop or documents folder or
attaches it to an email, and [[menu.file.github]] keeps drawings in a GitHub
repository (see [GitHub](github.md)). [[menu.file.revert]] goes back to the
version last saved.

## Closing drawings {#closing}

Choose [[menu.file > menu.file.close]] (<kbd>Ctrl+F4</kbd>), click the close
button on the drawing's tab, or middle-click the tab.
[[menu.file.close_all]] closes every drawing, and [[menu.file.exit]]
(<kbd>Alt+F4</kbd>) closes TraceDraw.

When a drawing has unsaved changes, TraceDraw asks whether to save them
first: [[dialog.yes]] saves (asking for a name if the drawing has no file yet),
[[dialog.no]] closes without saving, and [[dialog.cancel]] keeps the drawing open.
When several drawings have changes, it asks about each one; Cancel stops.

![The question about unsaved changes](../../img/get-started-open-and-save-closing.png)

## Auto-backup and recovery {#recovery}

On the desktop, TraceDraw keeps backups of your open drawings while
auto-backup is on (the [[options.save]] page of
[[menu.tools > menu.tools.options > menu.tools.options_app]] sets how often
and where; every 20 minutes at first). Closing TraceDraw normally removes
them.

If TraceDraw ended without closing, for example after a power cut, it offers
the backups the next time it starts: [[dialog.recover]] opens each one as an
unsaved drawing (Save then asks where to save it), and [[dialog.discard]]
leaves the backups in their folder. Two TraceDraw windows running at once
never offer each other's drawings.
