# Text

Artistic text: a single line (or several, with Enter) placed at a
baseline point; paragraph text: a frame with wrapping. Shortcut F8.

| Control | Default |
|---|---|
| Font | first sans-serif found (Arial, Liberation Sans, DejaVu Sans) |
| Size | 24 pt artistic, 12 pt paragraph |
| Leading | 100% of the font's line height (ascender - descender + line gap) |
| Paragraph spacing | before 0, after 0 |
| Kerning | font kerning on (GPOS kern, kern table) |
| Tracking (range kerning) | 0% of em |
| Baseline shift | 0 pt |
| Alignment | left; justify stretches spaces only, last line left |
| Hyphenation | off; when on, at least 2 letters before and 3 after a break, from a compact pattern set (a vowel and consonant rule for words it leaves whole); digraphs (ch, ck, gh, ph, sh, th, wh, qu) stay together; words with a capital letter (names, acronyms) are never broken |
| Columns | 1, gutter 5 mm |
| Bullets | off; indent 5 mm, bullet "*" from the paragraph font |
| Drop cap | off; 3 lines. The paragraph's first letter is scaled so it sits on the baseline of the last line it spans and its top meets the top of the first line's letters; the lines beside it are narrowed by its width plus 1 mm, so they wrap and justify inside the column |
| Tabs | every 12.7 mm (0.5 in) |
| Text on path | distance 0, offset 0, orientation "rotate letters" |

## Property bar

Text tool, and the Pick tool with only text selected (after the object
fields): the font list (below), the size in points (a field and an arrow
dropping 6, 7, 8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 40, 48,
60, 72, 96, 144 and 288), Bold, Italic, Underline, the alignment (a
button with the alignment's picture dropping Left, Center, Right and
Full justify), Bullets and Drop Cap (for paragraph text; they open their
dialogs), Text Properties (the Text docker, Ctrl+T) and Edit Text
(Ctrl+Shift+T). Each change applies to the
selected text objects at once (every span takes the font, size and
style) as one "Text Style" undo step, and becomes the default for the
next text. The alignment goes to the paragraphs (see Paragraph
formatting below) as one "Text Alignment" step. Text typed at a point
lines up about that point, so a new alignment (from the bar, the Text
docker or the keys) moves the object back so that the reference point
of the X and Y fields (the 3 x 3 selector) stays where it was, in the
same step; text in a frame keeps its frame and realigns its lines inside
it, text on a path keeps its path. While text is being typed, Ctrl+L
aligns left, Ctrl+E centres, Ctrl+R aligns right, Ctrl+J justifies,
Ctrl+H forces justification and Ctrl+N turns alignment off: for text
typed at a point the whole text lines up, for text in a frame the
paragraphs at the caret or in the selection; when no text is being
typed the same keys keep their other jobs (Export, Combine, Repeat and
so on). Checks: `text_wrap_tests::a_new_alignment_keeps_the_reference_point_in_place`,
`text_wrap_tests::frame_text_keeps_its_frame_when_the_alignment_changes`,
`pick_drag_tests::alignment_keys_while_typing`. Double-clicking text
with the Pick tool edits it in place.

## Right-to-left and mixed text

