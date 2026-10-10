# Drawing window

## Desktop and page

- The desktop (the area around the page) is white by default; the
  colour is a setting (`desktop_rgb`, Options > Customization).
- The page is white with a 1 px border `#AAAAAA` and a flat shadow
  `#BFBFBF`: the bordered page rectangle moved 6 px right and 4 px down,
  drawn behind the page, so a 6 px band shows on the right and a 4 px
  band at the bottom. The page rectangle is snapped to whole pixels.
  View > Page > Page Border hides the border and the shadow.
- Double-clicking the page border (within 3 px) or its shadow with the
  Pick tool, where no object is hit, opens the page size options
  (Layout > Document Options at Page Size).

## Scrollbars

- The vertical scrollbar runs down the right edge between the ruler and
  the bottom row; the horizontal one fills the right part of the bottom
  row. Both are 17 px: arrow buttons at the ends, a light track and a flat
  grey thumb that darkens when hovered or dragged.
- They cover the page plus one page size on every side, grown to include
  whatever is visible. Dragging the thumb pans; a click on the track pages
  by 90 % of the window towards the click; an arrow moves 40 px, then
  repeats every 50 ms after 0.4 s while held.

## Bottom row

From the left, after the ruler's width:

1. The document navigator: insert page before, first page, previous
   page, "n of m", next page, last page, insert page after. The insert
   buttons add a page of the current size next to the current page and
   make it current.
2. The page tabs: one slanted tab per page, the current one white; a click
   goes to the page, a double click renames it.
3. A dotted splitter: dragging it shares the row between the page tabs and
   the horizontal scrollbar (45 % for the tabs by default).
4. The horizontal scrollbar.

The Navigator button (a magnifier over a cross) sits in the corner under
the vertical scrollbar; see `zoom-and-pan.md`.

## Checks

- `ui::window_bars::tests::insert_page_before_and_after_the_current_one`
- `ui::window_bars::tests::the_page_border_and_shadow_are_hit_but_not_the_page_inside`
