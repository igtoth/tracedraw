# Colour palettes

Two rows under the drawing window: the default palette, then the
document palette above the status bar (Window > Color Palettes >
Document Palette shows or hides it). The dotted grip at the start of the
default palette moves it to another edge (on a side it is a column) or
into a floating window with as many rows as the Rows setting (see "Moving the bars" in
`toolbars.md`); where it is put is kept across restarts, in the browser
too. The document palette stays a row under the drawing window wherever
the default palette is (over the drawing window when the default palette
is along the top). Each further palette ticked in Window > Color
Palettes (or in the Palette Manager) gets a strip of one row of its own
next to the default palette, in the order ticked, each further from
it: rows over it at the bottom (under it along the top), columns
inside it on a side, rows under it in its floating window (#1164). Its menu offers the Palette Editor, the Palette Manager and
Close Palette, which unticks it. Window > Toolbars > Color Palette hides or shows
them. The rows described below are the same along a column, turned
downwards.

## Look

- Each row is 25 px: a 1 px `#D8D8D8` line, a pixel of space, the
  swatches (20 x 20 px in a 1 px `#5A5A5A` frame that also separates
  them), a pixel of space. The background is the panel colour `#F4F4F4`.
- From the left: a dotted grip, the palette menu button (a dark triangle),
  the eyedropper (grey on the default palette, which cannot change; dark
  on the document palette), the left scroll arrow, then the swatches from
  77 px. At the right: the right scroll arrow and the button that shows
  every colour at once. Arrows are light grey when they cannot scroll.
- Rows (the palette menu's Rows > 1 Row, 2 Rows, 3 Rows, and Options >
  Customization > Color Palette; 1 at first, up to 3): the default
  palette shows that many rows of swatches (columns when docked on a
  side), each further row making it 21 px thicker. The swatches fill
  line by line: each line holds as many swatches as fit and the next
  line goes on where it ends (with 2 rows, Black and 90% Black sit side
  by side on the first line). The arrows and the wheel scroll one swatch,
  and the buttons stay on the first row. The document palette keeps one
  row (#610, `ui::palette::tests::the_default_palette_can_show_several_rows`,
  `ui::palette::tests::the_palette_menu_picks_the_rows`).
- The No Color well comes first: white with a red diagonal.
- The swatch under the pointer gets a white frame; the document palette's
  current swatch (the one clicked last) keeps it.
- An empty document palette shows "Drag colors (or objects) here to store
  these colors with your document".

## Default palette

The default CMYK palette, 99 named colours in this
order: Black, 90% to 10% Black, White, Blue, Cyan, Green, Yellow, Red,
Magenta, then tints in 20% steps (Purple 20 80 0 20, Orange 0 60 100 0,
Pink 0 40 20 0, Dark Brown 0 20 20 60, Powder Blue 20 20 0 0 and so on,
to Deep Blue 60 60 0 60).

In a drawing whose primary colour mode is RGB (New Document's colour
mode, Document Properties) the default palette shows and applies its
colours as clean RGB values (names kept; spot inks and registration stay
as they are). The default palette's colours come from the RGB default
palette, a table of RGB values defined directly
(`palette::default_rgb`, the same 99 names in the same order, also the
Palette Manager's built-in Default RGB palette), found by name and colour
so a default palette edited in the Palette Editor keeps them, and worked
out when the palette changes rather than every frame: 10%
greys (90% Black 26 26 26, 50% Black 128 128 128) and multiples of 51
otherwise (Black 0 0 0, Red 255 0 0, Orange 255 102 0, Purple 153 0 204,
Navy Blue 0 51 153, Dark Brown 102 51 51, Ruby Red 153 0 0, Crimson
153 51 102, Plum 102 0 102, Deep Blue 51 0 204). Any other CMYK colour
takes the plain RGB of its inks, R = 255 (1 - min(1, C + K)) and so on,
not the press simulation; other colours are converted as the colour
management converts. A CMYK drawing keeps them CMYK (#483, #620,
`ui::palette::tests::an_rgb_drawing_gets_an_rgb_palette`,
`ui::palette::tests::the_default_palette_of_an_rgb_drawing_is_clean_rgb`,
`ui::palette::tests::inks_with_black_become_rgb_by_adding_the_black`,
`ui::palette::tests::edited_default_palettes_keep_their_rgb_values`).

## Using a palette

| Action | Result |
|---|---|
| Click a swatch | Uniform fill of the selection (with nothing selected: the default fill for new objects) |
| Right click | Outline colour (Options > Customization > Color Palette can make it open the palette menu instead) |
| Ctrl+click | A tenth of the colour mixes into the selection's uniform fill (CMYK with CMYK, else in RGB) |
| Click and hold (0.5 s) | A 5 x 5 pop-up of the colour's shades: lighter tints upwards, more black downwards, less colourful to the left, more to the right; click one (right click for the outline) |
| Drag a swatch onto an object | Inside it (also inside a closed object with no fill): its uniform fill; within 4 points of its outline: the outline's colour (an object with no outline gets the default pen in that colour); No Color takes the fill or the outline away. The object need not be selected and the selection stays; one undo step; the colour joins the document palette as with a click |
| Drag a swatch onto the document palette | The colour joins it |
| Drag objects (Pick tool) onto the document palette | Their fill and outline colours join it (as Add from Selection); the objects stay where they were; the pointer shows a copy sign there |
| While dragging a swatch | A chip of the colour follows the pointer: filled over a fill, a ring over an outline; the pointer shows whether letting go changes anything; Esc drops nothing |
| Scroll arrows, wheel | One swatch at a time (every row moves by one); holding an arrow keeps scrolling |
| The show-all button | Every colour of the row in a grid above it (below it, clear of every row, when the palette is along the top; beside a column, clear of every column; #610, `ui::palette::tests::every_colour_opens_clear_of_all_the_rows`); click to apply, click elsewhere or Esc to close |
| Tooltip | The colour's name in bold, then its values one per line (`C: 60`, `M: 0`, `Y: 40`, `K: 20`; `R: 255`, `G: 0`, `B: 0`) |
| No drawing shown (Welcome Screen, or none open) | The swatches keep their colours and their tooltips; clicks and drags do nothing and the eyedropper is off (#1082) |

## Document palette

- Every colour applied to an object (fill or outline, from a palette, the
  colour panel or a dialog) joins the document palette, in the same undo
  step as the change. The palette menu's Automatically Update turns that
  off (also in Options > Customization > Color Palette).
- A colour dragged in a colour picker (Properties > Fill > Uniform, and
  Properties > Outline > Colour): the object shows each colour the
  pointer passes over, but the whole drag is one undo step and only the
  colour it ends on joins the document palette, when the button is let
  go. Other drags in the outline editor (its sliders) are one undo step
  too.
- With characters selected in a text being edited, a swatch colours
  only them (see `text.md`, Character colours).
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

Open Palette... (files of `.tdpal`, `.xml`, `.gpl`, `.ase`, `.aco` and
`.json`, the filter naming each extension; an XML palette is read as
the document palette of a `.cdr` file, its CMYK, RGB, grey, Lab and the
other colour models with their names, spot colours skipped, and a file
with no colour that can be read says so in the status bar, #882,
`palette::tests::xml_palettes_open_with_names_and_values`), Palette Editor..., Palette Manager, the Document
Palette switch, and Rows > 1 Row, 2 Rows, 3 Rows (a radio choice, kept
in the preferences, the same setting as in Options). A right click on a row outside the swatches opens the
same menu.

## Options > Customization > Color Palette

Show "No Color" well (on); Rows of the default palette (1, 2 or 3;
`palette.rows`, 1 at first); right mouse button: Context menu or Set
outline color (default); Show the document palette (on); Automatically
update the document palette (on).

## Checks

- `ui::palette::tests::tooltips_name_the_colour_and_its_values`
- `ui::palette::tests::the_default_palette_has_greys_process_colours_and_tints`
- `ui::palette::tests::shades_keep_the_colour_in_the_middle`
- `ui::palette::tests::mixing_moves_a_tenth_of_the_way`
- `ui::palette::tests::applying_a_colour_adds_it_to_the_document_palette_in_the_same_step`
- `ui::palette::tests::rows_fit_whole_swatches`
- `ui::palette::tests::the_default_palette_of_an_rgb_drawing_is_clean_rgb`
- `ui::palette::tests::the_default_palette_can_show_several_rows`
- `ui::palette::tests::every_colour_opens_clear_of_all_the_rows`
- `pick_drag_tests::every_palette_shown_gets_a_strip`
- `ui::dockers::tests::a_fill_dragged_in_the_picker_is_one_step_and_one_palette_colour`
- `color_drop::tests` (fill inside, outline on the edge, No Color, an
  object without a fill, the document palette)
- `tracedraw_core` `engine::tests::amending_joins_the_last_step_only_right_after_it`
- `tracedraw_cdr` `container::tests::document_palette_xml_reads_names_and_tints`
