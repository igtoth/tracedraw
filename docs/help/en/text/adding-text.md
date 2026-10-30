# Adding and editing text

## Adding text {#add}

1. Choose the [[tool.text]] tool (<kbd>F8</kbd>).
2. For **artistic text**, click where the text starts and type. Press
   <kbd>Enter</kbd> to start a new line.
3. For **paragraph text**, drag a frame, then type. The text wraps at the edges
   of the frame.
4. Press <kbd>Esc</kbd> to finish. An empty text is removed.

New artistic text is 24 points, paragraph text 12 points, in the first
sans-serif font found. Change the defaults by formatting with nothing selected.

To make an existing object a text frame, right-click it and choose to create
an empty text frame; see [Empty frames](../objects/clip-frames.md#empty-frames).
[[menu.text > menu.text.insert_placeholder_text]] fills the selected frames
with sample text, as much as fits.

## Editing text {#edit}

Click text with the [[tool.text]] tool, or double-click it with the
[[tool.pick]] tool, to edit it in place. [[menu.text > menu.text.edit_text]]
(<kbd>Ctrl+Shift+T</kbd>) opens a box with the whole text to edit
instead; **OK** puts it back in the drawing, and the characters you did
not change keep their formatting.

| Keys | What they do |
|---|---|
| Typing, <kbd>Ctrl+V</kbd> | insert at the caret, replacing the selection |
| <kbd>Backspace</kbd>, <kbd>Delete</kbd> | delete the selection, else the character before or after |
| <kbd>Enter</kbd>, <kbd>Shift+Enter</kbd> | new paragraph, new line |
| Arrow keys, with <kbd>Ctrl</kbd> | move by a character or a word; <kbd>Shift</kbd> selects |
| <kbd>Home</kbd>, <kbd>End</kbd>, with <kbd>Ctrl</kbd> | start or end of the line, or of the text |
| <kbd>Ctrl+A</kbd> | select all |
| <kbd>Ctrl+B</kbd>, <kbd>Ctrl+I</kbd>, <kbd>Ctrl+U</kbd> | bold, italic, underline |
| <kbd>Ctrl+L</kbd>, <kbd>Ctrl+E</kbd>, <kbd>Ctrl+R</kbd> | align left, center, right |
| <kbd>Ctrl+J</kbd>, <kbd>Ctrl+H</kbd>, <kbd>Ctrl+N</kbd> | full justify, force justify, no alignment |
| <kbd>Shift+F3</kbd> | change case (see [Writing tools](writing-tools.md#change-case)) |
| Double-click | select a word |

A new alignment keeps artistic text in place at the reference point of the X
and Y fields of the property bar; in a frame it applies to the paragraphs at
the caret.

Text in right-to-left scripts (Arabic, Hebrew) is laid out right to left,
also when it is mixed with left-to-right words, and selections shade every
selected letter. Keyboards and input methods that send characters (such as
those for Tamil and other Indian scripts on Windows) type into text and
fields, and <kbd>Alt</kbd> codes typed on the keypad give their character
once.

All your typing in one editing session is one undo step.

## Formatting from the property bar {#property-bar}

With the [[tool.text]] tool, or with text selected, the property bar has the
font list, the size (type it or pick from the list), bold, italic, underline,
the alignment (left, center, right, full justify), bullets and drop cap for
paragraph text, [[menu.text.text_docker]] (<kbd>Ctrl+T</kbd>) and
[[menu.text.edit_text]]. The button beside the font size opens the sliders of
a variable font's axes (see [Variable fonts](fonts.md#variable-fonts)). While editing, they act on the selected characters;
otherwise on the whole selected texts. Your choice also becomes the default
for the next text.

The font list shows each family in its own font, with the ones you used last
at the top. Type to narrow the list. A family with several styles has an arrow
that shows them. See [Fonts and glyphs](fonts.md).

## The Text panel {#text-docker}

The [[docker.text]] panel (<kbd>Ctrl+T</kbd>) has every text setting, in three
sections you can open and close: **Character**, **Paragraph** and **Frame**.
The three buttons at the top show the panel from that section on.

Full justification spreads Chinese and Japanese lines to the edge of the frame
by adding room between the characters, and paragraph text in those scripts
breaks lines by their rules: punctuation such as ，。、 and closing brackets
never start a line, and opening brackets never end one. Letters given another
color keep the font's kerning with their neighbours.

![The Text panel](../../img/text-adding-text-text-docker.png)

- **Character**: the font, style and size; range kerning (-50 to 200%) and
  underline; the fill, background and outline of the characters. More options
  under the section: character offsets and angle, strikethrough and OpenType
  features.
- **Paragraph**: the alignment (none, left, center, right, full justify, and
  forced justify, which spreads the last line too); line spacing in percent of
  the character height, points or percent of the font size; left, first-line
  and right indents; space before and after paragraphs; character and word
  spacing. The gear opens custom hyphens.
- **Frame**, for paragraph text: the background color of the frame, the
  vertical alignment (top, middle, bottom), fit text to frame, columns (1 to
  8) and the gap between them, and links to the bullets, drop cap and columns
  dialog boxes, and hyphenation.

Every change applies to the selected characters while editing, or to the
selected texts, and is one undo step. With no text selected, the settings
become those of new text. The button at the top right gives new text the
settings of the selected text.

## Paragraph text frames {#frames}

Frames show as dashed lines. A frame turns red, with a small tab under its
bottom edge, when its text does not fit; the text that does not fit is not
drawn, printed or exported. To make it fit:

- drag the frame's handles to enlarge it. The text flows again; the letters
  keep their size;
- choose [[menu.text > menu.text.paragraph_text_frame > menu.text.frame_fit_text]]
  to resize the text to fill the frame;
- link the frame to another one, so the text flows on:
  [[menu.text.frame_link]]. [[menu.text.frame_unlink]] breaks the link.
  Or click the small tab under the frame's bottom edge, then click another
  frame (on this page or another one) or click or drag on an empty spot
  for a new frame: the text flows on into it.

[[menu.text > menu.text.show_text_frames]] shows or hides the frames.

## Moving and turning characters {#characters}

With the [[tool.shape]] tool, text shows a node at each character: drag nodes
to move characters, and drag the arrows at the corners of the text to change
the character, word and line spacing. The character offsets and angle can also
be typed in the [[docker.text]] panel.
[[menu.text > menu.text.straighten_text]] puts characters back in line, and
[[menu.text.align_to_baseline]] (<kbd>Alt+F12</kbd>) removes their vertical
offsets.

## Text on a path {#text-on-path}

1. Select a text and a curve or shape.
2. Choose [[menu.text > menu.text.fit_text_to_path]].

Or click the outline of a path with the [[tool.text]] tool and type. Text on a
closed shape stands on its outside, centered on its top; clicked on the bottom
of an ellipse, it starts there.

With text on a path selected (also with the [[tool.pick]] tool), the property
bar shows the path's fields first, then the usual text fields:

- The orientation of the letters: rotated, vertical skew, horizontal skew or
  upright.
- The vertical placement: on the baseline, the ascender, the descender, or
  centred on the path.
- The placement along the path: at its start, centre or end, and on a closed
  path also **Top centre** (centred on the highest point, outside and
  upright) and **Bottom centre** (centred on the lowest point, inside and
  reading left to right, as the bottom line of a stamp).
- The distance from the path and the offset along it. A new distance keeps
  the text centred where it was, so a line at the bottom of a circle stays
  centred as it moves in or out.
- The two mirror buttons, which put the text on the other side of the path
  or turn it round, and [[menu.text.straighten_text]], which takes it off the
  path.

Drag the text to slide it along the path, or across the path to move it away
from it.

## Other text commands {#commands}

- [[menu.text > menu.text.insert_formatting_code]]: special characters such as
  [[menu.text.code_em_space]], [[menu.text.code_nonbreaking_space]],
  [[menu.text.code_optional_hyphen]] and [[menu.text.code_column_break]].
- [[menu.text.show_non_printing]] shows spaces, tabs and breaks.
- [[menu.text.make_web_compatible]] prepares text for web pages.
- [[menu.text > menu.text.convert]] switches between artistic and paragraph
  text.
- Converting text to curves ([[menu.object > menu.object.convert_to_curves]])
  keeps its look without the font.
- Text files can be imported; see [Importing files](../file-formats/import.md).
