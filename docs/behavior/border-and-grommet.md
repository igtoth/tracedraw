# Border and Grommet

Tools > Border and Grommet: finishing for large-format prints (banners,
canvases). The dialog grows the page by a border and places grommet
marks on its edge.

## The dialog

- On the left a preview: the final page, the border band (light grey for
  mirrored or stretched edges, the colour for a solid one), the original
  area dashed in blue and the grommets as circles of their size, with
  "Height: ..., Width: ..." of the final page in the document's units at
  the bottom.
- On the right, in titled boxes:
  - **Source**: Page, or Selection (only with something selected): the
    selection's box becomes the page.
  - **Border**: Add border (off at first); Type: Mirror the edges,
    Stretch the edges or Solid colour (with its colour); Size (31.75 mm
    at first), on every side.
  - **Grommet**: Add grommets (off at first); Use border area (with a
    border: each grommet is centred in the border band, and the margin
    is greyed); Size (the diameter, 6.35 mm at first); Margin (31.75 mm
    from the outer edge to the rim).
  - **Placement and distribution**: five toggle buttons, Corners, Top,
    Bottom, Left and Right edge (all on at first), then Number of
    grommets per edge (4, the default) or Spacing (one every 500 mm).
- "?", Reset (every field back to its default), OK and Cancel. OK is
  greyed while neither a border nor grommets are on.

## Border

- The page grows by the border on every side and the design stays where
  it was on the area; master layers are not moved. With Selection, the
  page takes the selection's box plus the border, and the objects move
  so the box sits inside the border.
- Mirror and Stretch are built from a render of the area itself (at up
  to about 4000 pixels on its longest side, 10 to 150 dpi), placed as
  bitmaps behind the content so the original objects stay editable. The
  area is drawn whole even where it reaches past the page: a selection
  larger than the page (a big imported photo) mirrors its own edges. It
  used to be cut from a render of the page, so the first run on such a
  selection mirrored strips from inside it, and only a second run,
  once the page had grown around it, came out right (#1283).

## Grommets

- Hairline circles of the diameter, drawn on a new layer with a
  localised name, with the border.
- Count mode places the number per edge (corners included); spacing
  mode one every spacing along each edge. Corner marks are shared by the
  two edges that meet there; a page too small for the margin and
  diameter gets no marks.
- Marks are placed on the page after the border has been applied.

## Checks (tests in `border_grommet.rs`)

- Given a 300 x 200 mm page with only Corners on, then the four centres
  are inset from each corner by the margin plus the radius.
- Given the same page at 100 mm spacing, then six marks: three along the
  top and bottom edges, two along the sides, corners not duplicated.
- Given a count of 3 per edge, then eight marks (four shared corners),
  with one at the middle of each side.
- Only the top edge gives marks on the top only; Use border area centres
  them in a 40 mm border at 20 mm from the edge, and without a border
  the margin applies.
- A selected rectangle with Selection and a 10 mm solid border becomes a
  page 20 mm larger than its box, with the box's corner at (10, 10), and
  two grommets per edge in the border give four marks.
- The final page size and the border rectangles follow from
  `area + 2 x border`.
