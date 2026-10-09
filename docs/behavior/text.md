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
| Hyphenation | off; when on, min 3 letters before/after, 2 in a row |
| Columns | 1, gutter 5 mm |
| Bullets | off; indent 5 mm, bullet "*" from the paragraph font |
| Drop cap | off; 3 lines, no indent |
| Tabs | every 12.7 mm (0.5 in) |
| Text on path | distance 0, offset 0, orientation "rotate letters" |

## Property bar

Text tool, and the Pick tool with only text selected (after the object
fields): font family, size in points, Bold, Italic, Underline, and the
alignment (Left, Center, Right, Justify). Each change applies to the
selected text objects at once (every span takes the font, size and
style; the object takes the alignment) as one "Text Style" undo step,
and becomes the default for the next text. Double-clicking text with the
Pick tool edits it in place.

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
characters while a selection exists, and show the style at the caret. All
the keystrokes of one editing session are one "Edit Text" undo step.

## Formulas

- Line height `= size * leading / 100 * (asc - desc + gap) / upm`.
- Justify: extra space `= (frame_width - line_width) / n_spaces`.
- Text on path: each glyph is placed at arc length `s + offset`
  where `s` is the glyph's x-advance midpoint, rotated to the tangent
  and raised by `distance` along the normal.
- Fit text to frame: scale size so the paragraph fills the frame height.

## Checks

- Given "ab" at 100 pt, then the width equals the sum of the two
  advances minus the kerning pair from the font.
- Given a paragraph frame narrower than one word, then the word breaks
  by character.
