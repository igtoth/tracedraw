# Writing tools

Text > Writing Tools: Spell Check, Grammar, Thesaurus and Autocorrect,
plus Text > Custom Optional Hyphens, Text > Encode and Text > Make Text
Web Compatible.

## Writing Tools dialog

Text > Writing Tools > Spell Check (Ctrl+F12), Grammar and Thesaurus open
one dialog on that tab: Spell Checker, Grammar and Thesaurus, with "?"
(the behaviour pages) and Close at the bottom. The checkers look at every
text object of the page (Check: Document) or the selected ones (Check:
Selection).

## Spell Checker

Dictionaries (`spell.rs`, `hunspell.rs`): Hunspell dictionaries, a
`.dic` word file with the `.aff` affix file of the same name, found in
TraceDraw's `dictionaries` folder under the configuration folder and in
the folder named under Options > Text > Spelling > Also read
dictionaries from (each with the folders inside it, three deep, so a
dictionary package unpacked there or another program's dictionary folder
works), then in the system's folders (`spell::system_folders`, #1405):
on Linux `/usr/share/hunspell`, `/usr/share/myspell`,
`/usr/local/share/hunspell` and `~/.local/share/hunspell`; on macOS
`/Library/Spelling` and `~/Library/Spelling`; on Windows
`%LOCALAPPDATA%\Microsoft\Spelling` and `%APPDATA%\Microsoft\Spelling`
(two deep: Hunspell dictionaries added for the system's spell checker,
one folder per language, and its lists of added words, UTF-16 with a byte
order mark) and `%APPDATA%\hunspell`; and everywhere the dictionary
extensions of office suites installed in the usual places
(`<Program Files>\<suite>\share\extensions\dict-*`,
`/usr/lib/<suite>/share/extensions`, `/opt/<suite>/share/extensions`,
`/Applications/<suite>.app/Contents/Resources/extensions`). A name found
twice is used once, the user's folders first. `hyph_*` and `th_*` files
(hyphenation patterns, thesauri) are left out; a `.dic` with no `.aff`
is a plain word list. The Spelling tab lists them by name, each with a
check box (on; the names switched off are kept in the preferences),
Open Folder (creates and opens TraceDraw's folder) and Read Again
(reads the folders and the dictionaries again).

More dictionaries on every platform (#1405):

- Download dictionary: a list of 42 languages of the wooorm/dictionaries
  collection on GitHub (Hunspell dictionaries in UTF-8, each with its
  own free licence, shown in the button's tip and listed in
  `ATTRIBUTION.md`; the interface language's is chosen first) and
  Download, which fetches the affix file, the word file and the licence
  (`dictionary_download.rs`). On the desktop and on Android they go to
  TraceDraw's dictionaries folder as `<code>.aff`, `<code>.dic` and
  `<code>.license.txt` (`en_GB`, `pt`); a browser has no folder, so it
  holds them in memory and fetches them again at each visit (the
  settings remember them, `downloaded_dictionaries`; the browser's cache
  answers). The status bar says when one is installed or why it failed.
- Add Files: `.dic` and `.aff` files picked with the system's (or the
  browser's, or Android's) file picker are copied to TraceDraw's
  folder, or held in memory for the visit in a browser.
- Windows: its own spell checker (the Spell Checking API,
  `spell_windows.rs`) checks the languages installed in Windows; it is
  listed as "Windows spell checker (en-US, ...)" with a check box. A word
  no dictionary knows is asked of it, and its suggestions come first.
- The browser hides the dictionaries folder and "Also read dictionaries from" (it has no folders).

A word is right when any dictionary in use, the spell checker of Windows,
a system word list (`/usr/share/dict/*`), the built-in core words or the
user's own words know it; a hyphenated word when each part is. The affix file is read for `SET` (UTF-8, ISO 8859,
Windows code pages, KOI8-R), `FLAG` (characters, `long`, `num`), `AF`
aliases, `PFX` and `SFX` rules (strip, add, condition, cross product,
continuation classes: two suffixes, a suffix and a prefix),
`NEEDAFFIX`, `FORBIDDENWORD`, `ONLYINCOMPOUND`, `KEEPCASE`, compounds
(`COMPOUNDFLAG`, `COMPOUNDBEGIN`, `COMPOUNDMIDDLE`, `COMPOUNDLAST`,
`COMPOUNDPERMITFLAG`, `COMPOUNDMIN`, `COMPOUNDWORDMAX`), `IGNORE`, `TRY`
and `REP`. Case: a word typed with a capital (or in capitals) is also
looked up in small letters, but not one kept in its case. A typographic
apostrophe counts as a straight one. Suggestions: the dictionaries'
replacement tables, then single edits with their `TRY` letters, then a
split into two words, then edits found in the word lists; eight at most.
Checks: `hunspell::tests` (and `real_dictionaries`, ignored, over
dictionaries in `HUNSPELL_TEST_DIR`: English, Brazilian Portuguese and
German words, affixed and compound), and
`spell::tests::dictionaries_in_a_folder_are_found_and_used`,
`spell::tests::system_folders_per_platform`,
`dictionary_download::tests::the_catalog_names_its_files_and_licences` and
`an_installed_dictionary_is_used`.

Left: Replace with, the
Replacements list (suggestions; a click puts one in Replace with, a
double click replaces with it) and the status ("Ready: Begin spell
checking", the word not found, then "Spell check completed"). Right:
Start (Replace once a word is shown), Auto Replace (every place the word
is), Skip Once, Undo (the last replacement), Skip All (for this check),
Options (ignore words with numbers, ignore words in capitals, add
corrections to Autocorrect; the same as Options > Text > Spelling), Add
(to the user word list) and Check.

## Grammar

Rule-based checks: doubled words, double spaces, a space before
punctuation, a missing space after punctuation, lower-case sentence
starts, unbalanced brackets, unbalanced quotes, repeated punctuation,
sentences over 40 words, a paragraph's last sentence of three words or
more with no end mark, and, for English, "a" before a vowel sound or
"an" before a consonant ("an hour", "a one-off" and "a university" are
right). The tab shows the replacement, the new sentence and the rule
name; Start (Replace once a finding is shown), Undo, Skip Once, Skip Rule
(for this check) and Check. When no finding is left it asks "Grammar
check completed. Close the grammar checker?": Yes closes the dialog, No
keeps it open for a new check.

## Thesaurus

The word field (the first word of the selected text at first) with the
words looked up before in its list, Look Up (or Enter), Insert and
Options (close after inserting; pick a MyThes `.dat` file). Sources: a
built-in English list for the most common words, or the MyThes file,
which is how the open-source office suites ship their thesauri. The
meanings show with their synonyms; a click picks one, a double click
looks it up. Insert swaps the word in the text keeping its
capitalisation, or types the synonym at the caret of the text being
edited.

## Custom Optional Hyphens

Text > Custom Optional Hyphens, and the gear of the Text docker's
Paragraph section. Words the user hyphenates by hand ("Trace-Draw"),
kept in the preferences with their language, break only there when
their text is hyphenated (they win over the patterns, capitals or not).

- Automatically insert custom optional hyphens: When typing (a text that
  holds a defined word when its editing ends gets hyphenation on) and
  When pasting and importing text (the same for pasted and imported
  text objects). Both on at first.
- Language (the interface's and English, or every one with Show all
  languages), Word, Hyphenated Word (hyphens where the word may break;
  the letters must match the word), Add Definition, Remove Definition,
  and the list of definitions (Word | Hyphenated Word) for the language.
- "?", Apply to Selection (keeps the definitions and turns hyphenation on
  for the selected texts that hold a defined word), OK (keeps them) and
  Cancel.

## Checks

- `ui::writing_tools::tests::every_tab_draws`,
  `ui::writing_tools::tests::spell_options_and_skips_leave_words_out`,
  `ui::writing_tools::tests::grammar_findings_skip_the_rules_skipped`,
  `ui::writing_tools::tests::hyphenated_words_give_break_points`,
  `ui::writing_tools::tests::custom_hyphens_turn_hyphenation_on_and_break_words`,
  `grammar::tests::a_and_an_follow_the_next_sound_and_sentences_need_an_end`,
  `tests::custom_hyphens_come_before_the_patterns` (text crate).

## Autocorrect

Tools > Options > Text > Autocorrect (Text > Writing Tools > Autocorrect
opens that tab). Applied while typing (`App::update_text`), once a word
is finished with a space or punctuation:

- capitalize first letter of sentences (default on),
- correct two initial capitals ("THe" to "The", default on),
- capitalize names of days (English day names, "monday" to "Monday",
  "monday's" too; default off),
- automatically create hyperlinks when typing web addresses (default
  off): a word starting `http://`, `https://` or `www.` with a dot in its
  host, finished with a space, gives the text object that link
  (`www.` gets `http://`), unless it has a link already; one undo step of
  its own,
- language: whose quotation marks are used (default the interface
  language); choosing another resets the marks to that language's,
- typographic quotes (default on): `"` and `'` become opening or closing
  quotes depending on the character before them; use straight quotation
  marks after numbers (default off) leaves a quote right after a digit
  straight (6' 2"),
- single and double quotes: the opening and closing marks, each one of
  ' ‘ ’ ‚ ‹ › " “ ” „ « » 「 」 『 』; the language's until one is chosen,
- replacement table, applied while Replace text while typing is on
  (default on): `(c)` ©, `(r)` ®, `(tm)` ™, `1/2` ½, `1/4` ¼, `3/4` ¾.
  Replace and With fields; Add (a new word, no spaces), Delete (the row
  clicked), Replace (gives the row of the word in Replace the text in
  With). A click on a row fills the fields.
- Apply to Selection runs every rule over the selected text objects.

Settings written by older versions with autocorrect switched off as a
whole load with every rule off.

Autocorrect leaves alone the letters typed in a font that sets another
script at the places of Latin letters, as fonts made before Unicode had
the script do (older Tamil fonts typed with the Latin keys of their
letters, symbol fonts): a capital or a curly quote would be another
letter there. Such a font has a symbol character map, or says it draws
a script (Indian scripts, Sinhala, Thai and the others of South-East
Asia, Tibetan, Arabic, Hebrew, Armenian, Georgian, Ethiopic, Greek,
Cyrillic) and maps none of its characters (#1358,
`text_editing::tests::autocorrect_leaves_fonts_with_a_script_at_latin_places_alone`,
`tracedraw_text::tests::fonts_with_a_script_at_latin_places_are_told_apart`).

Autocorrect never changes text that was pasted or imported, only typed
characters.

Checks: `autocorrect::tests::days_quotes_and_links_follow_the_options`,
`autocorrect::tests::older_settings_with_autocorrect_off_turn_every_rule_off`,
`ui::writing_tools::tests::corrections_go_to_the_autocorrect_table`,
`ui::options::tests::every_text_tab_draws`.

## Encode

Text > Encode re-interprets the characters of the selected text: each
character is mapped back to the byte it came from under the "read as"
encoding and the bytes are decoded again with the right one. Encodings:
UTF-8, Windows-1252, ISO-8859-1, Mac OS Roman, Windows-1251. Characters
that cannot have come from the source encoding are kept. The dialog
previews the result before OK.

Check: "cafÃ©" read as Windows-1252, really UTF-8, becomes "café"; a
Cyrillic word shown as "Ïðèâåò" (read as Latin-1, really Windows-1251)
becomes "Привет".

## Make Text Web Compatible

For each selected text object: gradient, pattern, texture and mesh fills
become a solid colour (first stop, front colour, first node); the outline,
drop shadow and live effects are removed; per-character fills that are
not solid are cleared. Solid fills and character colours are kept.
