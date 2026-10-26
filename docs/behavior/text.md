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
| Hyphenation | off; when on, at least 2 letters before and 3 after a break, from a compact pattern set (a vowel and consonant rule for words it leaves whole); digraphs (ch, ck, gh, ph, sh, th, wh, qu) stay together; words with a capital letter (names, acronyms) are never broken. Options > Text > Paragraph Text > Hyphenation patterns picks a hyphenation dictionary instead (`hyph_*.dic`, found where the spelling dictionaries are, see `writing-tools.md`): its first line names the character set; `LEFTHYPHENMIN` and `RIGHTHYPHENMIN` set the minimums (else 2 and 3); `%` and `#` lines are comments; `NEXTLEVEL` starts a second level of patterns, a word breaking where either level allows; patterns with a replacement (`/`) are left out. With a dictionary a capital first letter no longer stops a break (nouns in some languages); other capitals still do. A dictionary that is gone at start-up leaves the built-in patterns and a status message (`hyphen::tests::pattern_files`, `spell::tests::real_hyphenation_files`, ignored, over English, Brazilian Portuguese, German and Italian files: hy-phen-ation, com-pu-ta-dor, Haus-tür, Schiff-fahrt, in-for-ma-zione). Punctuation before or after a word (a comma at a line's end, quotation marks) is left aside and the letters between break; an apostrophe inside a word (' or ’) is kept for the patterns, and without a pattern file only the part after the last apostrophe breaks (#255, `hyphen::tests::punctuation_round_a_word_and_apostrophes_inside_it`). Text > Use Hyphenation turns hyphenation on or off for the selected texts (off when they all have it); while a text is edited it is the setting of the paragraphs at the caret, its check following them; with no text selected it is the setting new text takes (`writing_tools::tests::use_hyphenation_acts_on_the_selected_text`). Typing keeps a text's own hyphenation; it took the setting for new text, so editing a text turned its hyphenation on or off (#470, `writing_tools::tests::editing_a_text_keeps_its_hyphenation`) |
| Chinese and Japanese line breaks | text written without spaces breaks between any two characters when one of them is Chinese or Japanese (ideographs, kana, full-width forms and their punctuation), except before a character that may not start a line (，。、；：？！ and their half-width forms, closing brackets and quotes such as ）」』】”’, iteration marks, ー and small kana) and after one that may not end a line (opening brackets and quotes such as （「『【“‘, and ￥ ＄ £ before a sum); the character before goes down to the next line with it. Latin text keeps breaking at spaces and hyphenation points (#847, `tests::cjk_punctuation_never_starts_a_line`) |
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
next text. The size is the size as seen: a text scaled with the Pick
tool shows its font size times its vertical scale (24 pt doubled reads
48 pt), and a size typed for it is the size it then shows (the
characters keep it before the text's scale); a field left as it was
leaves each character's size alone. The alignment goes to the paragraphs (see Paragraph
formatting below) as one "Text Alignment" step. Double-clicking text
with the Pick tool edits it in place.

## Vertical text

Text > Vertical Text (a check item) and the Horizontal text and
Vertical text buttons after the alignment on the property bar set the
selected text objects (and the one being edited) in columns, as CJK
typesetting does, or back in lines; one "Vertical Text" undo step
(`App::set_vertical_text`, `ParagraphStyle::vertical`, saved with the
drawing). They show with a Japanese or Chinese interface language, and
for any text whose letters stand upright in columns (ideographs, kana,
hangul; `App::vertical_text_offered`, `tracedraw_text::upright`).

- The written lines become columns read top to bottom and placed right
  to left, each column a line's height apart (the line spacing). Artistic
  text starts at its origin and goes down and left; paragraph text
  starts at the top right of its frame, its columns running down the
  frame's height and across its width (so the frame's width holds the
  columns and its height the line length; linked frames, columns and
  the gutter apply as to lines).
- Ideographs, kana, hangul, their punctuation and brackets, full-width
  and enclosed forms and pictorial symbols stand upright, centred on the
  column, each advancing by its width (the Unicode vertical orientation,
  `upright`); the font's vertical forms (the `vert` feature) give the
  brackets and punctuation their column shapes. Latin, Greek, Cyrillic
  and Arabic letters, digits and the other characters lie on their side,
  turned a quarter clockwise, their tops to the right of the column.
  Underlines, strike lines and hyphens turn with the text.
- The text engine lays the text out as written lines (in a frame turned
  on its side) and turns the glyphs into columns as it places them
  (`layout_scaled`, `orient`); `TextLayout::lines` and `baselines` keep
  describing the written lines and `TextLayout::line_frame` turns them,
  so carets, selections, clicks, the character nodes of the Shape tool,
  snapping to baselines and the text box follow the columns
  (`text_editing::layout_of_text_shape` composes the turn into the
  text's transform). The caret is a bar across the column; while editing,
  Down and Up move along the column, Left and Right to the next and
  previous column.
- Vertical alignment in the frame moves the columns sideways by the
  room left. Text fitted to a path, and text wrapping around objects,
  stay horizontal. Files of other formats hold the text as written lines
  (`.cdr`, SVG as curves); `.tdraw` keeps the flag.

## Font list

The family is picked from a font list, the same in the property bar, the
Text and Glyphs dockers and Options (default font):

- A field with the family and an arrow. A click in the field selects the
  name, so typing replaces it, and opens the list; typing narrows the
  list to the families holding the letters typed, ignoring case: the name
  itself first, then names starting with them, then the rest. Down and Up
  move through the list, Enter takes the highlighted family, Escape
  closes the list and puts the name back.
- The mouse wheel over the closed list (the field or its arrow) takes
  the family after (wheel down) or before (wheel up) the one in use, in
  the list's order, stopping at the ends; the families used last stay as
  they were. In the property bar and the Text docker it acts only on text
  being edited or selected: with nothing to change it does nothing
  rather than ask for the font of new text (#665,
  `ui::font_list::live_tests::the_wheel_over_the_font_list_steps_the_font`).
- At the top of the list, the number of families shown and what to show:
  all fonts, installed fonts, the fonts in this drawing, or (in the
  browser) the fonts to download.
- "Used last": the last five families picked in any font list, newest
  first (kept in the preferences, `recent_fonts`), while nothing is typed.
- Every family, its name drawn in its own font in the theme's text
  colour, light on the dark themes (#660; in the interface font for
  a symbol font and for a family not here yet), with an icon of how it is
  built: T TrueType, O OpenType (PostScript outlines), V variable, D came
  with the drawing, a downward arrow downloaded the first time it is used
  (the tooltip says which). The family in use is marked and brought into
  view. A family with more than one face has an arrow that shows its
  styles under it (Regular, Italic, Bold, Light ..., each drawn in itself);
  picking a style sets Italic as its slant says, and Bold for a bold face
  (700, or a weight from 600 the style does not name). A face of another
  weight (Thin, Light, Medium, SemiBold, Black ...) is named by the family
  with its weight, not bold: SemiBold of "Titillium Web" sets "Titillium
  Web SemiBold", as a .cdr file names that face, so the bars show it and
  never Bold (#892). With no family row of that whole name, the family's
  row is the one marked.
- The sample size: a slider from 12 to 40 points (18 at first, kept in
  the preferences, `font_sample_px`); the rows grow with it.
- The preview: "AaBbYyZz" in the family (and style) under the pointer,
  else the one the keys reached, else the family in use, between its top,
  baseline and bottom lines, with "Family (Style)" under it.
- In the browser, resting the pointer on a family to download for 0.4 s
  fetches it, and its row and preview then draw in it.
- Live preview in the drawing (#370): while the property bar's or the
  Text docker's list is open, the selected text (the selected
  characters while editing, the selected table cells) shows the family
  under the pointer, or the one the keys reached, as it would take it.
  The property bar's size list does the same with the size under the
  pointer. Moving off the list or closing it shows the text as it was;
  the preview is no undo step and leaves the redo steps and the saved
  state as they were (`Engine::begin_preview`, `withdraw_preview`;
  `the_text_shows_the_font_a_list_points_at`). Options' default font
  list does not preview.
- The font for new text is kept between runs; at start, one that is no
  longer installed gives way to Arial (else Liberation Sans, DejaVu Sans,
  Selawik, or the first font there is), since text typed in it was drawn
  with a stand-in while the bars named the missing font (#504,
  `a_default_font_no_longer_installed_gives_way`).
- On Windows the installed fonts are the ones its font registry lists
  (the machine's and the user's): a file left in a font folder by a font
  uninstalled while in use is not offered (#504,
  `installed::tests::faces_of_files_not_registered_are_left_out`).

Names are drawn by the text engine into pictures of the screen's
resolution, only for the rows in view, and kept until fonts are added.

## Edit Text dialog

Text > Edit Text (Ctrl+Shift+T), the text bar's Edit Text button and the
object's menu open the Edit Text dialog on the first selected text
(#355): its characters in a box of several lines (Enter makes a new
paragraph there), OK and Cancel. OK gives the text the characters of the
box as one undo step ("Edit Text"): the characters both texts share (the
longest common sequence, by character; past 4 million comparisons the
common start and end) keep their formatting, each added run takes the
formatting of the character before it, and linked frames reflow
(`App::open_edit_text_dialog`, `App::set_text_content`; check
`ops2::text_dialog_tests::the_edit_text_dialog_keeps_the_formatting_of_what_did_not_change`,
`ops2::text_dialog_tests::char_edits_keep_what_both_texts_share`).
A double click on a text still edits it in place.

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
| Mixed directions (an Arabic or Hebrew word in English text, or the other way round) | the Unicode bidirectional algorithm gives each character its level, the first strong letter setting the direction; it is worked out for each run of one formatting between tabs and line ends, so a direction change across differently formatted characters keeps them in text order. Each run of one level is shaped its own way; lines break in text order and each line shows its runs in display order: Arabic typed after "Asasa " reads right to left, joined, to the right of it, and a paragraph too narrow for "Hi مرحبا بك" keeps "Hi مرحبا" on the first line; digits inside right-to-left text read left to right (#628, `tracedraw_text::tests::mixed_text_is_cut_into_runs_of_one_direction`, `tracedraw_text::tests::lines_are_seen_in_bidi_order`, `tracedraw_text::tests::mixed_directions_lay_each_run_its_own_way`, `tracedraw_text::tests::mixed_paragraph_text_breaks_in_text_order`, `pick_drag_tests::right_to_left_typed_after_english_runs_right_to_left`) |
| Right-to-left text (Arabic, Hebrew) | the caret before a letter is at its right end and the end of the text at its left; a selection shades the letters it holds, all of them after Ctrl+A, where it used to shade only the first letter (#629, `tracedraw_text::tests::right_to_left_carets_run_from_the_right`, `pick_drag_tests::select_all_in_right_to_left_text_highlights_all_of_it`); character backgrounds and a callout's box cover such text too, mixed directions included (`callouts::tests::text_boxes_and_backgrounds_cover_mixed_direction_text`) |
| Ctrl+B / I / U | toggle bold, italic, underline on the selection, or on the whole text without one |
| Ctrl+C / X / V | copy, cut, paste the selected characters |
| Click / Shift+click / drag | place the caret / extend / select |
| Double-click | select the word |
| Esc | finish editing and return to the Pick tool; an empty text is removed |
| Ctrl+Space | finish editing and switch to the Pick tool (Space alone types a space) |
| Undo of typing | the typing of one editing session of one text is one step ("Edit Text"); a session folds keystrokes only into its own step, while it is the last one, so editing another text never touches it (#373, `text_editing::tests::editing_a_second_text_keeps_the_edit_of_the_first`) |
| Typing while a field has the keyboard (the font search field, a docker's box) | goes to the field only; the text waits until the field lets the keyboard go (#342, `pick_drag_tests::typing_in_a_focused_field_leaves_the_edited_text_alone`) |

The caret is a black bar on a white edge, so it shows on the white page
and on dark fills whatever the interface theme; selected characters are
shaded a see-through mid grey, coloured letters keeping their colour
under it. Table cells being edited show the same caret and shade (#632,
`pick_drag_tests::the_text_caret_is_black_on_a_white_edge`,
`text_frames::tests` checks the shade).

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
- While characters are selected, the Properties docker's Fill and
  Outline sections show those characters' own fill and outline (the
  object's where they have none) and edit them alone, and the Character
  section shows the span of the first selected character
  (`App::text_selection_fill`, `text_selection_stroke`,
  `text_selection_span`,
  `text_editing::tests::the_selected_characters_show_their_own_fill_and_outline`).
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
Text with an envelope, perspective or distortion breaks instead into
one curve per letter (an accent stays with its letter, underlines and
strikethroughs are one more curve), each warped as the whole text was
(a twister turns each letter by its distance across the whole text) and
keeping its holes, a single letter too, in one "Break Apart" step that
Undo takes back to the text with its envelope. Several texts broken
together keep their stacking order
(`text_effect_tests::break_apart_bakes_the_envelope_into_letter_curves`,
`text_effect_tests::break_apart_keeps_the_hole_of_a_single_warped_letter`,
`text_effect_tests::breaking_several_warped_texts_keeps_the_stacking_order`).

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

### Join Lines into Paragraph

Text > Join Lines into Paragraph (and the object menu, two or more
artistic texts selected) makes the selected lines one paragraph text, for
text that came one object per line (a PDF's): typing into it flows the
words on to the next lines. The lines go in reading order, top to bottom,
joined by a space; a gap between baselines more than half again the usual
one starts a new paragraph. The frame is as wide as the widest line (and
a little more), with room for two more lines; the first baseline stays
where it was and the line spacing is the usual distance between the
baselines, in points. The lines must be turned and sized alike, in the
same group, else the status bar says so and nothing changes. One undo
step; the first line's object keeps its fill and outline (#599,
`ops2::tests::lines_of_text_join_into_a_paragraph_that_flows`). Hindi
lines joined this way flow the same way, breaking only between syllables
(`devanagari_join_tests::joined_hindi_lines_flow_and_break_between_syllables`).
Lines kept as separate objects do not flow into each other: each grows
on its own line, which is why the lines are joined first.

## Linked frames

- Text > Paragraph Text Frame > Link: with two paragraph frames
  selected, the first flows into the second; the chain's text lives in
  its frames in order and overflow runs on into the next frame after
  typing, linking and resizing (`textflow.rs`, links kept in the object
  data as `text.next` and `text.prev`). Unlink leaves each frame with the
  text it shows.
- The flow tab (#371): a paragraph frame that is picked, or whose text
  does not fit, has a tab under its bottom edge's middle, below the
  selection's bottom handle (a down arrow when the text does not fit,
  two links when the frame flows on, empty otherwise). A press on a
  handle is the handle's, never the tab's: the tab sat under the bottom
  handle, so dragging a frame taller (one just made by Text > Convert,
  say) started a text link and the next click linked the text to
  whatever it hit (#503,
  `the_bottom_handle_of_a_converted_frame_drags_it_taller`). A click on the tab of a frame that flows into no other
  (Pick tool) waits for the next click, with the target arrow pointer
  and a hint on the status bar: on another paragraph frame, on this page
  or on another one reached meanwhile, the text flows into it; on empty
  space a new frame the size of the first one takes the text, its top
  left corner at the click; a drag makes the new frame the dragged
  rectangle. Esc or a right click cancels
  (`the_flow_tab_links_to_a_new_frame_on_another_page`).
- A copy of a frame (Duplicate, a right click during a move, Paste,
  stamping) is a frame of its own, in no chain: typing in it does not
  flow into the original's next frame. Frames of a chain copied together
  (duplicated, pasted or imported as a set) are a chain of the copies
  (`textflow::relink_copies`).

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
- Character: the font list across the docker; the style list (the
  faces of the family, by their own names: Regular, SemiBold, Bold ...;
  Normal, Italic, Bold, Bold-Italic for a family whose faces are not
  known, and the weight of a family named with one, "SemiBold") and the
  size list; range kerning (-50 to
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
  become those of new text; a font or size picked so first asks
  ("You have not selected anything. Use this setting as the default for
  new text?", OK or Cancel), as the reference program does, since a pick
  made by mistake changed the font of every new text without a word
  (`pick_drag_tests::a_size_picked_with_nothing_selected_asks_before_changing_the_default`).
  The Enter that applies a typed size does not answer the question.
- Labels show without a trailing colon.

## Glyphs docker

Window > Panels > Glyphs. From the top:

- Recent glyphs: the glyphs inserted last, newest first (up to 24, kept
  in the preferences as font and glyph); empty, it says "Shows the most
  recently used glyphs". A click inserts one again in its own font. The
  box takes the theme's field colour (white in the light themes, dark in
  the dark ones), so the glyphs, drawn in the theme's text colour, stay
  readable (#660).
- The font list (every family installed or carried by a drawing): the
  property bar's list, with its search as you type and font previews
  (#447).
- The character range: Entire Font, Basic Latin, Latin-1 Supplement,
  Latin Extended, Greek, Cyrillic, Hebrew, Arabic, Punctuation, Numbers,
  Currency, Arrows, Mathematical Operators, Symbols.
- A search field: a character or characters ("é", "aB") keeps those; a
  code point with a U+ or 0x prefix, or bare hex digits with at least one
  digit ("20AC", "004"), keeps the characters whose code starts with it.
- An info line: the code point, the character and its range of the glyph
  last under the pointer, and the keystroke that types it when code page
  1252 has it ("Keystroke: Alt+0169" for ©, `keystroke`). While text is
  edited, digits typed with Alt held give a character when Alt is let go:
  with a leading zero the code page 1252 number, else the Unicode number
  (`text_editing::alt_code_char`), on every system: on Windows the
  windowing layer drops the character the system makes of them (#469).
  The keypad digits count with Num Lock off too (the physical key).
  The digits typed under Alt are not typed themselves (the windowing
  layer also sends each keypad digit as text with Num Lock on), and the
  character, should the system send it too when Alt is let go, is typed
  once (#447, #469;
  `pick_drag_tests::alt_codes_type_their_character_once_and_not_the_digits`).
  The digits of the top row count as well, which Windows itself does
  not do: there only the keypad types Alt codes.
- A grid of large cells with the glyphs the font draws in that range
  (from its character map), scrolled row by row. A font made for the
  symbol encoding (legacy script and dingbat fonts: a Windows symbol
  character map only, its glyphs at U+F020 to U+F0FF) lists its glyphs
  as the codes typed for them ("A" with Alt+0065 for U+F041), and text
  typed in it draws those glyphs instead of falling to another font
  (#469; `tests::a_symbol_font_draws_the_codes_typed` in the text
  crate). The tip names the
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

- A node at the lower left of every character (of its own glyph, in right-to-left text too, each under its letter in reading order, #474,
  `text_nodes::tests::right_to_left_characters_have_their_nodes_under_their_glyphs`), on its (shifted)
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
use in documents (the OS/2 embedding bits read as the specification
has them: restricted only when the restricted bit is set without a more
permissive one, since a font setting several takes the most permissive;
bitmaps-only counts from table version 2; the reserved bits, which old
fonts set, mean nothing; `tracedraw_text::usable_in_documents`); others
are skipped with a log line and their text falls back like any missing
font. The Fonts docker marks carried families
"embedded" under "Fonts in document". In every font list (the property
bar, the Text docker, the Glyphs docker) a carried family that is not
installed shows "Embedded" in orange at the end of its row, and the font
box shows it after the name while that family is the current one and
nothing is typed in it (#518,
`ui::font_list::tests::fonts_embedded_in_the_drawing_say_so`). Saving a drawing as `.tdraw` adds
the carried fonts its text uses (text imported or pasted from another
drawing), so the file keeps its letters on machines without them, and,
with Save As's Embed fonts on (it starts off), the installed fonts its
text uses whose licence allows it (`FontSystem::fonts_to_embed`; see
`files.md`, Save Drawing dialog). An embedded copy of an installed font
is ignored where the font is installed.

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
from another program usually gives); the font lists show both. Names
in every language the file gives count ("微软雅黑" as well as "Microsoft
YaHei"), including family names a font keeps only in a Chinese,
Japanese or Korean code page (GBK, Big5, Shift JIS, Wansung), as older
Chinese fonts such as "方正黑体_GBK" do (#860, #898). A
legacy name picks the face of that name nearest the text's weight and
style. A family name ending in a weight ("Titillium Web SemiBold",
"Lato Light", "Semi Bold" written apart too) that no font lists is
drawn with the family's face nearest that weight, and is not reported
missing (the notice on opening, the Font Manager and the fonts docker
check the family without the weight, #892). Drawn that way, through the family without the weight or a
stand-in, the weight in the name carries on everywhere: a variable font
draws at it on its weight axis ("Quicksand SemiBold" at 600), a family
with only a regular face draws a weight from 600 up thickened as bold
text is, a missing family's stand-in is the one for the family without
the weight at that weight ("Segoe UI SemiBold" in Selawik Bold), and in
the browser the family is fetched when no font of the whole name is
offered. A face a font lists under the whole name (family or legacy
name, in any case: "Archivo Black", "Quicksand Light") draws as that
face, at 400 or 700 for bold, as before.

A .cdr run set in a face between or beyond regular and bold (thin,
extra light, light, medium, semibold, extra bold, black) opens as that
face: the family takes the weight's name ("Titillium Web" at 600 opens
as "Titillium Web SemiBold", not bold), so editing, the PDF and the
saved file keep the face. A family already named with a weight keeps
its name, and is not bold when its name gives the run's weight
("Segoe UI Semibold" at 600). The saved file names the family as it is
shown, at regular or bold weight, and opens the same.

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
The face is chosen for each syllable, not each letter: a syllable goes to
the span's face only when it has all of its letters, else to the face
that has the first one it lacks, so a conjunct is never drawn half in one
face and half in another.

A syllable is a letter with the characters that continue it: combining
marks (vowel signs, virama, nukta, anusvara, accents), joiners, and a
consonant after a virama or a zero width joiner. Text is never cut inside
one for shaping (by face, by script or by span) and lines never break
inside one. A syllable that a span boundary cuts (a vowel sign or the
second half of a conjunct in another colour or style) is shaped whole
in the style of the span it starts in: the font draws a syllable as one
cluster, so the whole syllable takes that span's font, size and colour.
After a linker (the virama of Devanagari, Bengali, Gujarati, Oriya,
Telugu and Malayalam) only a consonant of the same script continues the
syllable. Plain ASCII text skips these checks.
Shaped in pieces, a half form showed with a visible halant, a vowel sign
with a dotted circle, and a line broken inside a word could start with a
vowel sign or the rest of a conjunct (#599). The corpus in
`crates/text/testdata/devanagari-corpus.txt` (half forms, conjuncts,
reph, rakar, nukta, vowel signs on conjuncts) checks it against Noto
Sans Devanagari: every word laid out in one span, after Latin text, in
a Latin face with the Devanagari face as fallback, split between two
spans at every character, and broken across lines, gives the glyphs of
the word shaped in one piece (`devanagari_tests`).

Each script is shaped on its own: a run is cut where its letters change
from one script to another (Latin, Greek, Cyrillic, Devanagari, Bengali
and the other Indian scripts, Thai, Lao, Tibetan, Myanmar, Khmer), what
the scripts share (spaces, digits, punctuation) going with the run
before it. Shaped together, Hindi after a Latin word took the Latin rules
and showed its half forms with a visible halant and its conjuncts apart
(#599, `tests::text_is_shaped_one_script_at_a_time`). Text with Hebrew
or Arabic letters is shaped as one run.

Variable fonts draw at weight 400, 700 for bold or the weight a family name ends in, on their weight
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

- `tracedraw_text::tests::vertical_text_sets_columns_with_turned_and_upright_letters`,
  `tests::letters_that_stand_upright_in_columns`,
  `document::tests::vertical_text_is_off_by_default_and_placed_from_the_top_right`,
  `text_editing::tests::vertical_text_has_carets_across_its_columns_and_a_switch`
  (vertical text).

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
  text draws nothing (`ui::font_list::tests::names_are_drawn_in_their_own_font`),
  in the theme's text colour (`ui::font_list::tests::names_take_the_theme_text_colour`);
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
- The text engine keeps each outline's box with the outline it caches;
  the box of a text object that is not turned or skewed comes from it,
  without copying the outline, which hovering, snapping and the Pen
  preview ask for many times a frame (#802,
  `tests::text_bounds_come_from_the_cached_box_and_match_the_outline`).
- `tests::fonts_are_found_by_their_legacy_family_names`,
  `tests::family_names_ending_in_a_weight_split`,
  `tests::a_family_named_with_its_weight_draws_with_the_nearest_face`,
  `tests::a_weight_in_the_family_name_reaches_every_way_of_drawing`,
  the .cdr writer's `write::tests::faces_named_with_their_weight_round_trip`,
  `tests::a_drawing_brings_every_face_of_a_family`, the .cdr reader's
  `parse::tests::semibold_runs_keep_their_face` and
  `text::tests::style_string_weights_are_style_codes`,
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
- While it waits, the pointer over a path or within 150 screen pixels of
  one shows the selected text where a click puts it: its letters
  outlined in blue, the line they stand on dashed, a dashed line from
  the nearest point of the path to the pointer with the distance (in
  the drawing's units), and a red mark across the line where the text
  begins. The text is centred on that nearest point and stands as far
  from the path as the pointer is, on the pointer's side; the click
  places it so, near the path or on it (`text_path_target`,
  `text_path_preview`; texts, pictures and groups are not paths;
  `text_path::tests::the_path_near_the_pointer_previews_the_text_and_a_click_places_it`).
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
  Distance from path and Offset along path in the document's units
  (a new distance, from the bar or a drag across the path, keeps the
  text where it is along the path: the offset, measured along the
  baseline, scales with the baseline's length, so text centred on the
  bottom of a circle stays centred as it moves in or out; the mirrors,
  which put the baseline on the path's other side, scale it the same
  way, #193,
  `text_path::tests::distance_from_the_path_keeps_the_text_in_its_place`),
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
