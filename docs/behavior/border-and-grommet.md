# Border and Grommet

Tools > Border and Grommet: finishing for large-format prints (banners,
canvases). The dialog grows the page by a border and places grommet
marks on the new edge.

## Border

- Kinds: None, Mirror (the edge content reflected outwards), Stretch (the
  edge pixels pulled outwards), Solid (a colour).
- Width: default 25 mm, applied to all four sides. The page grows by the
  border on every side and the design stays centred; master layers are
  not moved.
- Mirror and Stretch are built from a render of the current page at the
  document's rendering resolution, placed as a bitmap behind the content
  so the original objects stay editable.

## Grommets

- Diameter 12 mm (the mark is a hairline circle of that diameter, drawn
  on a new layer with a localised name).
- Margin 10 mm from the page edge to the circle's rim.
- Placement: one every 500 mm along each edge (default), or a fixed
  count per edge (default 4), or corners only. Corner marks are shared by
  the two edges that meet there; a page too small for the margin and
  diameter gets no marks.
- Marks are placed on the page after the border has been applied, so
  they sit on the border, not on the design.

## Checks (tests in `border_grommet.rs`)

- Given a 300 x 200 mm page, corners only, then the four centres are
  inset from each corner by the margin plus the radius (10 mm in the
  test's parameters).
- Given the same page at 100 mm spacing, then six marks: three along the
  top and bottom edges, two along the sides, corners not duplicated.
- Given a count of 3 per edge, then eight marks (four shared corners),
  with one at the middle of each side.
- The final page size and the border rectangles follow from
  `page + 2 x border`.
