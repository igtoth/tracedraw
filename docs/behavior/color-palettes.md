# Colour palettes

Two rows under the drawing window: the
default palette, then the document palette above the status bar
(Window > Color Palettes > Document Palette shows or hides it).

## Look

- Each row is 25 px: a 1 px `#D8D8D8` line, a pixel of space, the
  swatches (20 x 20 px in a 1 px `#5A5A5A` frame that also separates
  them), a pixel of space. The background is the panel colour `#F4F4F4`.
- From the left: a dotted grip, the palette menu button (a dark triangle),
  the eyedropper (grey on the default palette, which cannot change; dark
  on the document palette), the left scroll arrow, then the swatches from
  77 px. At the right: the right scroll arrow and the button that shows
  every colour at once. Arrows are light grey when they cannot scroll.
- The No Color well comes first: white with a red diagonal.
- The swatch under the pointer gets a white frame; the document palette's
  current swatch (the one clicked last) keeps it.
- An empty document palette shows "Drag colors (or objects) here to store
  these colors with your document".

## Default palette

the target design's default CMYK palette, 99 named colours in its
order: Black, 90% to 10% Black, White, Blue, Cyan, Green, Yellow, Red,
Magenta, then tints in 20% steps (Purple 20 80 0 20, Orange 0 60 100 0,
Pink 0 40 20 0, Dark Brown 0 20 20 60, Powder Blue 20 20 0 0 and so on,
to Deep Blue 60 60 0 60). The values of the first 64 were checked
against the target design's palette on screen through the press model
(`colour-management.md`).

## Using a palette

| Action | Result |
|---|---|
| Click a swatch | Uniform fill of the selection (with nothing selected: the default fill for new objects) |
| Right click | Outline colour (Options > Customization > Color Palette can make it open the palette menu instead) |
| Ctrl+click | A tenth of the colour mixes into the selection's uniform fill (CMYK with CMYK, else in RGB) |
| Click and hold (0.5 s) | A 5 x 5 pop-up of the colour's shades: lighter tints upwards, more black downwards, less colourful to the left, more to the right; click one (right click for the outline) |
| Scroll arrows, wheel | One swatch at a time; holding an arrow keeps scrolling |
| The show-all button | Every colour of the row in a grid above it; click to apply, click elsewhere or Esc to close |
| Tooltip | The colour's name, then its values: `C: 60 M: 0 Y: 40 K: 20`, `R: 255 G: 0 B: 0` |

## Document palette

- Every colour applied to an object (fill or outline, from a palette, the
  colour docker or a dialog) joins the document palette, in the same undo
  step as the change. The palette menu's Automatically Update turns that
  off (also in Options > Customization > Color Palette).
- The eyedropper waits for a click on the drawing and adds the colour
  under the pointer; with Ctrl it keeps sampling. Esc or a right click
  stops it.
- Its menu: Automatically Update, Add from Selection, Add from Document
  (every colour used in the drawing, after the ones already there),
  Delete Color (the current swatch), Palette > Reset Palette (keeps only
  the colours the drawing still uses).
- It is saved with the drawing; opening a `.cdr` reads its
  `color/docPalette.xml`.

## Default palette menu

Open Palette..., Palette Editor..., Palette Manager, and the Document
Palette switch. A right click on a row outside the swatches opens the
same menu.

## Options > Customization > Color Palette

Show "No Color" well (on); right mouse button: Context menu or Set
outline color (default); Show the document palette (on); Automatically
update the document palette (on).

## Checks

- `ui::palette::tests::tooltips_name_the_colour_and_its_values`
- `ui::palette::tests::the_default_palette_is_the_reference_one`
- `ui::palette::tests::shades_keep_the_colour_in_the_middle`
- `ui::palette::tests::mixing_moves_a_tenth_of_the_way`
- `ui::palette::tests::applying_a_colour_adds_it_to_the_document_palette_in_the_same_step`
- `ui::palette::tests::rows_fit_whole_swatches`
- `tracedraw_core` `engine::tests::amending_joins_the_last_step_only_right_after_it`
- `tracedraw_cdr` `container::tests::document_palette_xml_reads_names_and_tints`
