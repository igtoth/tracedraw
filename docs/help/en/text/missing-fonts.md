# Missing fonts

A drawing made on another computer can use fonts that are not installed on
yours. TraceDraw always shows such text in an installed font that looks
close, and keeps the text's own font name. When the drawing opens, the
[[missing_fonts.title]] dialog box lets you choose which installed font
draws each missing one, for now or for good.

## Choosing the fonts that stand in {#dialog}

When you open a drawing with missing fonts, the [[missing_fonts.title]]
dialog box lists them:

1. Each row shows the [[missing_fonts.missing]] and the font it is
   [[missing_fonts.substitute]] now. The first choice in the list is the
   font TraceDraw picked by itself, marked "(automatic)": a font drawn to
   the same widths when there is one, else one of the same style.
2. To use another font, open the list of the row and click an installed
   family. Type part of a name in [[missing_fonts.filter]] to narrow the
   lists.
3. Choose how long the choice holds:
   - [[missing_fonts.session]] (the default): the font you chose draws the
     text until TraceDraw closes, bold and italic text included. The drawing
     keeps its own font names, so it is saved with them and shows in its own
     fonts on a computer that has them.
   - [[missing_fonts.keep]]: every text in the missing family, also in
     groups, clip frames, tables and symbols, takes the font you chose. This
     changes the drawing and undoes in one step.
4. Click [[dialog.ok]].

[[dialog.cancel]], <kbd>Esc</kbd> or the close button change nothing: the
automatic fonts stay. Either way, the status bar then names the fonts still
missing and the fonts that draw them, or says they were replaced. When some
objects of the file could not be read, their note follows the dialog box.

## Turning the dialog box off and on {#turn-off}

Tick [[notes.dont_show]] in the dialog box to stop it. The drawing then opens
straight away and the status bar names the missing fonts. Click that note on
the status bar to open the [[options.warnings]] page of
[[menu.tools > menu.tools.options > menu.tools.options_global]], where
[[options.ask_missing_fonts]] turns the dialog box on again (it is on at
first).

[[options.warn_missing_fonts]], on the same page, is a separate, plain message
that only names the missing fonts after the drawing opens.

Opening drawings from scripts, the command line or the AI connection never
stops at this dialog box.

## Other ways to deal with missing fonts {#other}

- Install the fonts, or add their folder in the Font Manager (see
  [Fonts and glyphs](fonts.md#font-manager)); the text then shows in them.
- In the [[docker.fonts]] panel, families the drawing uses that are not
  installed are marked [[docker.missing]].
- Drawings saved as `.tdraw` and `.cdr` files with [[dialog.embed_fonts]]
  carry their fonts along, and a [package](../file-formats/package.md) holds
  the font files in its `Fonts` folder.
- The [preflight](../printing/preflight.md) lists text in fonts that are not
  installed before you print or publish.
