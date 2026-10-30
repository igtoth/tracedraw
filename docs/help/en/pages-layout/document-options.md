# Document options

[[menu.layout > menu.layout.document_options]] opens the settings of the
active drawing: page size, layout, background, bleed, rulers, grid and
guidelines. Double-clicking a ruler or the page border also opens it. Choose a
page from the list on the left. Changes show at once; [[dialog.ok]] keeps
them and [[dialog.cancel]] puts everything back. **Save as default** keeps the
grid, ruler and guideline settings for new drawings.

## General {#general}

![The General page of the document options](../../img/pages-layout-document-options-general.png)

- [[options.rendering_resolution]]: the resolution of effects that are
  turned into pixels, such as drop shadows and bitmap effects.
- [[options.fill_open_curves]] (off at first): when on, open curves show their
  fill as if they were closed.
- [[options.auto_inflate_bitmaps]] (on at first): bitmaps grow to make room
  for effects that spread past their edges.
- [[options.duplicate_offset]]: how far [[menu.edit.duplicate]] places the
  copy.

## Page size {#page-size}

![The Page Size page of the document options](../../img/pages-layout-document-options-page-size.png)

- Choose [[options.page_size_radio]] for a paper size, or
  [[options.label_presets]] for sheets of labels, which set the page to the
  sheet size and add guidelines around each label.
- [[options.size]], [[dialog.width]] and [[dialog.height]], and
  [[options.portrait]] or [[options.landscape]].
- [[options.apply_current_only]]: change the current page only.
- [[options.show_page_border]], and [[options.add_page_frame]], which adds a
  rectangle the size of the page.
- [[options.rendering_resolution]], [[options.bleed]] and
  [[options.show_bleed]] (see below).

## Layout {#layout}

![The Layout page of the document options](../../img/pages-layout-document-options-layout.png)

The page shows how many pages the drawing has, with
[[menu.view.page_sorter]] and [[menu.layout.switch_orientation]], and lists
the [[options.master_layers]], each shown on [[options.scope_all]],
[[options.scope_odd]] or [[options.scope_even]]. The buttons add a new master
layer for all, odd or even pages. See
[Master layers](../objects/objects-and-layers.md#master-layers).

## Background {#background}

![The Background page of the document options](../../img/pages-layout-document-options-background.png)

Choose [[options.no_background]], a [[options.solid]] color, or a
[[options.bitmap]] ([[options.choose_bitmap]]) to show behind every page.

## Bleed {#bleed}

![The Bleed page of the document options](../../img/pages-layout-document-options-bleed.png)

Bleed is the part of the drawing that runs past the edge of the page, so the
print has no white edge once trimmed. Set the [[options.bleed]] distance and
tick [[options.show_bleed]] to see it. Printing and PDF export can include the
bleed; see [Printing](../printing/print.md).

## Rulers {#rulers}

![The Rulers page of the document options](../../img/pages-layout-document-options-rulers.png)

- [[options.units]]: the drawing units (inches, millimeters, picas, points,
  pixels, ciceros, didots, feet, yards, miles, centimeters, meters or
  kilometers).
- [[options.drawing_scale]]: makes a distance on the page stand for a distance
  in the world, such as 1:100. Choose a typical scale, or type the page
  distance and the world distance. Rulers, positions, sizes and new dimension
  lines then show world distances; page sizes, outline widths and text sizes
  do not.
- [[options.origin]]: where the rulers count from
  ([[options.horizontal_origin]] and [[options.vertical_origin]]);
  [[options.reset_origin]] puts it back. You can also drag the origin out of
  the corner where the rulers meet.
- [[options.tick_divisions]]: the number of ticks between ruler numbers.
- [[options.nudge_section]]: the [[options.nudge_label]] distance of the arrow
  keys (2.54 mm), the [[options.super_nudge]] with <kbd>Shift</kbd> (5.08 mm)
  and the [[options.micro_nudge]] with <kbd>Ctrl</kbd> (0.254 mm).
- [[options.show_rulers]].

## Grid {#grid}

![The Grid page of the document options](../../img/pages-layout-document-options-grid.png)

- [[options.document_grid]]: [[options.show_grid]], [[options.snap_to_grid]],
  the [[options.horizontal]] and [[options.vertical]] spacing (10 mm at
  first), as a distance or as lines per unit, and [[options.show_grid_as]]
  [[options.grid_lines]] or [[options.grid_dots]].
- [[options.baseline_grid]]: lines across the page for lining up text, with
  the [[options.baseline_spacing]] (14 points), the
  [[options.baseline_start]] (12.7 mm) and the [[options.line_color]].
- [[options.pixel_grid]]: the [[options.color]] and [[options.opacity]] of the
  grid of pixels, [[options.pixel_grid_800]] and [[options.snap_to_pixels]].

## Guidelines {#guidelines}

![The Guidelines page of the document options](../../img/pages-layout-document-options-guidelines.png)

- [[options.show_guidelines]] and [[options.snap_to_guidelines]].
- [[options.default_guide_color]] and [[options.default_preset_guide_color]].
- [[guides.presets]]: one inch margins, bleed area, page borders, printable
  area, a three-column newsletter, a basic grid and an upper left grid.
- [[guides.user_defined]]: [[guides.margins]] ([[guides.top]],
  [[guides.bottom]], [[guides.left]], [[guides.right]]), [[guides.columns]]
  ([[guides.number_of_columns]] and [[guides.distance_between]]) and a
  [[guides.grid]] ([[guides.rows]] and [[guides.columns_count]]).
  [[guides.apply_presets]] adds them; a line that is already there is not
  added twice.

See [Rulers, grids and guidelines](guidelines-and-grid.md) for working with
guidelines on the page.
