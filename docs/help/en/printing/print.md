# Printing a drawing

Choose [[menu.file > menu.file.print]] (<kbd>Ctrl+P</kbd>). TraceDraw builds a
PDF of the sheets and opens it in your system's viewer, where you choose the
printer and print. In the browser version, the PDF is downloaded.

![The Print dialog box](../../img/printing-print.png)

## General {#general}

- **Destination**: the printer, a PDF file or a PostScript file.
- **Orientation**: match the layout, portrait or landscape.
- **Print range**: the whole document, the current page, a list of pages
  (such as `1-3, 5, 8-`), or the selected objects. The selected objects
  print alone (nothing else of the page, its master layers or its
  background), on a page the size of the area they cover: with the sheet
  set to the page's size the paper is that area, on a fixed sheet they are
  centred at their size (smaller only when they do not fit, unless
  **Keep page size** is ticked), and the layouts repeat that area.
- **Copies**, collated as whole sets or not.
- **Print as bitmap**: each sheet becomes one picture at the resolution you
  give.

## Color {#color}

- **Composite** prints all colors on each sheet; **Separations** prints one
  gray page per ink (cyan, magenta, yellow, black, then each spot ink).
- **Convert spot colors to CMYK** prints spot inks as their CMYK mix.

## Composite {#composite}

- **Print text in black**.
- **Always overprint black**: full black fills and outlines print over the
  colors beneath.

## Layout {#layout}

- **As in document**, **Fit to page**, or **Scale** to a percentage. **Fit
  to page** also shrinks objects larger than the page (or the selection, when
  you print only that) so they print whole on it.
- **Bleed limit**: how far beyond the page objects print (4 mm at first).
- The imposition: several pages on one sheet, or a booklet, whose pages are
  paired for folding, with the sheet size and the gap between pages. A sketch
  shows the result. For the back of double-sided cards, tick **Back side**:
  the pages run right to left on the sheet, so each one prints behind its
  front, and the text is not mirrored. **Keep page size** keeps the pages at
  their size on a fixed sheet: they do not shrink for the marks, and the
  columns and rows stay as you set them (pages that do not fit run off the
  sheet, and a warning says so). Without it, the pages shrink to fit the
  columns and rows you ask for. In Print
  Preview, drag on the sheet to move the page information.

## Booklets {#booklet}

Choose **Booklet** as the imposition to print two pages on each side of
a sheet, ready to fold. The page count is rounded up to a multiple of
four with blank pages at the back, and the summary under the sketch says
how many pages, blanks and sheets of paper it takes. To print only some
of the pages, list them in **Pages** first.

- [[print.binding]]: [[print.saddle_stitch]] nests every sheet inside the one
  before and staples them through the fold (8 pages: 8 1, 2 7 on the outer
  sheet). [[print.perfect_bound]] folds the sheets in signatures of
  [[print.signature_sheets]], stacks the signatures and glues them at the
  spine (one sheet per signature: 4 1, 2 3, then 8 5, 6 7).
- [[print.creep]] (saddle stitch): how far the pages of the innermost sheet
  move toward the spine, so the fore edges line up after trimming; the
  sheets between move in proportion, the outer one not at all.
- [[print.booklet_gap]]: the space between the two pages of a spread,
  centred on the fold.

## Prepress {#prepress}

- **Invert** and **Mirror** the whole sheet, for film.
- **Print file information** (the drawing's name or your own text, the page
  numbers, sheet number and date) and **Print page numbers**.
- **Crop marks** (and fold marks for booklets), **Registration marks**,
  **Color bars** and **Densitometer scale**.

## PostScript and issues {#issues}

With a PostScript file as the destination, the **PostScript** tab sets the
language level (2 or 3). The **Issues** tab shows what the preflight check
finds; see [Preflight](preflight.md).

**Apply** keeps the settings for the next time; see
[Global options](../customize/global-options.md#printing) for the defaults.

## Print preview {#preview}

[[menu.file > menu.file.print_preview]], or **Print preview** in the Print
dialog box, shows the sheets exactly as they will print, with the
imposition and the marks. The bar at the top has **Print**, **Close**, the
sheet shown with previous and next (<kbd>PgUp</kbd>, <kbd>PgDn</kbd>), the
plate to show while printing separations, and the zoom. The options on the
left change the sheets at once.

## Other ways to print {#other}

- [[menu.file > menu.file.send_to]] saves a PDF to your desktop or documents
  folder, or attaches it to an email.
- [[menu.file.publish_to_pdf]] writes a PDF with full control of its
  settings; see [Publishing to PDF](../file-formats/publish-pdf.md).
