# Spot colours

A spot colour is a named ink printed on a plate of its own (a metallic,
a brand colour). The drawing keeps a list of its inks (`Document::inks`:
a number, a name and the ink's process stand-in, C, M, Y, K at full
strength), saved in `.tdraw`; a colour on an ink is `Color::Spot` (the
ink's number, a tint 0 to 100 % and the stand-in, so it shows and
converts without the list).

## Colour panel

Under Fill, Outline and the "..." menu, Spot inks lists the drawing's
inks: a swatch at the tint, the name, Fill and Outline (apply the ink at
the tint to the selection) and a cross that takes the ink off the list
(objects keep its stand-in). Tint (0 to 100 %, 100 by default) sets the
strength applied; while the palette view shows, the control sits under
its swatches instead (one Tint control in the panel). A name typed in Ink name and New spot ink add an ink
standing for the new colour (in process inks); names are unique. The
"..." menu's Add to Custom Spot Colors does the same with the name typed,
or with the colour's `#rrggbb` code when none is (see `color-docker.md`).

## Colour books

A palette file can be a colour book of spot inks: in an `.ase` palette,
an entry whose colour type is 1 (spot) becomes a spot colour named as
the entry, standing for its colour in process inks; global (0) and
normal (2) entries stay process colours. Its ink number comes from the
name (FNV-1a with the top bit set), so the same name is the same ink in
every drawing. Applying one (palette click, drag, the Colour panel) adds
the ink to the drawing's list, under the palette's name, as part of the
same undo step, unless the drawing has it already. Licensed colour
books are not bundled; a book the user owns opens like any palette
(Window > Color Palettes > Open Palette, `.tdpal`, `.gpl`, `.ase`,
`.aco`).

## Output

- On screen and in bitmap, SVG and `.cdr` output a spot colour is its
  stand-in times the tint (each process ink scaled).
- PDF: each ink becomes one Separation colour space (its name, the
  DeviceCMYK alternate through a type 2 function from no ink to the
  stand-in), referred to from the page resources as `/CS<ink>`; the tint
  is the colour value. A spot colour whose ink is no longer on the list
  prints as its stand-in. PDF/X keeps the Separation spaces.
- Separations (Print, or Export with separations): after the C, M, Y and
  K plates, a plate per ink of the list; a spot colour prints on its own
  plate only, at its tint, and on none of the process plates.

## Checks

- `color::tests::spot_colors_show_and_convert_through_their_stand_in` (core)
- `pdf::tests::spot_colors_print_on_separation_color_spaces` (io)
- `export::tests::spot_inks_get_plates_of_their_own`
- `ui::color_docker::tests::spot_inks_are_added_applied_and_removed`
- `palette::tests::colour_book_spot_entries_are_spot_colours`
- `palette::tests::a_colour_book_spot_colour_brings_its_ink_along`
