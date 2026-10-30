# Text

TraceDraw has two kinds of text:

- **Artistic text**: a line (or a few lines, with **Enter**) placed at a
  point. Good for headlines, labels and logos. New artistic text is
  24 pt.
- **Paragraph text**: text in a frame that wraps from line to line. Good
  for body copy, columns and anything long. New paragraph text is 12 pt.

## Add text

Pick the **Text** tool (`F8`).

- To add **artistic text**, click on the page where the text should
  start and type.
- To add **paragraph text**, drag a frame, then type inside it.
- Press **Esc** when you are done: you go back to the Pick tool. Empty
  text is removed.

To edit existing text, click it with the Text tool, or double-click it
with the Pick tool. `Ctrl+Shift+T` (**Edit Text**) does the same for the
selected text.

### Editing keys

| Keys | What they do |
|---|---|
| **Enter**, **Shift+Enter** | new paragraph, line break |
| **Left**, **Right** (with **Ctrl**: a word) | move the caret; add **Shift** to select |
| **Up**, **Down** | the line above or below |
| **Home**, **End** (with **Ctrl**: the whole text) | start or end of the line |
| **Ctrl+A** | select all the text |
| **Ctrl+B**, **Ctrl+I**, **Ctrl+U** | bold, italic, underline |
| **Ctrl+C**, **Ctrl+X**, **Ctrl+V** | copy, cut, paste |
| double-click | select a word |

All the typing of one editing session is a single undo step.

## Choose the font and size

The property bar shows the text settings when the Text tool is active,
or when only text is selected with the Pick tool:

- the **font list**;
- the **size** in points (type one, or pick from the list);
- **Bold**, **Italic** and **Underline**;
- the **alignment**: Left, Center, Right or Full justify;
- for paragraph text, **Bullets** and **Drop Cap**;
- **Text Properties**, which opens the Text panel (`Ctrl+T`).

A change applies to the selected text objects, or to the selected
characters while you are editing. It also becomes the setting for the
next text you add.

### The font list

- Click in the field and type to find a family: names that hold the
  letters you typed are listed, the best match first. **Down**, **Up**
  and **Enter** pick one; **Esc** puts the old name back.
- The top of the list filters it: all fonts, installed fonts, fonts in
  this drawing, or (in the browser) fonts to download.
- **Used last** lists the five families you picked most recently.
- Each name is drawn in its own font. A family with several styles has
  an arrow to show them (Regular, Italic, Bold and so on).
- A slider sets the sample size, and a preview shows the family under
  the pointer.

`Ctrl+8` and `Ctrl+2` make the selected text larger and smaller by the
keyboard text increment (1 pt; set in **Tools > Options > TraceDraw >
Text**).

### Missing fonts

If a drawing uses a font you do not have, the text keeps its font name
(saving writes it back unchanged) and is drawn with a stand-in, chosen
to match its widths where possible so lines break as before. The status
bar and the **Font Manager** (on the standard toolbar's **Launch**
button) name the stand-in beside each missing family. Fonts embedded in
an opened `.cdr` file are used when their licence allows it, and saving
as `.tdraw` keeps them with the drawing.

Detail: [the text notes](../behavior/text.md#missing-fonts).

## The Text panel

Open it with `Ctrl+T`, **Text Properties** on the property bar, or
**Window > Panels > Text**. It has three sections:

- **Character**: font, size and style; tracking; character offsets and
  angle; strikethrough; and OpenType features (ligatures, small
  capitals, old-style numerals, fractions, swashes).
- **Paragraph**: alignment; line spacing; character and word spacing;
  space before and after paragraphs; first-line, left and right
  indents. While you type in a text, these settings (and Bullets, Drop
  Cap, Tabs and Hyphenate) change only the paragraphs you have selected,
  or the one with the cursor; with the whole text object selected they
  change every paragraph.
- **Frame** (paragraph text): 1 to 8 columns and the gutter between
  them; vertical alignment (top, middle, bottom); Bullets; Drop Cap;
  Hyphenate; and Fit Text to Frame.

## Work with paragraph frames

- Frames show as dashed lines. **Text > Show Text Frames** shows every
  frame, or only the selected one.
- A frame turns **red** when the text does not fit. Make the frame
  larger, or link it to another frame so the text flows on.
- Resizing a frame with the Pick tool's handles reflows the text; the
  letters keep their size.
- **Fit Text to Frame** scales the text so it fills the frame.
- Paragraph text flows round objects placed over it: select the object
  and click **Wrap Paragraph Text** on the property bar (or in its
  context menu). The arrow beside the button picks how: along the
  object's outline or its bounding box, with the text on its left, on
  its right, on both sides or (bounding box) only above and below, and
  the room kept free around it. Objects below the frame do not push
  its text.
- With **Text > Align to Baseline Grid**, the lines of every frame sit
  on the page's baseline grid, so text in frames side by side lines up.
- **Text > Insert Placeholder Text** fills the selected frames with
  filler text, as much as each one holds.

## Move single characters

With text selected, the **Shape** tool shows a node at each character:

- Drag nodes to shift the chosen characters.
- Drag the arrow at the lower right corner to change the character
  spacing (with **Shift**: word spacing).
- Drag the arrow at the lower left corner to change the line spacing.
- The property bar sets the chosen characters' offsets and angle.

**Text > Straighten Text** puts every character back, and **Text >
Align to Baseline** (`Alt+F12`) removes the vertical offsets only.

## Text on a path

Text can follow a curve, with a distance from the path, an offset along
it, and a mirror option to read along the other side. **Text >
Straighten Text** takes text off its path again. The formulas are in
[the text notes](../behavior/text.md#formulas).

## Tables

To draw a table, pick the **Table** tool, set the rows (3 to start with)
and columns (4) on the property bar, and drag.

- Click a cell and type. **Tab** moves to the next cell, **Shift+Tab**
  back. A row grows as its text needs more room.
- The **Table** menu inserts rows and columns, selects cells, rows,
  columns or the table, deletes, distributes rows and columns evenly,
  merges cells (`Ctrl+M`), unmerges and splits.
- **Convert Text to Table** turns paragraphs into rows and cuts them
  into cells at the separator you choose (commas, tabs, your own, or a
  cell per paragraph);
  **Convert Table to Text** does the reverse.
- The property bar sets the cell fill and the border.

See [the table notes](../behavior/table.md).

## Writing tools

**Text > Writing Tools** offers:

- **Spell Check**: unknown words with suggestions; Replace, Replace All,
  Skip, Skip All and Add to the user list.
- **Grammar**: checks such as doubled words, extra spaces, missing
  spaces after punctuation and unbalanced brackets or quotes.
- **Thesaurus**: synonyms for a word; **Replace** swaps it in.
- **Autocorrect**: capitalises sentences, fixes two initial capitals,
  types typographic quotes and replaces `(c)`, `(r)`, `(tm)` and simple
  fractions as you type.

Also in the Text menu: **Change Case**, **Text Statistics**, **Encode**
(repairs text that was read with the wrong character encoding) and
**Make Text Web Compatible**. **Find and Replace** (`Ctrl+F`) searches
text and object attributes, and the **Glyphs** panel shows every
character of a font.

Detail: [the writing tool notes](../behavior/writing-tools.md).

## Where next

- Colour your text: [Colour and fills](color-and-fills.md).
- Bring text in from documents: [Files](files.md#import).