- Each paragraph runs one way: from its first strong letter (a Hebrew or
  Arabic first letter runs right to left) unless set to Left to right or
  Right to left. Within it, runs of the other direction, numbers,
  punctuation and spaces are ordered by the Unicode Bidirectional
  Algorithm (UAX #9): each run is shaped in its own direction, and each
  line is put in visual order when it is drawn (rules L1 and L2: spaces
  at the line end and before tabs take the paragraph's direction, then
  runs are turned round from the highest level down). Line breaking,
  justification and hyphenation work on the characters in reading order.
- Setting the direction: the Left to right and Right to left buttons on
  the text bar and in the Text docker's paragraph row, or while typing
  Left Ctrl + Left Shift (left to right) and Right Ctrl + Right Shift
  (right to left), both keys of one side pressed and let go with no
  other key between (on Windows, where the system tells the keys' sides;
  elsewhere the buttons). Right to left also aligns right a paragraph
  that was aligned left or not at all, and left to right aligns left one
  that was aligned right. Text typed at a point takes it as a whole,
  text in a frame in the paragraphs at the caret. It is one "Text
  Direction" undo step and keeps the reference point of the X and Y
  fields in place, like a new alignment.
- The caret before a character is on its leading side (the right of a
  right-to-left letter). A click on a letter puts the caret on the
  nearer side of it. Left and Right move on screen on lines with
  right-to-left text (Left always goes left), and one character at a
  time on left-to-right lines; Shift extends the selection. A selection
  is highlighted over every selected character, in several pieces where
  a line mixes directions; Ctrl+A highlights all the text.
- The direction is saved with the paragraph in `.tdraw` files (only when
  it is not automatic, so older files open unchanged and run by their
  first letter). Exports to SVG and PDF draw the text as outlines, in the
  order shown. `.cdr` files keep the text; the direction setting is not
  written to them.
- Checks: `tracedraw_text::tests::hebrew_and_english_on_one_line_are_in_reading_order`,
  `tracedraw_text::tests::the_paragraph_direction_can_be_set`,
  `tracedraw_text::tests::hebrew_carets_and_selections`,
  `tracedraw_text::tests::nikud_and_wrapped_mixed_text`,
  `text_wrap_tests::select_all_highlights_all_hebrew_text`,
  `text_wrap_tests::arrows_move_on_screen_in_hebrew`,
  `text_wrap_tests::the_direction_is_set_kept_and_saved`,
  `direction_keys::tests::both_keys_of_one_side_set_the_direction_when_let_go`,
  `direction_keys::tests::another_key_or_mixed_sides_cancel_it`.

## Font list

The family is picked from a font list, the same in the property bar, the
Text and Glyphs dockers and Options (default font):

- A field with the family and an arrow. A click in the field selects the
  name, so typing replaces it, and opens the list; typing narrows the
  list to the families holding the letters typed, ignoring case: the name
  itself first, then names starting with them, then the rest. Down and Up
  move through the list, Enter takes the highlighted family, Escape
  closes the list and puts the name back.
- At the top of the list, the number of families shown and what to show:
  all fonts, installed fonts, the fonts in this drawing, or (in the
  browser) the fonts to download.
- "Used last": the last five families picked in any font list, newest
  first (kept in the preferences, `recent_fonts`), while nothing is typed.
- Every family, its name drawn in its own font (in the interface font for
  a symbol font and for a family not here yet), with an icon of how it is
  built: T TrueType, O OpenType (PostScript outlines), V variable, D came
  with the drawing, a downward arrow downloaded the first time it is used
  (the tooltip says which). The family in use is marked and brought into
  view. A family with more than one face has an arrow that shows its
  styles under it (Regular, Italic, Bold, Light ..., each drawn in itself);
  picking a style sets Bold and Italic as its weight (600 or more is bold)
  and slant say.
- The sample size: a slider from 12 to 40 points (18 at first, kept in
  the preferences, `font_sample_px`); the rows grow with it.
- The preview: "AaBbYyZz" in the family (and style) under the pointer,
  else the one the keys reached, else the family in use, between its top,
  baseline and bottom lines, with "Family (Style)" under it.
- In the browser, resting the pointer on a family to download for 0.4 s
  fetches it, and its row and preview then draw in it.

Names are drawn by the text engine into pictures of the screen's
resolution, only for the rows in view, and kept until fonts are added.

## Editing in place

Clicking text with the Text tool (or double-clicking it with the Pick
tool) edits it where it is. The caret is a character position in the
concatenated spans; the layout's line boxes give its place on the page.

| Input | Behaviour |
|---|---|
| Typing, paste | inserts at the caret, replacing the selection, with the style of the character before the caret |
| Backspace / Delete | the selection, else the character before / after the caret |
| Enter / Shift+Enter | paragraph break / line break (U+2028) |
| Tab | a tab character |
| Left / Right, Ctrl+Left / Right | one character, one word; Shift extends the selection |
| Up / Down | the same x on the previous / next line (the x is remembered across moves) |
| Home / End, Ctrl+Home / End | line start / end, text start / end |
| Ctrl+A | select all |
| Ctrl+B / I / U | toggle bold, italic, underline on the selection, or on the whole text without one |
| Ctrl+C / X / V | copy, cut, paste the selected characters |
| Click / Shift+click / drag | place the caret / extend / select |
| Double-click | select the word |
| Esc | finish editing and return to the Pick tool; an empty text is removed |

The property bar's font, size and style buttons act on the selected
characters while a selection exists, and show the style at the caret
(the character before it). On a selection a button changes only what it
changes from the style shown: a new size keeps the selected characters'
fonts and styles, a new font keeps their sizes. All the keystrokes of
one editing session are one "Edit Text" undo step.

### Character colours

- With characters selected while editing (artistic or paragraph text),
  a colour picked anywhere goes to them only: a palette or document
  palette swatch (click: their fill; right click: their outline), the
  Properties docker's fill, the Text docker's fill and the Color
  docker. Without a selection the colour goes to the whole object as
  usual.
- Each such change is its own undo step ("Character Fill", "Character
  Outline"), not part of the typing, and the colour joins the document
  palette in that step.
- A character outline starts from the characters' own outline, else
  the object's, else the default pen, in the new colour; No Color (an
  outline of width 0) takes the outline off those characters only.
- Characters without their own fill or outline take the object's. The
  text is drawn (on the canvas, in exports to bitmaps, SVG and PDF) in
  one part per look: each part lays out the whole text and draws only
  its own characters, so letters keep their places. Character
  backgrounds are drawn once under all parts; the drop shadow comes
  with the first part.
- Colours of characters read from `.cdr` files (the fill of each run of
  characters) are drawn the same way.

**Input methods** (Chinese, Japanese, Korean and the others): while text
or a table cell is edited on the canvas, the system's input method is
switched on at the caret (its candidate window opens there). The text it
composes shows underlined at the caret, keys go to it while it composes,
and what it commits is inserted like typed text. A focused field of a
docker or bar keeps its own input method. Letters the text's font lacks
are drawn from an installed font that has them (Noto CJK and the
system's CJK fonts), for each character
(`text_effect_tests::an_input_method_composes_then_commits_cjk_text`,
`text_effect_tests::the_canvas_asks_for_the_input_method_while_text_is_typed`).

**Break Apart** (Ctrl+K) on artistic text makes separate text objects of
its lines when it has more than one line, else of its words, else of its
letters (a text without spaces, like CJK text, breaks into its
characters). Each piece keeps the text's look (font, size, styles, fill,
outline, transformation) and sits where its letters were; the first
piece keeps the object, the others go just above it, all selected, in one
"Break Apart" undo step. A single letter, paragraph text and text on a
path stay as they are
(`text_effect_tests::break_apart_splits_text_into_lines_words_then_letters`).

Text takes blends (with another text or any object) and extrusions like
any other object; the steps and sides follow the letters' outlines
(`text_effect_tests::the_blend_tool_blends_two_texts_and_a_text_with_a_shape`,
`text_effect_tests::text_can_be_extruded`).

## Paragraph text frames

- A paragraph text's bounds are its whole frame (with the letters, when
  they run over it): the selection handles sit on the frame, a click
  anywhere in it picks the text, and alignment and snapping use it.
