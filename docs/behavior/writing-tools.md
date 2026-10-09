# Writing tools

Text > Writing Tools: Spell Check, Grammar, Thesaurus and Autocorrect,
plus Text > Encode and Text > Make Text Web Compatible.

## Spell Check

System Hunspell word lists (see `text.md`). Unknown words are listed with
suggestions; Replace, Replace All, Skip, Skip All, Add to user list.

## Grammar

Rule-based checks on the selected text (or all text objects): doubled
words, double spaces, a space before punctuation, a missing space after
punctuation, lower-case sentence starts, unbalanced brackets, unbalanced
quotes, repeated punctuation, and sentences over 40 words. Each finding
names the rule and the sentence and carries a mechanical fix when one
exists; Replace applies it through the normal command path. The rules
are language-neutral.

## Thesaurus

Looks up the first word of the selected text, or a typed word. Sources:
a built-in English list for the most common words, or a MyThes `.dat`
file picked by the user (Options > Text or the dialog's button), which is
how the open-source office suites ship their thesauri. Double-click a
synonym to look it up, Replace swaps the word in the text keeping its
capitalisation.

## Autocorrect

Applied while typing (`App::update_text`), per the Autocorrect dialog:

- capitalise the first letter of sentences (default on),
- correct two initial capitals ("THe" to "The", default on),
- typographic quotes (default on): `"` and `'` become opening or closing
  quotes depending on the character before them,
- replacement table: `(c)` ©, `(r)` ®, `(tm)` ™, `1/2` ½, `1/4` ¼,
  `3/4` ¾; the user can add rows. The quote style follows the UI
  language.

Autocorrect never changes text that was pasted or imported, only typed
characters.

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

For each selected text object: fountain, pattern, texture and mesh fills
become a solid colour (first stop, front colour, first node); the outline,
drop shadow and live effects are removed; per-character fills that are
not solid are cleared. Solid fills and character colours are kept.
