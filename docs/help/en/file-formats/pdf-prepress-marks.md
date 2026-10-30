# Printers' marks in PDF files

A print shop trims, registers and checks printed sheets with marks drawn
outside the page: lines that show where to cut, targets that line up the
plates, patches of ink and a line naming the file. The Prepress tab of
[[menu.file > menu.file.publish_to_pdf]] adds these marks around every page
of the PDF, so the file can go to the press as it is.

## Adding the marks {#add}

1. Choose [[menu.file > menu.file.publish_to_pdf]].
2. On the [[publish.tab_prepress]] tab, set the [[dialog.bleed]]: how far the
   drawing runs past the edge of the page (the drawing's bleed at first,
   0 to 50 mm). Objects that reach past the page print up to this line, so
   no white edge shows after trimming.
3. Under [[publish.marks]], tick the marks you want (all are off at first):
   - [[dialog.crop_marks]]: short lines (5 mm) in line with the edges of the
     page, starting 1.5 mm outside the bleed, that show where to cut.
   - [[dialog.registration_marks]]: a target (a small circle with a cross) in
     the middle of each side, to line up the plates of each ink.
   - [[print.color_bars]]: patches of the four process inks, the mixes of
     two of them and their 50 % tints, beside the top target, to check the
     inks on the press.
   - [[publish.file_info]]: the drawing's name, the page's name, the page
     number and page count, and the date and time, under the page.
4. Click [[dialog.ok]].

The [[publish.preset_prepress]] preset ticks [[dialog.crop_marks]]. The marks
you tick are kept for the next time and in the presets you save with **+**.

## What the file looks like {#boxes}

When any mark is ticked, each page of the PDF grows by a 12 mm margin
outside the bleed, on every side, to hold the marks:

- The trim box, the size of the finished piece, is the page.
- The bleed box is the page and its bleed. The drawing is clipped to it, so
  nothing of it runs into the marks.
- The media box, the whole sheet of the PDF, takes in the bleed and the
  margin with the marks.

A page background covers the bleed too. Links and bookmarks keep pointing
at the page.

The marks are drawn in the registration colour, so on a press they print on
every plate, and they print the same in separations from a PDF viewer.
Without any mark ticked the page is the page and its bleed, as before.

> **Tip:** The marks are the same ones the [[menu.file > menu.file.print]]
> dialog box draws on its sheets (see
> [Printing a drawing](../printing/print.md#prepress)). Use the PDF when the
> print shop asks for a file; use Print when you print the sheets yourself.