- Frames show as dashed lines: grey, blue for the selected texts and the
  one being edited, red with a small tab holding a down arrow under the
  bottom edge's middle when the text does not fit (the layout says so;
  kept per document revision). The lines that do not fit are not drawn,
  printed or exported; they show again when the frame grows, and flow
  into the next frame of a linked chain. Options > Text > Show Text Frames (on by default, kept between
  sessions) shows every frame; off, only the selected and edited ones
  show.
- Scaling a paragraph text with the Pick tool's handles, or with the
  property bar's size and scale fields, changes its frame and the text
  flows anew in it; the letters keep their size and the text's transform
  stays as it was (`text_frames.rs`). While the handle is dragged the
  frame follows and the letters stay. This holds for text that is
  neither turned nor slanted and a scale by positive amounts; turning,
  slanting, mirroring and moving act on the whole text, and a turned
  text scales as a whole. With other objects in the selection each is
  scaled as usual, all in one undo step.

## Paragraph formatting

- Alignment, line spacing, character and word spacing, space before and
  after, the first-line, left and right indents, bullets, the drop cap,
  tab stops and hyphenation belong to each paragraph. Columns, vertical
  alignment, Fit Text to Frame, the baseline grid and the frame
  background belong to the frame.
- A text object keeps one alignment and paragraph style of its own,
  which every paragraph has unless it was given its own. A paragraph with
  its own formatting carries it on its characters
  (`TextSpan::paragraph`, `crates/core/src/paragraphs.rs`); the format of
  its first character rules it, and an empty last paragraph follows the
  one before it. Typing at the start of a paragraph keeps its format;
  Enter splits a paragraph into two with the same format; joining two
  paragraphs keeps the first one's.
- While a text is edited, the property bar's alignment, the Text
  docker's Paragraph and Frame fields and the Tabs, Bullets and Drop Cap
  dialogs act on the paragraphs the selection touches (the one holding
  the caret without a selection) and show the values of the first of
  them. With the whole object selected they act on every paragraph and
  show the first paragraph's values. Only the settings changed reach
  the paragraphs: changing the left indent of a whole text keeps each
  paragraph's alignment and spacing. Each change is one undo step.
- A paragraph set back to the object's formatting stops carrying its
  own. `.tdraw` files write `paragraph` only on characters whose
  paragraph has its own formatting; files without it give every
  paragraph the object's formatting.
- `.cdr`: artistic text is written with each paragraph's alignment
  and, from version 16, its line spacing. Paragraph text goes out as one
  artistic text a line, each placed where its paragraph's spacing and
  indents put it, so the look is kept but the spacing values are not
  (the frame records of paragraph text are not known well enough to
  write; see `docs/cdr-format.md`). Reading, paragraphs whose alignment,
  line spacing, space before and after (version 16+ style strings) or
  indents differ from one another keep their own; text whose paragraphs
  agree keeps one object-level format as before.

