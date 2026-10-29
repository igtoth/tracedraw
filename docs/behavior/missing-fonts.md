# Missing fonts

When a drawing opens with text in font families that are not installed
(and Options > Global > Warnings, Missing fonts, is on), the windowed
application shows the Missing Fonts dialog (#936). Automated opens never
ask: the headless application used by tests, scripts and the MCP server
(`App::headless`, `App::ask_missing_fonts` false) keeps the plain note
after opening, and the command line tool has no dialogs.

## The dialog

- A row per missing family: its name and the installed font that draws
  its text, to start with the automatic choice (`FontSystem::substitute`:
  a font drawn to the same widths, else one of the same style, else the
  general fallback; see `text.md`), shown first in the list as
  "<font> (automatic)", then every installed family. Find a font narrows
  the list as letters are typed.
- For this session only (the default): the drawing keeps its font names;
  the font chosen draws each family until TraceDraw closes
  (`FontSystem::set_substitute`, tried before the automatic stand-ins,
  bold and italic text included). Saved files keep the missing names, so
  the text comes back in its own font on a computer that has it.
- Replace in the drawing (saved with it): every span in the missing
  family, in groups, clip frames, tables and symbols, takes the font
  chosen (`Command::ReplaceFontFamily`, one undo step, "Replace Font").
  The drawing has unsaved changes afterwards.
- Do not show this message again turns the warning off.
- OK applies the choices; Cancel, Esc or the close button changes
  nothing and the automatic stand-ins stay. The status line then names
  the fonts still missing and what draws them, or says they were
  replaced.
- When objects of the file could not be read, their note follows the
  dialog.

## Checks

- Given a headless app, when a drawing with a missing font opens, then
  the plain note is shown, not the dialog; given the windowed app, the
  dialog lists the family with the automatic choice; choosing a font and
  Replace in the drawing, then OK (Enter), replaces the family in one
  undo step and the unread note follows
  (`ui::missing_fonts::tests::missing_fonts_are_replaced_in_the_drawing_when_asked`).
- Given For this session only, the font chosen draws the family, the
  drawing keeps its font name and the status names the stand-in
  (`ui::missing_fonts::tests::a_font_chosen_for_the_session_draws_the_text_and_the_drawing_keeps_its_font`).
- `tracedraw_text`: a chosen stand-in draws the family and clears back to
  the automatic one (`tests::a_chosen_stand_in_draws_a_missing_family`).
- `tracedraw_core`: `command::tests::a_font_family_is_replaced_throughout_the_drawing`.

Not done: choices kept between sessions (a table of saved substitutions
in the options).
