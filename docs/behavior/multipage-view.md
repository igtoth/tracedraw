# Multipage View

View > Multipage View lays every page of the drawing out on the desktop
at once, so pages can be compared and edited side by side.

## Layout

- Pages sit in a grid with as many columns as the square root of the
  page count, rounded up (4 pages: 2 x 2; 5 to 9 pages: 3 columns),
  left to right, then down, in page order.
- Each column is as wide as its widest page and each row as tall as its
  tallest; a page sits at the top left of its cell. Cells are 30 mm
  apart.
- Every page shows its number and name below it; the current page's in
  the normal text colour, the others dimmed.
- Objects outside a page's edges are not shown on the other pages; the
  current page shows them as always.

## Working

- The current page is edited as in the normal view: rulers, guides,
  grids and the status bar refer to it.
- A press on another page makes it current before the tool acts, so a
  click selects there and a drag draws there. Nothing moves on screen.
- The page tabs and the navigator change the current page the same way.
- Zoom to Fit (F4) and turning the view on fit every page in the window.
- Page Sorter View and Multipage View exclude each other.

## Drawing

- Each other page keeps its own picture. While the view pans or zooms,
  the last picture is scaled and moved with its paper, and the pages are
  rendered anew once the view has rested for 0.12 s.
- A page that covers no more of the screen than the canvas is rendered
  whole; a larger one only where it shows.
- One frame spends at most 10 ms rendering the current page and the
  others; pages left over show their last picture and render in the next
  frames. One render runs every frame, so every page gets its turn.

## Formula

With `cols = ceil(sqrt(n))`, page `i` is in row `i / cols`, column
`i % cols`; its left edge is the sum of the earlier column widths plus
`column * 30` mm, its top edge the sum of the earlier row heights plus
`row * 30` mm below the first row's top. The grid is then moved so the
current page's bottom-left corner is the origin.
