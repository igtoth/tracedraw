# Writing tools

TraceDraw checks spelling and grammar, finds synonyms, corrects as you type,
and fixes text that came in with the wrong characters. The commands are in
[[menu.text > menu.text.writing_tools]] and the [[menu.text]] menu.

## The Writing Tools dialog box {#writing-tools}

[[menu.text > menu.text.writing_tools > menu.text.spell_check]],
[[menu.text.grammatik]] and [[menu.text.thesaurus]] open the
[[writing.title]] dialog box at that tab. [[writing.check]] chooses whether to
check every text of the page ([[writing.scope_document]]) or only the
selected texts ([[writing.scope_selection]]).

![The Writing Tools dialog box](../../img/text-writing-tools.png)

### Spell Checker {#spell-checker}

1. Click [[writing.start]]. The first word not found shows, with suggestions
   under [[writing.replacements]].
2. Click a suggestion to put it in [[writing.replace_with]] (double-click to
   replace at once), or type the right word.
3. Click [[writing.replace]] for this place, [[writing.auto_replace]] for every
   place, [[writing.skip_once]] or [[writing.skip_all]] to leave it, or
   [[writing.add]] to add the word to your own word list.
4. [[writing.undo]] takes back the last replacement.

[[writing.options]] has [[writing.ignore_digits]],
[[writing.ignore_capitals]] and [[options.add_to_autocorrect]].

### Dictionaries {#dictionaries}

TraceDraw checks spelling with Hunspell dictionaries: for each language a
`.dic` file and an `.aff` file with the same name, such as `pt_BR.dic` and
`pt_BR.aff`. Free dictionaries for most languages can be downloaded from the
projects that make them; other programs often install them too.

1. Open [[menu.tools > menu.tools.options]] > [[options.text]] >
   [[options.spelling]].
2. Click [[options.open_folder]] and copy the `.dic` and `.aff` files into that
   folder (folders inside it work too), or name a folder that already has
   dictionaries in [[options.more_dictionaries]].
3. Click [[options.refresh_dictionaries]]. The [[options.dictionaries]] list
   shows each one; clear a box to stop using it.

A word is right when any dictionary in use knows it. Dictionaries installed on
your system are used as well. Without any, only a built-in word list is used.

Hyphenation dictionaries (`hyph_` files such as `hyph_pt_BR.dic`) go in the
same folders. Choose one under [[options.text]] > [[options.paragraph_text]] >
[[options.hyphenation_patterns]] to hyphenate text in that language.

### Grammar {#grammar}

The grammar checker finds doubled words, double spaces, spaces before
punctuation, missing spaces after it, sentences that start in lower case,
unbalanced brackets and quotes, repeated punctuation, sentences over 40 words,
missing end marks and, in English, "a" and "an" before the wrong sound. For
each finding it shows the replacement, the [[writing.new_sentence]] and the
[[writing.rule_name]]. Click [[writing.start]] (then [[writing.replace]]),
[[writing.skip_once]] or [[writing.skip_rule]].

### Thesaurus {#thesaurus}

Type a word (the first word of the selected text at first) and click
[[thesaurus.look_up]]. The meanings show with their synonyms: click one to
pick it, double-click to look it up. [[writing.insert]] puts it in the text,
keeping the capitals. Under [[writing.options]], [[writing.auto_close]]
closes the dialog box after inserting, and [[thesaurus.pick_file]] uses a
thesaurus file you have.

## Autocorrect {#autocorrect}

[[menu.text > menu.text.writing_tools > menu.text.autocorrect]] opens the
[[dialog.autocorrect]] tab of [[menu.tools > menu.tools.options]] >
[[options.text]], which sets what is corrected while you type:

- [[autocorrect.capitalize]];
- [[autocorrect.two_capitals]] ("THe" becomes "The");
- [[autocorrect.days]] ("monday" becomes "Monday");
- [[autocorrect.hyperlinks]]: typing `www.example.com` or an address starting
  with `http://` or `https://` gives the text object that hyperlink, unless it
  has one;
- [[autocorrect.language]] the language whose quotation marks are used (the
  interface language unless you pick another);
- [[autocorrect.quotes]], and [[autocorrect.straight_after_numbers]];
- [[autocorrect.single_quotes]] and [[autocorrect.double_quotes]]: the
  [[autocorrect.open]] and [[autocorrect.close]] marks, if you want others than
  the language's;
- the [[autocorrect.replacements]], used when [[autocorrect.replace_while_typing]]
  is on: `(c)` becomes ©, `(r)` ®, `(tm)` ™, `1/2` ½ and so on. Type in
  [[autocorrect.replace_col]] and [[autocorrect.with_col]], then click
  [[autocorrect.add]]; click a row to [[autocorrect.delete]] it, or change its
  text and click [[autocorrect.replace]].

[[autocorrect.apply_selection]] applies the rules to the selected text. Text
you paste or import is never changed.

![The Autocorrect tab](../../img/text-writing-tools-autocorrect.png)

## Custom hyphens {#custom-hyphens}

[[menu.text > menu.text.custom_hyphens]] (also the gear of the Paragraph
section of the [[docker.text]] panel) defines where your own words may break,
such as product names. Those words break only there when hyphenation is on.

1. Choose the [[hyphens.language]] ([[hyphens.all_languages]] lists every
   one).
2. Type the [[hyphens.word]] and the [[hyphens.hyphenated]] with hyphens where
   it may break, such as `Trace-Draw`.
3. Click [[hyphens.add]]. [[hyphens.remove]] removes the selected
   definition.
4. Under [[hyphens.auto_insert]], [[hyphens.when_typing]] and
   [[hyphens.when_pasting]] turn hyphenation on for texts that hold a defined
   word.
5. Click [[hyphens.apply_selection]] to apply them to the selected text now,
   or [[dialog.ok]] to keep them.

![The Custom Optional Hyphens dialog box](../../img/text-writing-tools-custom-hyphens.png)

## Change case {#change-case}

[[menu.text > menu.text.change_case]] changes the selected text to
[[menu.text.case_sentence]], [[menu.text.case_lower]],
[[menu.text.case_upper]], [[menu.text.case_title]] or
[[menu.text.case_toggle]].

![The Change Case dialog box](../../img/text-writing-tools-change-case.png)

## Text statistics {#statistics}

[[menu.text > menu.text.text_statistics]] counts the text objects, lines,
words, characters and fonts of the selected text, or of the page.

![The Text Statistics dialog box](../../img/text-writing-tools-statistics.png)

## Encode {#encode}

Text that came from another program sometimes shows the wrong characters,
such as "cafÃ©" instead of "café". [[menu.text > menu.text.encode]] fixes it:

1. Select the text.
2. Choose how the text [[dialog.encode_from]] and what [[dialog.encode_to]]
   (UTF-8, Windows-1252, ISO-8859-1, Mac OS Roman or Windows-1251).
3. Check the [[dialog.preview]] and click [[dialog.ok]].

![The Encode Text dialog box](../../img/text-writing-tools-encode.png)

## Web-compatible text {#web-compatible}

[[menu.text > menu.text.make_web_compatible]] simplifies the selected text for
web pages: gradient, pattern, texture and mesh fills become a flat color, and
outlines, shadows and effects are removed.