## Text docker

Ctrl+T, the property bar's Text Properties button and Window > Panels >
Text. Three sections, Character, Paragraph and Frame, each under a header
bar with a triangle that opens or closes it (the ones closed are kept in
the preferences, `text_sections_closed`).

- The three buttons at the top ("A", paragraph lines, a frame) show the
  docker from that section on: Character shows all three, Paragraph the
  Paragraph and Frame sections, Frame only the Frame section (kept as
  `text_docker_from`).
- The two buttons at the right: Apply to default text (a box with an
  arrow) gives new text the font, style, size, underline and alignment
  of the selected text, and offers its fill or outline as the default
  through the usual question about new objects; the eye shows boxes at
  the right of the rows, and while they are shown only the ticked rows
  are taken (`text_docker_checks`, `text_docker_ticked`).
- Character: the font list across the docker; the style list (Normal,
  Italic, Bold, Bold-Italic) and the size list; range kerning (-50 to
  200 %) and the underline button; the text fill, the characters'
  background and the text outline, each as a kind (No Fill or Uniform
  Fill; none, hairline or a width in points for the outline), a colour
  and "..." for the Properties docker's full fill or outline settings.
  The background is drawn behind each character's box, from its descent
  to its ascent (`TextSpan::background`). The expander under the section
  shows the characters' offsets and angle, strikethrough and the
  OpenType features.
- Paragraph: the gear opens Custom Optional Hyphens; six alignment
  buttons, no alignment (lines start at the left; characters keep their
  offsets), left, centre, right, full justification and forced
  justification (the last line of each paragraph is spread too); line
  spacing with its unit (% of the character height, points, or % of the
  font size; a change of unit keeps about the same spacing); left,
  first-line and right indents and space before and after paragraphs in
  the ruler unit. The expander shows character and word spacing.
- Frame, for paragraph text: the frame's background colour (on or off
  and its colour; drawn behind the text inside the frame), the vertical
  alignment as a list (top, middle, bottom), Fit Text to Frame, columns
  (1 to 8) and the gap between them (greyed for one column). The
  expander shows Bullets, Drop Cap and Columns (their dialogs) and
  Hyphenate. With artistic text the section says it is for paragraph
  text.
- Every field edits the selected text: the characters selected while
  editing (the paragraphs they touch for the Paragraph and Frame fields,
  see Paragraph formatting), otherwise every selected text object, one
  undo step each.
  With no text selected the font, style, size, underline and alignment
  become those of new text.
- Labels show without a trailing colon.

## Glyphs docker

Window > Panels > Glyphs. From the top:

- Recent glyphs: the glyphs inserted last, newest first (up to 24, kept
  in the preferences as font and glyph); empty, it says "Shows the most
  recently used glyphs". A click inserts one again in its own font.
- The font list (every family installed or carried by a drawing).
- The character range: Entire Font, Basic Latin, Latin-1 Supplement,
  Latin Extended, Greek, Cyrillic, Hebrew, Arabic, Punctuation, Numbers,
  Currency, Arrows, Mathematical Operators, Symbols.
- A grid of large cells with the glyphs the font draws in that range
  (from its character map), scrolled row by row. The tip names the
  character and its code point. A small arrow under a glyph means the
  font has alternates for it (stylistic alternate, swash, small
  capitals, capitals to small capitals, stylistic sets 1 to 5: the
  features whose shaping gives another glyph); it lists them.
- A click inserts the glyph at the caret of the text being edited, in
  the docker's font (an alternate with its OpenType feature on), else as
  a new artistic text at the page centre.

## Character offsets, angle and spacing

