# Publishing to PDF

[[menu.file > menu.file.publish_to_pdf]] (also on the standard toolbar) writes
a PDF file with full control of what goes in: which pages, which PDF standard,
how pictures are compressed, document information, bookmarks, printers' marks
and password protection. The tabs are [[publish.tab_general]],
[[publish.tab_color]], [[publish.tab_document]], [[publish.tab_objects]],
[[publish.tab_prepress]], [[publish.tab_security]] and the issues tab.

![The PDF Settings dialog box](../../img/file-formats-publish-pdf.png)

## General {#general}

- **File name**, with **Browse** to choose another place.
- **Export range**: the current document (every page), every open document,
  the current page, a list of pages (such as `1-3, 5`), or the selection.
- **Page size** for a selection: [[publish.size_document]] (the page, with
  only the selected objects) or [[publish.size_selection]] (a page the size of
  the selected objects, the default).
- **PDF preset**: [[publish.preset_distribution]], [[publish.preset_web]],
  [[publish.preset_prepress]], [[publish.preset_x1a]], [[publish.preset_x3]],
  or your own; **+** saves the current settings as a preset under a name you
  type (the same name replaces it) and **-** deletes the chosen one. A preset
  sets the compatibility, the output condition, the flattening resolution,
  the bookmarks, the bleed, the printers' marks and the bitmap choices.
- **Compatibility**: PDF 1.4, PDF/X-1a:2003, PDF/X-3:2003 or PDF/X-4. PDF/X
  files are made for print shops.

## Color {#color}

For PDF/X, the **output condition** (FOGRA39 at first); the CMYK profile of
the drawing goes into the file. A plain PDF keeps the colors as they are.

## Document {#document}

**Title**, **Author**, **Subject** and **Keywords**, filled in from the
drawing's properties; **a bookmark for each page** (on at first) and **show
the bookmarks when the file opens**.

## Objects {#objects}

- [[publish.flatten_resolution]] a resolution (300 dpi), for PDF/X-1a and
  X-3, which do not allow transparency. The other standards keep
  transparency live, so the field is greyed for them.

### Bitmaps {#bitmaps}

The [[publish.bitmaps]] group sets how pictures go into the file. The
choices are kept for the next time and in the presets.

- [[publish.downsample_color]] (off, 300 dpi): a colour picture with more
  pixels than this resolution, as it is placed on the page, is made smaller
  to that resolution.
- [[publish.downsample_gray]] (off, 300 dpi): the same for pictures that are
  only grays. Gray pictures always go in with one channel, which keeps them
  small.
- [[publish.downsample_mono]] (off, 1200 dpi): the same for pictures of only
  black and white pixels.
- [[publish.compression]]: [[publish.compression_zip]] (the default) keeps
  every pixel; [[publish.compression_jpeg]] makes smaller files with some
  loss, at the [[publish.jpeg_quality]] you set (1 to 100, 85 at first).
  Black-and-white pictures, transparency masks and PDF/X-1a files always use
  ZIP.
- [[publish.compress_content]] (on) compresses the pages' text and drawing.

A picture that lies wholly outside the page and its bleed cannot show, so it
is left out of the file.

The [[publish.preset_web]] preset downsamples colour and gray pictures to
150 dpi and black-and-white ones to 300 dpi, with JPEG at quality 75.

## Prepress {#prepress}

- [[dialog.bleed]] (0 to 50 mm, the drawing's bleed at first) around every
  page: the trim box is the page, the media box the page and the bleed.
- [[publish.marks]]: [[dialog.crop_marks]], [[dialog.registration_marks]],
  [[print.color_bars]] and [[publish.file_info]] around every page. See
  [Printers' marks in PDF files](pdf-prepress-marks.md).

## Text and fonts {#text}

Text goes into the PDF as real text: in a PDF viewer you can search it,
select it and copy it, and it keeps its fill, outline, colors and angle. The
fonts go into the file as well, so the text looks the same on any computer.
In the [[publish.text_fonts]] group of the Objects tab:

- **[[publish.text_as_curves]]** (off at first) writes every text as curves
  instead: the file has no fonts and its text cannot be searched. The curves
  follow the letters' own outlines exactly, so they stay smooth at any zoom.
- **[[publish.embed_fonts]]** (on at first) puts the fonts in the file. Turned
  off, the fonts are only named and the viewer uses its own copy.
- **[[publish.embed_base14]]** also puts Helvetica, Times and Courier in the
  file; otherwise the viewer's standard fonts draw them.
- **[[publish.subset_fonts]]** puts in only the letters the file uses, which
  keeps it small, when a font uses less than the share you set (80 %).

PDF/X files always carry their fonts. Some fonts do not allow being put in
other files: their text is written as curves, the status bar names them after
publishing, and the preflight lists such text beforehand.

## Security {#security}

- **Require a password to open the file**, and the password.
- **Restrict printing, copying and changes with a password**, and the
  password, then the **permissions**: printing (not allowed, low resolution
  only, allowed), copying, changing the content, adding comments, filling in
  forms, managing pages and text access for screen readers.
- **Show passwords**.

PDF/X files cannot be protected. The passwords are never kept.

## Issues {#issues}

The last tab lists what the preflight check finds; see
[Preflight](../printing/preflight.md). Click [[dialog.ok]] to write the file.
If something is missing, such as a password or a page in the list, the dialog
box says so in red.

## Opening protected PDF files {#password}

When you open or import a PDF that needs a password, the
[[pdf_password.title]] dialog box asks for it. Type the
[[publish.password]] and click [[dialog.ok]]. A wrong password is refused and
you can try again; [[dialog.cancel]] leaves the file closed.

![The Protected PDF dialog box](../../img/file-formats-publish-pdf-password.png)
