# Printing booklets

A booklet prints two pages on each side of a sheet of paper, placed so that
the folded sheets read in order: a menu, a program, a small catalog or a
zine. TraceDraw pairs the pages for you; you choose how the booklet is bound.

## Printing a booklet {#print}

1. Put the pages of the drawing in reading order, the cover first.
2. Choose [[menu.file > menu.file.print]] (<kbd>Ctrl+P</kbd>) or
   [[menu.file.print_preview]].
3. On the [[print.tab_layout]] tab, choose [[print.layout_booklet]] as the
   [[print.imposition]].
4. Under [[print.binding]], click one of the two pictures (see below).
5. Set the binding's fields: [[print.signature_sheets]] for a perfect-bound
   booklet, [[print.creep]] for a saddle-stitched one, and the
   [[print.booklet_gap]] for either.
6. Check the sketch and the counts under it, then print on both sides of the
   paper.

To make a booklet of only some pages, list them in the pages to print on the
[[print.tab_general]] tab first.

## Choosing the binding {#binding}

- [[print.saddle_stitch]] (the default): every sheet is folded inside the one
  before and the booklet is stapled through the fold. For eight pages, the
  outer sheet holds pages 8 and 1 on one side and 2 and 7 on the other; the
  inner sheet holds 6 and 3, then 4 and 5. Best for thin booklets.
- [[print.perfect_bound]]: the sheets go in signatures, small groups of
  [[print.signature_sheets]] (4 at first, 1 to 32) folded together. The
  signatures are stacked and glued at the spine. The last signature takes
  the sheets left over. With one sheet per signature the pages pair as
  4 1, 2 3, then 8 5, 6 7. Best for thicker booklets.

## The settings {#settings}

- [[print.signature_sheets]] (perfect bound only): how many sheets are folded
  together in each signature.
- [[print.creep]] (saddle stitch only, 0 to 20 mm, 0 at first): when nested
  sheets are folded, the inner ones stick out at the open edge and lose more
  when the booklet is trimmed. Creep moves the pages of the innermost sheet
  this far toward the spine; the sheets between move in proportion and the
  outer sheet does not move. The crop marks move with the pages.
- [[print.booklet_gap]] (0 to 100 mm, 0 at first): the space between the two
  pages of a spread, centred on the fold. On a sheet sized to the pages, the
  sheet grows by the gap.

## Blank pages and the counts {#counts}

A booklet always has a multiple of four pages, since each sheet holds four.
When the drawing has another number, blank pages are added at the back.
Under the sketch, the summary gives the number of pages, the blank pages
added and the sheets of paper printed on both sides, and for a perfect-bound
booklet the number of signatures.

With [[dialog.crop_marks]] on the [[print.tab_prepress]] tab, a booklet also
gets a dashed fold mark at the spine. See
[Printing a drawing](print.md#prepress) for the other marks.

> **Tip:** Print one copy first and fold it to check the order and the
> margins before you print them all.