- Each character can be shifted and turned: Horizontal character offset
  (percent of the font size, positive right), Vertical character offset
  (percent of the size, positive up; stored as the baseline shift in
  points) and Character angle (degrees, counter-clockwise, about the
  character's origin on its shifted baseline). The characters after a
  shifted one keep their place. The fields are in the Text panel's
  Character section and on the Shape tool's property bar; they act on
  the selected characters while editing, on the chosen character nodes
  with the Shape tool, else on every character of the selected texts.
- Paragraph: Character spacing (percent of the width of a space, -100 to
  2000, added after every character) and Word spacing (percent of the
  space's own width, 0 to 2000), besides line spacing.
- Text > Straighten Text puts the characters back (no offsets, no
  angle) and takes text fitted to a path off it; Text > Align to
  Baseline (Alt+F12) removes the vertical offsets only.

## Shape tool on text

- A node at the lower left of every character, on its (shifted)
  baseline; newlines have none. Click chooses one, Ctrl+click adds or
  removes, Shift+click adds, a marquee (rectangular or freehand) chooses
  the nodes inside. Chosen nodes are filled.
- Dragging a node moves the chosen characters (the one dragged when it
  was not chosen); Ctrl keeps the move horizontal or vertical. The move
  becomes their offsets (layout space, so rotated text moves along its
  own axes). One undo step per drag ("Move Characters").
- The Interactive horizontal spacing arrow, below the lower right
  corner, changes the character spacing so the end of the widest line
  follows the pointer: `+dx / n` per character for `n` characters on
  that line. With Shift it changes word spacing: `+dx / spaces` per
  space. The Interactive vertical spacing arrow, below the lower left
  corner, changes line spacing so the last baseline follows the pointer:
  `leading' = leading * (pitch - dy / (lines - 1)) / pitch`.
- The arrows act on every paragraph of the text, those with spacing of
  their own too, so they keep their differences: the character or word
  spacing change (in percent) is added to each paragraph's, and each
  paragraph's line spacing is multiplied by the same factor. One undo
  step per drag.
- Text fitted to a path shows no character nodes.

## Placeholder text

Text > Insert Placeholder Text fills each selected paragraph text frame
with filler text (the traditional pseudo-Latin passage, cycled), as much
as the frame holds without overflowing: the number of words is found by
doubling, then halving the gap, with a full layout each time (at most
20000 words). The filler takes the style of the frame's first
character, or the default font and size of an empty frame; paragraphs
end at the first full stop after 60 words and the text ends with a full
stop. Artistic text and other objects are left alone; a text being
edited is finished first. One undo step.

## Fonts that come with a drawing

A drawing can carry font files: the fonts embedded in a `.cdr` file it
was opened from (see `docs/cdr-format.md`, Embedded fonts), saved with it
in `.tdraw` files. When such a drawing opens (or a `.cdr` file is
imported), each carried font whose family is not installed serves the
text for the rest of the session and joins the font lists; an installed
font of the same family wins, since an embedded copy may hold only the
letters the drawing used. A font is used only when its licence allows
use in documents (the OS/2 embedding bits: not "restricted", outlines
allowed); others are skipped with a log line and their text falls back
like any missing font. The Fonts docker marks carried families
"embedded" under "Fonts in document". Saving a drawing as `.tdraw` adds
the carried fonts its text uses (text imported or pasted from another
drawing), so the file keeps its letters on machines without them.

The missing-font check (status bar, Font Manager, preflight) looks at
text everywhere in the drawing: on layers, in groups and clip frames,
in table cells and in symbols; a family is missing when it is neither
installed nor carried.

Text set in the Symbol family keeps the codes of that font's symbol
encoding (as drawings store it: "a" for alpha, 0xC5 for a circled
plus). When no installed face of that name maps those codes (a font
with a symbol character map), the codes draw as the characters they
stand for, from the public table of that encoding (Greek letters,
arrows, set and logic signs, bracket pieces), each through a face that
has it.

## Family names

A font is found by the family name its file lists (the typographic
family, "Arial") and by its legacy family name when that differs
("Arial Narrow", "Arial Black", "Segoe UI Semibold", the name a drawing
from another program usually gives); the font lists show both. A
legacy name picks the face of that name nearest the text's weight and
style.

A drawing that carries several faces of one family (regular and bold)
has each face serve its text; an installed face of the same family,
weight and style wins over the carried one.

## Missing fonts

Text set in a family that is neither installed nor carried keeps its
font name (saving writes it back unchanged) and is drawn with a stand-in,
the first installed of:

1. Fonts drawn to the same widths, so lines break where they did:
   Liberation Sans, Arimo, Nimbus Sans or TeX Gyre Heros for Arial and
   Helvetica; Liberation Serif, Tinos, Nimbus Roman or TeX Gyre Termes
   for Times and Times New Roman; Liberation Mono, Cousine, Nimbus Mono
   or TeX Gyre Cursor for Courier and Courier New; Liberation Sans
   Narrow for Arial Narrow; Carlito for Calibri; Caladea for Cambria;
   Gelasio for Georgia; Selawik for Segoe UI; TeX Gyre Pagella for
   Palatino and Book Antiqua, Bonum for Bookman, Schola for Century
   Schoolbook, Adventor for Century Gothic and Avant Garde, Chorus for
   Zapf Chancery (or their URW counterparts); DejaVu Sans for Verdana
   and Tahoma; Comic Neue for Comic Sans; EB Garamond for Garamond;
   DejaVu Sans Mono for the programmers' faces.
2. Fonts of the same style, judged by the name: monospaced ("mono",
   "courier", "code", ...), serif ("serif", "roman", "garamond",
   "bodoni", ... and not "sans" or "gothic"), casual ("comic",
   "marker", ...), else sans-serif.
3. DejaVu Sans, Arial, the system sans-serif, any font.

Within a run, letters the face lacks are drawn with a face that has
them: the fonts made for their script first (Noto Sans CJK, WenQuanYi
and the system faces for Chinese, Japanese and Korean; Noto Sans Arabic,
Hebrew, Thai, Devanagari and Bengali; DejaVu Sans and Noto Sans for the
rest), then any installed face that has the letter, upright faces of the
run's weight first. Spaces, controls and combining marks stay with the
letter before them. A letter no face has is drawn with the run's face.

Variable fonts draw at weight 400, or 700 for bold, on their weight
axis (within its range), and slant on their italic or slant axis for
italic when the face is upright; only what no axis gives is made
synthetically (bold by drawing the letters twice, italic by slanting
them).

The browser build offers the families of its font folder (see
`web/FONTS.md`): the font lists show them, and text asks for one the
first time it needs it (text set in it, in a family it stands in for,
or holding letters of a script only it has); the page fetches the files
and draws again when they arrive.

Names are compared in lower case without spaces, punctuation, style
words and foundry endings ("Arial-BoldMT" and "Arial MT" are Arial,
"TimesNewRomanPSMT" is Times New Roman), so names written by PostScript
and PDF producers match too. The status bar, the message on opening, the
Font Manager and the Fonts docker (on the "missing" mark) name the
stand-in next to each missing family: "Arial (Liberation Sans)".

## Formulas

- Line height `= size * leading / 100 * (asc - desc + gap) / upm`, each
  line from the tallest letters in it (a big first letter drops the
  first line only).
- Justify: extra space `= (frame_width - line_width) / n_spaces`.
- Text on path: the baseline is the path's first subpath (followed from
  its end back to its start when mirrored), moved `distance` to its left,
  towards the letters' tops (corners mitred, at most 4 times the
  distance out). Each glyph is centred at arc length `start + s` along
  the baseline, where `s` is the glyph's x-advance midpoint and `start`
  is `offset`, plus `(length - width) / 2` for centred text or
  `length - width` for right-aligned text, and turned to the baseline's
  direction there. On a closed path positions wrap round past the start;
  on an open one they stop at its ends.
- Text on path, its letters: the line of the letters put on the path is
  the baseline, the ascender (shifted down by the ascent), the descender
  (shifted up by the descent) or the centre between them. Rotated letters
  turn to the path's direction; upright letters do not turn; vertical
  skew shears each letter so its width follows the path's slope and its
  upright strokes stay vertical; horizontal skew keeps horizontal strokes
  level and leans the upright ones along the path's normal (the slope's
  horizontal part is kept at least 0.2 so steep stretches do not flatten
  a letter). Mirror vertically flips the letters to the other side of
  the path, upside down; mirror horizontally reads the text from the
  path's end, letters mirrored; both together turn the text half round
  onto the other side, reading the other way (the older single mirror).
- Fit text to frame: scale size so the paragraph fills the frame height.

## Checks

- Character colours: `text_editing::tests::a_colour_picked_with_characters_selected_colours_only_them`,
  `text_editing::tests::without_a_character_selection_the_colour_goes_to_the_object`,
  `text_editing::tests::the_bar_changes_only_what_it_changed_on_the_selection`,
  `tracedraw_core` `document::tests::characters_with_their_own_colours_are_drawn_in_one_part_per_look`,
  `document::tests::character_outlines_are_saved_and_layout_only_is_not`,
  `tracedraw_render` `tests::characters_are_drawn_in_their_own_colours`,
  `tests::spans_only_laid_out_keep_their_room_but_are_not_drawn` (text crate).
- The Text docker: `ui::text_docker::tests::the_sections_open_close_and_draw_for_any_text`,
  `ui::text_docker::tests::vertical_alignment_is_kept_with_the_text`,
  `ui::text_docker::tests::section_buttons_boxes_and_apply_to_default`,
  `ui::text_docker::tests::fields_edit_the_selected_text`;
  `tests::forced_justification_spreads_the_last_line_too`,
  `tests::line_spacing_in_points_and_in_percent_of_the_size`,
  `tests::character_backgrounds_cover_their_characters` (text crate);
- The Glyphs docker: `ui::glyphs_docker::tests::ranges_pick_their_characters`,
  `ui::glyphs_docker::tests::a_glyph_goes_in_as_new_text_in_the_dockers_font_and_is_remembered`,
  `ui::glyphs_docker::tests::the_docker_draws`;
  `tests::paragraph_text_sits_at_the_top_middle_or_bottom_of_its_frame`
  (text crate). Checked on screen: Bottom moved "Hello world" to the
  frame's bottom edge.
- The font list: typing finds the name, then names starting with it,
  then the rest (`ui::font_list::tests::typing_finds_the_name_then_names_starting_with_it_then_the_rest`);
  the fonts used last keep the newest five
  (`ui::font_list::tests::the_fonts_used_last_keep_the_newest_first`);
  names are drawn in their own font, and a family not installed or no
  text draws nothing (`ui::font_list::tests::names_are_drawn_in_their_own_font`);
  the list draws with every filter, a family's styles shown and typed text
  (`ui::font_list::tests::the_list_draws_in_every_state`); families tell
  how they are built, their style names, whether they came with a drawing
  or are to download (`tests::families_tell_how_they_are_built_and_their_styles`
  in `crates/text`). Checked on screen: the list open, typing "lib" left
  three families, Down and Enter took one, and it showed first among the
  fonts used last the next time.
- `document::tests::text_baselines_wrap_round_closed_paths_and_move_by_the_distance`,
  `document::tests::open_text_baselines_stop_at_their_ends_and_corners_keep_the_distance`
  (core), `tests::mirrored_text_on_a_path_reads_along_its_other_side` and
  `tests::text_on_a_closed_path_continues_past_its_start`,
  `tests::the_letters_line_on_the_path_follows_the_vertical_placement`,
  `tests::single_mirrors_flip_the_letters_and_both_turn_them_round`,
  `tests::upright_and_skewed_letters_keep_their_strokes_straight` and
  `tests::a_text_refitted_to_another_path_of_the_same_kind_is_laid_out_again`
  (text crate: the outline cache keys a fitted text on its whole path,
  not on the path's number of elements).
- `tests::fonts_are_found_by_their_legacy_family_names`,
  `tests::a_drawing_brings_every_face_of_a_family`,
- `tests::missing_fonts_get_stand_ins_with_the_same_widths_or_style`,
  `tests::letters_a_font_lacks_are_drawn_with_a_font_that_has_them`,
  `tests::variable_fonts_draw_bold_with_their_weight_axis` and
  `tests::fonts_the_application_offers_are_asked_for_once_and_then_draw`
  (text crate), `documents::tests::fonts_that_arrive_later_refresh_the_lists`,
  `font_folder::tests::the_font_list_reads_families_and_keeps_files_in_the_folder`
  and
  `prefs::tests::opening_tells_what_was_left_out_as_the_warnings_say`.
- `documents::tests::a_drawing_brings_its_fonts_along`,
  `tests::fonts_a_document_brings_are_loaded_once_unless_installed` and
  `tests::a_font_licence_decides_whether_a_document_may_use_it` (text
  crate), `container::tests::embedded_fonts_come_with_the_document` (cdr
  crate) and `document::tests::carried_fonts_are_saved_and_shared_between_copies`
  (core).
- `text_frames::tests::scaling_paragraph_text_resizes_its_frame_and_keeps_the_letters`
- `text_frames::tests::frames_show_dashed_and_follow_a_scale_being_dragged`
- `text_frames::tests::a_frame_too_small_for_its_text_overflows`

- `text_nodes::tests::every_character_has_a_node_on_its_baseline`,
  `text_nodes::tests::dragging_chosen_characters_shifts_only_them`,
  `text_nodes::tests::a_marquee_chooses_character_nodes`,
  `text_nodes::tests::spacing_arrows_change_character_word_and_line_spacing`,
  `tests::shifted_and_rotated_characters_leave_the_others_in_place` and
  `tests::character_and_word_spacing_widen_the_text` (text crate).
- `tests::drop_cap_spans_its_lines_without_rising_above_the_text`,
  `hyphen::tests::breaks_fall_on_the_pattern_positions` and
  `hyphen::tests::names_and_acronyms_are_not_broken` (text crate).

- Given "ab" at 100 pt, then the width equals the sum of the two
  advances minus the kerning pair from the font.
- Given a paragraph frame narrower than one word, then the word breaks
  by character.
- Given a 40 x 20 mm and a 120 x 90 mm paragraph frame and an artistic
  text selected, when Insert Placeholder Text runs, then both frames hold
  filler that fits (the larger one more), eight more words would
  overflow the larger one, the artistic text is unchanged, and one undo
  brings the old text back.

## Text menu without a selection

- Tabs, Columns, Bullets and Drop Cap with no text selected edit the
  defaults new paragraph text gets (`Document::defaults.paragraph`,
  saved with the drawing, one undo step); their spacing and indents are
  left alone.
- Fit Text to Path with a text and another object selected fits the text
  at once. Otherwise the status bar asks for a click on the path (a
  right-click cancels): the selected text goes on it, or with no text
  selected an empty text starts on the path, ready to type.
- The text itself goes on the path (no copy is left behind) and stays
  selected as one object, drawn in its fill. On an open path the letters
  follow the path from its start with the text's alignment. On a closed
  path the path is followed clockwise, so the letters stand on its
  outside, upright to the curve, from the point clicked (the point of
  the path nearest to the click; its highest point when the text and
  the path were selected together), and the text is centred on that
  point (its alignment becomes centred and the offset half the path's
  length). The Text tool clicked on a path's outline starts a new text
  that begins at the point clicked (on a closed path too) and runs on
  from there as it is typed
  (`ops2::tests::the_text_tool_on_a_paths_outline_starts_text_on_it`).
  A pick on the bottom of an ellipse puts the text there,
  outside and upside down; both mirrors then turn it round onto the
  inside, reading left to right, on the same spot.
- With the Pick tool, a text fitted to a path shows its own property
  bar after the object fields: Text orientation (Rotated letters,
  Vertical skew, Horizontal skew, Upright letters), Vertical placement
  (Baseline, Ascender, Descender on path, Centred on path), Text
  placement (start, centre or end of the path, the text's alignment; on
  a closed path also Top centre and Bottom centre: the text centred on
  the path's highest point, outside and upright, both mirrors off, or
  on its lowest point, inside and reading left to right, both mirrors
  on; the list shows these while the text is so placed,
  `text_path::tests::top_and_bottom_placement_on_a_closed_path`),
  Distance from path and Offset along path in the document's units,
  Tick snapping and its spacing (1 mm by default, kept in the
  preferences: a drag off the path then moves the text away from it in
  whole steps,
  `text_path::tests::tick_snapping_moves_the_text_off_the_path_in_steps`),
  Mirror horizontally and Mirror vertically (pressed while on) and
  Straighten Text. Mirror horizontally flips the letters left to right
  (the text reads the other way along the path), Mirror vertically
  turns them upside down onto the path's other side, and both together
  turn the text round by 180 degrees. The mirrors keep the letters where they are along
  the path: mirrored horizontally the text is measured from the path's
  other end, so its offset turns round and the start placement becomes
  the end one (and back), the centre one staying
  (`ops2::tests::text_goes_where_the_path_is_clicked_and_mirrors_keep_it_there`,
  `ops2::tests::the_horizontal_mirror_keeps_text_on_a_line_in_place`).
  Each change is one undo step and is stored with the
  text, so the bar shows it again when the text is selected again
  (`text_path::tests::the_bar_settings_edit_the_text_with_undo_and_come_back_on_reselect`).
- A selected fitted text draws as usual, in its fill, with the selection
  handles. Dragging it with the Pick tool slides it along the path and
  away from it instead of moving it off the path: while the button is
  held its letters show outlined in blue where they would go, the line
  they would stand on dashed, and the distance and offset by the
  pointer; on release the offset grows by the way the pointer went along
  the path (the short way round a closed one) and the distance by the
  way it went across, towards the letters' tops
  (`text_path::tests::dragging_moves_the_text_along_and_away_from_the_path`).
- Fit Text to Path with no text selected leaves nothing behind: an empty
  text started on the path and left without typing is removed, a typed
  one is the only text and sits on the path, and choosing the command
  while an empty text is being typed removes that text first
  (`text_path::tests::fit_text_to_path_with_nothing_selected_leaves_no_stray_text`).
- With the Text tool, a click on the outline of a path (within 4 pixels;
  not a text, a bitmap or a group) starts an empty text on that path,
  placed the same way, ready to type. A click elsewhere starts an
  artistic text as before
  (`ops2::tests::closed_paths_turn_clockwise_and_start_at_the_top`,
  `ops2::tests::text_fitted_to_an_ellipse_sits_centred_on_its_top_outside`,
  `ops2::tests::text_on_a_straight_line_stands_on_it_from_the_start`,
  `ops2::tests::the_text_tool_on_a_paths_outline_starts_text_on_it`).
- Change Case opens a dialog with Sentence case, lowercase, UPPERCASE,
  Title Case and tOGGLE cASE; it needs a text selected.

## Font Sampler

Text > Font Sampler (a panel) compares fonts: a line of sample text set
in each chosen font, one under the other, on white cards.

- The sample is what is typed in the field at the top, or the pangram
  in the interface language when it is empty.
- Size: 10 to 96 pixels to the em (28 by default); a sample longer than
  the panel is cut off at its edge.
- The first time it opens it lists the text font and the fonts of the
  drawing. Add font: type part of a name and click one of the first
  eight matches; at most 24 fonts are listed, each once. ✕ removes one.
- A font that is not installed shows in its stand-in, and says which.
- A click on a sample gives the selected text that font (or makes it the
  text font when nothing is selected); the text font is ticked.
