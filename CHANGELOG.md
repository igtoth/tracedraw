# Changes

Versions follow the tags of this repository; every push to `main` also
publishes a nightly build. [`docs/features.md`](docs/features.md) has the
full, row-by-row status.

## Unreleased

- PDF, print, PostScript, EPS and SVG: an outline set behind the fill is painted first and the fill over it, as on screen; a white outline behind black letters no longer covers them, so black text no longer prints white (#235, #191).

- Windows: at interface size 100 % the text is as large as the system's own (12 px at 100 % scaling), as the setting says; it was 10.4 px. A size raised to make up for it may be set back to 100 % (#229).

- Blend: the steps lie between the two objects in the stacking order; a front object blended to one behind it no longer has the steps drawn over it (#232).

- CMYK drawings: new artistic and paragraph text is CMYK black, and an outline turned on in Properties is CMYK black, so their colour pickers start in CMYK; RGB drawings keep RGB black (#259).

- Properties > Outline: dragging in the colour picker is one undo step and adds only the final colour to the document palette, when the button is let go, as the fill picker does (#258).

- Shape tool: Smooth and Symmetrical now act on nodes beside a straight segment: next to a line the curve's handle lines up with it, and between two lines both become curves and the node is smoothed (#228).

- Shape tool: a click takes the nearest node, handle or segment, measured on the page; on curves with a scale of their own, such as text converted to curves, a click no longer took a node far from the pointer (#242).

- Text: the size field shows the size as seen on the page after the text is scaled with the Pick tool (24 pt doubled reads 48 pt), and a size typed there is the size the text then shows (#243).

- Pick tool: while a rectangle with rounded corners is resized, the preview outline keeps the corners round as the result will, instead of stretching them (#239).

- Welcome Screen: the property bar (page size, orientation, units, nudge, duplicate offset), the toolbox, Snap To and the standard bar's drawing commands are pale, also with drawings open behind it; New, Open, Options, Launch and AI Generate stay on (#246).

- Transformations, Rotate: the centre of rotation moved on the canvas is the centre the docker turns about (and shows with Relative center off), and after Apply the new copies keep it, so repeated rotated copies go round one point instead of drifting (#230).

- Dimension tools: Ctrl keeps the leg being given (the second point of a linear dimension, each side of an angular one) to steps of the constrain angle, preview included (#251).

- Options > Customization > Commands: tools take shortcuts with Ctrl, Alt and Shift like menu commands, and Alt with a letter given to a command no longer opens a menu (#248); Delete also removes a built-in shortcut, leaving the command without one (#249); Reset to defaults resets only the category shown, and a separate Reset all shortcuts asks first (#250).

- A right click on a locked object (with the Pick tool) opens a menu with Unlock Object, which unlocks and selects it, and Unlock All Objects; the object stays unselected and uneditable until then (#256).

- Curve tools (Pen, Bezier and the others drawing node by node): Ctrl+Z while a curve is drawn takes back the last node, handle pull or click on a node, one at a time, before going on to the document's history; Ctrl+Shift+Z puts them back (#233).

- Shape tool: Join now makes the two chosen end nodes one node halfway between them, with no straight segment added: the ends of one curve close it there, ends of two curves make one curve. Dropping a dragged end node on another end node joins them the same way (#254).

- Pen: with Auto add/delete on, a click by the first node of the curve being drawn closes the curve again instead of deleting that node; the first and last nodes now answer within the larger of the join and pick distances (#238).

- Shape tool: a click on a segment now chooses the segment itself (drawn highlighted with its handles) instead of showing its end node as chosen, and Delete removes just that segment: an open curve splits there, a closed one opens there, the nodes stay (#240).

- Shape tool: deleting a node between segments now refits a single curve from the neighbour before to the neighbour after when either side is a curve, so the shape is kept instead of reshaping (or going straight). A node between two straight segments still leaves a straight join (#241, #228).

## 0.10.0 (2026-10-20)

- The `.tdraw` format is version 0 again: the 1 written by 0.5 to 0.9 had the same layout and is read as 0, and Save As shows "v0" with nothing to choose. The next real change of layout will be version 2.

- Every property bar, page bar and tool bar button now shows a rich tip like the standard bar: its name in bold and what it does under it (descriptions in all 18 languages). The Options button of the standard bar has a down arrow, since it opens a list.

- Dialogs taller than their first frame no longer stay cut off at the bottom (hiding OK and Cancel); they grow to fit the window before they scroll.

- Bitmaps > Remove Background...: makes the background of photos transparent on this computer, without an AI service. The background colours are learnt from the picture's edges (or picked with an eyedropper), with tolerance, edge softness, Contiguous, Remove specks and Clean edge colors, and a before and after preview; also on the bitmap property bar. The AI command is now Remove Background with AI (#192).

- AI service presets for OpenCode Zen and OpenCode Go (drawings only); requests to them carry the `x-opencode-session` header those services require, one id per run (#173).

- Dialogs can be dragged by their title bar; they open centred (or where they were left, with Center dialog boxes off).

- CMX files: text exported in several colours is read with each part in its colour and place (the long instruction form, character styles and per character styles are read), and version 8 `.cdr` files from the bidirectional edition read their paragraph text (#187).

- Tables: cells are selected by dragging across them, Ctrl+click,
  Shift+click, the row and column arrows at the left and top edges, the
  corner and Ctrl+A; selected whole rows or columns move by dragging;
  row and column lines resize by dragging, and W and H on the bar size
  the selected cells. Insert adds as many rows or columns as are
  selected, and Insert Rows, Insert Columns and Split into N ask how
  many. Selected cells get their own background (also from the
  palette), borders per edge (all, outside, inside, top, bottom, left,
  right, inner) with width, colour and style, and the bar's and Text
  docker's text settings; tables get margins per side and separated
  borders. Options > Tools > Table chooses the Tab order and whether
  Tab types a tab.
- Outline widths are shown and typed in a unit of their own, points by
  default (millimetres, centimetres, inches or pixels on request), and
  the width list offers round values in it.
- Outlines: position outside, centred or inside the path; a mitre limit
  angle (5 degrees for new outlines); Align dashes fits a dash pattern
  so corners and line ends fall on dashes; Edit Line Style makes custom
  dash patterns on a grid, kept in the settings and listed with the
  presets. SVG and PDF exports draw them the same.
- Drop shadows: perspective shadows dragged from an edge (angle,
  stretch, fade), merge modes (Multiply for new shadows), feather
  direction and edges, presets, on-canvas handles (end with colour drop,
  opacity bar, start) and Break Drop Shadow Apart (a bitmap behind the
  object, one undo step). Shadows in older files look as before.
- Ctrl+click selects an object inside a clip frame (where the frame
  shows it), and Ctrl+Alt+click digs through a clip frame's contents,
  as in groups; text over a rectangle in a `.cdr` drawing is hit first,
  and an empty rectangle over text gives way to Alt+click (#190).
- Fit Text to Path on a closed path centres the text where the path is
  clicked (the bottom of an ellipse too), the mirrors keep the letters
  in place along the path, and the Text placement list has Top centre
  and Bottom centre on a closed path, the bottom one reading left to
  right inside the curve; Tick snapping moves the text off the path in
  steps of its spacing (#193).
- Print > Prepress > Invert prints a true negative: the sheet's paper
  turns black as well, so black text no longer comes out white on white
  paper (#191).
- The Properties docker keeps its width when an object is selected:
  the Transparency section sits in three rows, the Rectangle section
  lists the corner kinds one under the other with the radii two by two,
  and no section can push the docker wider (#196).
- Properties docker, Outline Pen and property bar with several objects
  selected (#204): changing one value (such as the outline width) no
  longer gives every object the first object's whole outline, fill or
  transparency; only the value changed is set and each object keeps the
  rest. Values that differ across the selection show as mixed (an empty
  field, no colour, nothing highlighted); a value set there goes to all
  (docs/behavior/mixed-selection.md). The property bar's width, line
  style and arrowheads and a palette outline colour also keep each
  object's other outline values, inside groups too.
- Page tab menu: six rows with a picture each and no separators (Rename
  Page..., Insert Page After, Insert Page Before, Duplicate Page...,
  Delete Page, Switch Page Orientation); Page Size and Page Background
  stay in the Layout menu. Duplicate Page... (also in the Layout menu and
  the Pages panel) opens a dialog: insert the copy before or after the
  selected page, and copy the layers only or with their contents. The
  copy becomes the current page and one undo removes it.
- Rectangles: new rectangles keep their corner size when resized
  (Relative corner scaling is off by default on the property bar and in
  Options > Toolbox > Rectangle tool), so a rounded rectangle stretched in
  one direction keeps circular corners of the same radius. Turn the
  setting on to have corners scale with the rectangle. Drawings saved
  earlier open unchanged; preferences from older versions start with it
  off once. Scaling a group evenly keeps such corners fixed (grown by the
  factor) instead of turning a stretched rectangle into a curve, and
  `.cdr` versions with rectangle objects save a stretched one as a
  rectangle with its radius.
- Pen: Preview mode (on by default, on the property bar and in Options >
  Tools > Curve tools) shows the next segment dashed in blue from the last
  node to the pointer before the click, curved by the node's handle. Auto
  add/delete (on by default): a click on a segment of the curve being
  drawn or of the selected curve adds a node, a click on a node deletes
  it, with plus and minus pointers. While drawing with the Pen or Bezier
  the placed segments show in the outline colour new curves get, the
  last node filled blue and the others hollow. C held while pulling a
  handle makes a cusp even when pressed before the drag, and no longer
  aligns the selection. The snap screen tips also name the grid and
  guidelines.

- VBA macros: the object model they are written against (Application,
  Document, Page, Layer, Shapes and ShapeRange, Shape with Fill,
  Outline, Transparency, Curve, Text and its views, Color, the cdr*
  constants), with lengths in the document unit and the reference
  point (docs/behavior/vba.md); constant values and parameter names
  checked against the published developer reference. Ids minted by
  scripts no longer collide with the drawing's own after a run that
  groups or adds pages.

- VBA macros: a compiler from VBA modules, classes and form code to
  scripts over the TraceDraw object model, with the VBA run-time library,
  Collection and Scripting.Dictionary.
- `.cdr` saving writes only the versions checked in other programs (14
  to 27). Save on a drawing opened from an older version asks to choose a
  version first ("Older file version", then Save As on version 14)
  instead of writing the old version again. The command line writes
  version 27 by default and the experimental 7 to 13 only with
  `TRACEDRAW_CDR_EXPERIMENTAL=1`; the MCP `save` tool takes 14 to 27.
- Help: a user guide of 120 pages (getting started, lines and shapes,
  objects, color, effects, text, templates and styles, pages, bitmaps,
  printing, web graphics, file formats, customizing, reference) with
  screenshots of the dialogs and panels, published with the browser
  version in every interface language, with contents, search, previous
  and next, light and dark and right-to-left Arabic. Every "?" button
  opens the page of its dialog or options page, F1 the page of the open
  dialog, panel or tool, and Help > Help Topics the first page, in the
  interface language.
- Wrap Paragraph Text now works: paragraph text flows around the objects
  marked to wrap it that lie over its frame, along their outline or
  their bounding box, with the text on the left, the right, both sides
  or above and below, and a text wrap offset (the arrow beside the
  property bar button). Drawing, printing, exports, clicks and the caret
  follow the wrapped lines.
- Alignment, spacing, indents, bullets, drop caps, tabs and hyphenation
  are kept per paragraph: while a text is edited they change the
  paragraphs of the selection, with the whole object selected every
  paragraph (only the settings changed). Older drawings open with their
  object-wide formatting on every paragraph; `.cdr` files keep each
  paragraph's alignment both ways.
- The context menu's Wrap Paragraph Text is a submenu with the seven
  wrap styles and the offset. The Shape tool's spacing arrows change
  every paragraph, keeping their differences. `.cdr` files keep each
  paragraph's line spacing from version 16, and paragraph spacing
  before and after is read from version 16 style strings.
- Align to Baseline Grid puts the baselines on the page's baseline grid
  (from the page top and the grid's start offset), so frames line up
  with one another.
- Alignment guides (View > Alignment Guides, Alt+Shift+A) now work:
  while moving, drawing or resizing, lines show where the selection or
  the pointer lines up with other objects' edges and centres, the page
  or margin lines, and the move snaps there; equal gaps between
  neighbours snap and are marked, and a box being drawn or resized
  takes another object's width or height, marked on both. In the
  Multipage View the other pages' objects count too. Off by default (the code default was
  on). Options > Alignment Guides sets the object (the selection or
  each object), edges, centres, page, margins with offset and inset,
  equal spacing, equal sizes, colour and line style.
- Dynamic guides (View > Dynamic Guides, Alt+Shift+D) now work: guides
  at chosen angles run from the snap points the pointer acquired and
  from the line's start; points snap onto them and their crossings,
  with the distance and angle beside the pointer and optional tick
  marks. Options > Dynamic Guides sets the angles, tip, ticks, colour
  and line style. Both also sit in the standard bar's Snap To list.
- Text: Chinese, Japanese, Korean and other input methods work while
  typing on the canvas and in table cells (the text being composed shows
  at the caret). Break Apart (Ctrl+K) on artistic text splits it into
  lines, words or letters, each a text object of its own.
- A right click on a page tab opens the page menu: rename, insert a page
  before or after, duplicate, delete, switch orientation, page size and
  background.
- Blends of text and of shapes with several parts (letters, holes)
  morph part by part, so the steps between two texts read as letters.
- Pen and Bezier: a plain click after a pulled node gives a straight
  segment (the node becomes a cusp), clicking the last node again makes
  it a cusp, C while pulling a handle makes the node a cusp and Alt
  moves it, and dragging from the first node shapes only the closing
  segment. With no curve being drawn they add a node on a segment of the
  selected curve and drag its nodes. The pointer shows what a press
  does: start, continue, add a node, close, edit a node.
- Shape tool: nodes and handles are taken within a pick distance (6
  pixels, Options > Nodes and Handles), and a near miss no longer
  deselects the object. A rectangle, ellipse or polygon says what it is
  on the property bar and the status bar, apart from a curve.
- Linux: the window can be resized and maximized under every window
  manager (Xfwm4 kept it at one size) and fits small screens: a smaller
  minimum size, maximized at start when the kept size does not fit.
  Bars longer than the window slide with "«" and "»" buttons (and the
  mouse wheel), and dialogs taller than the window scroll (#168).
- VBA macro files are read (new `tracedraw-vba-project` crate): `.gms`
  and other compound files with a VBA project (modules, their kinds and
  code in the project's code page, references, protection, modules saved
  without source), exported `.bas`, `.cls` and `.frm`/`.frx` files, and
  the forms with their controls (labels, text boxes, buttons, check and
  option boxes, lists, frames, pages, images, spin buttons, scroll bars,
  tab strips). Projects can be written back as module files or as a
  compound file.
- PDF files keep text as text: it can be searched, selected and copied.
  The fonts go in as subsets (only the glyphs used), TrueType and CFF
  outlines alike, with a map back to the characters. Publish to PDF >
  Objects has Export all text as curves (off), Embed fonts in document,
  Embed base 14 fonts and Subset fonts below a share of glyphs. Fonts
  whose licence forbids embedding are written as curves, named on the
  status line and noted by the preflight. Text written as curves (and
  any curve in PDF and EPS files) keeps the font's exact curves:
  quadratic segments become the matching cubic instead of a misshapen
  one.

- Convert to Curves turns a table into a group of curves (cell fills,
  borders and text) and a symbol instance into its symbol's objects as
  curves, instead of a plain rectangle; a clip frame keeps its contents
  (frame and contents become curves and stay clipped); bitmaps are left
  as they are
  (the command is greyed for them, with a note in the status bar).
- Trim, Weld, Intersect and Join Curves take the bottom-most object as
  the target when the selection was made with a marquee; objects picked
  one by one keep the one selected last as the target.
- Colour Management: the Assign and Convert choices described each
  other. Assign keeps the colour values (the look may change), Convert
  keeps the look (the values change), in all 18 languages.
- Imported bitmaps come in at the resolution their file records (PNG,
  JPEG, TIFF, BMP), 96 dpi when it records none.
- Space finishes a curve drawn with the Bezier tool (and then switches
  to the Pick tool, as Space does).
- Edit > Clone makes a linked clone that follows its master's fill,
  outline and shape until the clone's own is edited, with Select
  Master, Select Clones, Revert to Master and Break Clone Link in the
  context menu (it used to make a plain duplicate).
- The feature list and guides no longer claim live colour styles or
  Find and Replace by properties; File > Open does not open bitmaps (File > Import does).
- VBA macros: a compiler from VBA modules, classes and form code to
  scripts over the TraceDraw object model, with the VBA run-time library,
  Collection and Scripting.Dictionary.
- VBA macros: the object model they are written against (Application,
  Document, Page, Layer, Shapes and ShapeRange, Shape with Fill,
  Outline, Transparency, Curve, Text and its views, Color, the cdr*
  constants), with lengths in the document unit and the reference
  point (docs/behavior/vba.md). Ids minted by scripts no longer collide
  with the drawing's own after a run that groups or adds pages.
- VBA macros in the app (desktop): Tools > Macros with the Macro
  Manager docker, Run Macro (Alt+Shift+F8), Load Macro Project and the Macro
  Editor (Alt+F11). Projects are kept in the settings folder and load at
  start-up; macros run on their own thread with MsgBox and InputBox
  dialogs, their user forms on screen with events, Stop and Esc, an
  error box with the VBA line and one undo step per run. Each macro can
  get a shortcut, and the Scripts panel runs VBA snippets too.

- Moving objects with the Pick tool shows the arrow with a dotted box,
  and the box gets a plus after a right click (the move leaves the
  original and a copy lands where the left button goes up). The pointer
  no longer vanishes after that right click.
- Ctrl+V pastes what was copied last: a picture or text copied in
  another program after a copy in TraceDraw is no longer ignored, files
  copied in a file manager are imported, and Ctrl+V works when the
  clipboard holds only a picture or files.
- Document Properties: language, title, subject, author, copyright,
  tags, notes and a rating list, applied on OK in one undo step, over a
  report of the file, the document, its colour settings, objects and
  points, text, bitmaps, styles, effects, fills, outlines, colour models
  and symbols. PDF files carry the language.
- Characters selected in an artistic or paragraph text being edited take
  a colour picked from a palette, the document palette, the Properties
  docker's fill or the Color docker as their own fill (right click: their
  own outline), each its own undo step; the rest of the text keeps the
  object's colours. Characters with their own colours are now drawn in
  them on screen and in bitmap, SVG and PDF exports, so the per-character
  colours of opened `.cdr` files show too (#187). The property bar's
  font, size and style change only what they change on a selection.
- Dragging in the Properties fill colour picker is one undo step and
  adds only the final colour to the document palette, when the button
  is let go (#188).
- A right click while dragging an object no longer drops the copy at
  once: the outline follows the pointer until the left button goes up.
- Files dropped on the window open or import: with a drawing shown they
  are imported into it in the middle of the view, otherwise each opens
  in its own tab (a picture gets a new drawing). `.psd` and every other
  format File > Open and File > Import read are supported.

## 0.9.4 (2026-10-19)

- Color Management Settings: Document and Default tabs, the drawing's
  current profiles, assign or convert new RGB, CMYK and grayscale
  profiles chosen from the profiles found on the computer, primary mode
  and intent; presets, colour engine, preserve pure black, map gray to
  CMYK black, open and import policies with warnings, spot colour
  definition.
- The standard bar's Options button opens the list of option pages; its
  buttons show their title, shortcut and what they do; two buttons open
  from and save to the drawing's Git repository.
- Object > Insert > Barcode is a three-step Barcode Wizard with a sample
  preview: Codabar, Code 39, Code 128, EAN-13, EAN-8, UPC-A, UPC-E and
  Interleaved 2 of 5 (with EAN and UPC add-ons); printer resolution, bar
  width reduction, magnification, bar height, wide to narrow ratio,
  check character and quiet zone; font, size, weight, alignment and the
  text options.
- QR codes are live objects: Object > Insert > QR Code places one at
  once, and Properties edits its kind of content (URL, email, phone,
  SMS, contact, text, location) and fields, pixel and background fills,
  pixel outline and pixel size, with Validate reading the code back.
- Effects menu: Flatten Effects, the effect groups, then the effect
  panels, each checked while open (Effect Picker moves to the Window
  menu); Object > Create keeps Arrowhead and Pattern Fill, which asks
  for a vector or bitmap pattern. The Picture Mosaic, Vector Mosaic,
  Extrude (pages with a type picture and Measure from), Envelope (24
  preset shapes in a grid) and Blend panels are laid out anew; blends
  gain spacing along a path, loop, along the full path, acceleration of
  sizes and clockwise or counterclockwise colours.
- Text on a path has its own property bar: text orientation (rotated,
  vertical or horizontal skew, upright), vertical placement (baseline,
  ascender, descender, centred), placement along the path, distance,
  offset, both mirrors and Straighten Text. Dragging a fitted text slides
  it along its path; refitting a text to another path of the same kind
  no longer shows the old letters.
- Text fitted to a closed path stands on its outside, upright and
  centred on its top, instead of starting at its right end on the
  inside; the Text tool starts text on a path when clicked on its
  outline.
- Font Manager: a window of its own with libraries (installed fonts,
  added folders, Favorites and collections), filters by status,
  embedding rights, technology, weight, width, style, scripts and
  OpenType features, a sample text and search, every face drawn in
  itself and grouped by family, details, and installing font files for
  TraceDraw.
- Text docker laid out in Character, Paragraph and Frame sections with
  the buttons that show it from a section on, Apply to default text and
  the row boxes: style list, range kerning, text fill, character
  background and outline rows, six alignments (no alignment and forced
  justification added), line spacing in % of the character height,
  points or % of the size, and the frame's background colour.
- Glyphs docker: recent glyphs, font and range lists, a grid of the
  font's glyphs with their alternates.
- Text > Writing Tools is one dialog with Spell Checker, Grammar and
  Thesaurus tabs; the grammar checker also flags "a"/"an" before the
  wrong sound and sentences with no end mark.
- Text > Custom Optional Hyphens: words hyphenated by hand break only
  there, turned on for text typed, pasted or imported with them.
- Publish to PDF opens PDF Settings with General, Color, Document,
  Objects, Prepress, Security and issues tabs: the file name, every open
  drawing or a page list as the range, the page size of the selection,
  and PDF presets saved with "+" and deleted with "-".
- Export lists formats as "EXT - Description (*.ext)", JPG, PDF, AI and
  PNG first; "Do not show filter dialog" skips the dialog next time, and
  PDF and AI take a title and tags.
- The Snap To list stays open while its boxes are ticked and has a close
  button.
- Bitmap Mask docker: ten colour slots with their own tolerance, the
  eyedropper and "Select colours from bitmap", saved and opened masks,
  and Hide selected or Show selected.
- Border and Grommet: a preview of the final page, Source (page or
  selection), Add border with type and size, Add grommets with Use
  border area, size and margin, placement on the corners and each edge,
  number or spacing, and Reset and help buttons.
- Dialogs are modal: while one is open, clicks, menus, dockers, the
  drawing and canvas typing wait until it closes.
- Create New Table asks for the number of rows and columns, height and
  width (in the document's units), with a help button.
- `.cdr` saving from version 16: the font table lists every face the
  text uses, and Save As has an "Embed fonts" box that stores the faces
  whose licence allows it (the command line with
  `TRACEDRAW_CDR_EMBED_FONTS=1`). Save As lists versions 27 down to 14.
- `.cdr` saving: version 14 is no longer marked experimental, and
  version 13 files carry the zero argument 12000 on layers, groups and
  objects, flag 0x40 on pages and an object box on the master page, as
  files of that version have them (the other program closed on them).
- Save As has a version list for TraceDraw files too: the `.tdraw`
  format versions, newest first, with the release that first wrote
  each; Save keeps the version a drawing was opened or saved in.
- `.cdr` version 15 is no longer marked experimental: files written in
  it open with every object and their text looks.
- Save As offers the newest `.cdr` version (27) first, drops the long
  version hint, and has an "Embed colour profile" box with the
  profile's name and size (version 16 and later).
- `.cdr` saving before version 16: the font table lists each bold or
  italic face the text uses, so bold text no longer opens in a
  stand-in font.
- `.cdr` saving in versions 7 to 15: text records are laid out as files
  of those versions hold them, one byte a letter from version 12 (text
  was dropped as an invalid object, or the other program closed), each
  text's frame numbered as its object, and curves end with the 8 bytes
  those versions keep after the nodes (rectangles, written as curves,
  were lost); layers and groups carry the zero argument 12000 from
  version 14, as files of versions 14 to 27 do; text before 16 takes its font, size,
  weight, colour and alignment from a style of its own (it all came out
  in the default style); curve nodes carry the marks of every version's
  files (filled curves before 16 were dropped).
- `.tdraw`: drawings whose grouped curves keep node types open again
  (they were refused with "expected usize").
- Save As lists each `.cdr` version with the release that writes it
  ("v13 (X3)", "v27 (2026)"); versions 23 to 27 are no longer marked
  experimental (files written here open in the 2026 release).
- `.cdr` saving in versions 13 to 15: no invalid object when a shape is
  partly transparent (the transparency is not kept in those versions
  yet); version 15 text carries the argument its files have.
- `.cdr` saving: images show in other programs (bitmap objects now carry
  their pixel size and image box; files opened without their images).
- `.cdr` versions 23 to 27: files open (version 23 files were refused),
  and saving offers them too (experimental: written with the records of
  version 22, which those versions share; not yet checked to open in
  other programs).
- `.cdr` version 21: the page background colour, hidden objects (kept,
  not drawn) and facing pages (objects were half a page off) are read;
  saving in version 21 or 22 keeps the background and hidden objects.
- `.cdr`: a page's layers come in the right order (the top layer was
  drawn at the bottom, under a full-page background).
- `.cdr`: arcs of ellipses drawn downwards turn the right way (smiles
  came out as frowns).
- `.cdrt` templates of version 21 open (form type `CDT`).
- `.cdr` saving: the layers are written top first, and textures and
  pattern fills, not written yet, go out as their mean colour instead of
  their first one.
- `.cdr` versions 13 to 15: artistic text no longer lands metres off the
  page when its record holds a number where later versions keep a place.
- `.cdr` versions 7 to 11: letters take the style their character
  record numbers, so a word with one letter in another font or a big
  first letter of a paragraph comes out right (it went to the end or
  vanished).
- Text in the Symbol font draws its Greek letters, arrows and math signs
  when that font is not installed (it showed accented Latin letters).
- A big first letter no longer opens up the spacing of every line of
  its paragraph.
- `.cdr` saving: a clip frame keeps the fill of its frame (it was lost).
- CMX files whose chunk sizes run short (pattern files of version 7)
  open; they came up empty.
- `.cdr` saving: paragraph text is written as one artistic text per line
  in its place, so it no longer runs together on one line; the frame
  itself is not written yet.
- `.cdr` saving: images keep their see-through pixels (they were put over
  white).
- The Print dialog has tabs: General (destination: system printer, PDF
  or PostScript file; orientation; current document, page, page list or
  selection; copies with collate; print as bitmap), Color (separations,
  spot colours as CMYK), Composite (text in black, overprint black),
  Layout (as in document, fit, scale, bleed limit, imposition), Prepress
  (invert, mirror, file information, page numbers, crop, registration,
  calibration), PostScript (language level) and the preflight's Issues.
- Print Merge is laid out around its records: Import File, Add Column
  (name and type), Configure Columns, View Single Record, record
  navigation, Save Data Source and Finish.
- `.cdr`: an object whose transform holds several matrices takes all of
  them, one after the other (only the first was used, so labels of a
  version 5 chart piled up in one place); version 5 text no longer gets
  an empty line after every line break.
- `.cdr` version 3: envelopes bend the shapes they hold (perspective
  charts and lettering); text under an envelope keeps its matrices.
- `.cdr` version 3: two-colour pattern fills (from the file's tiles);
  PostScript texture and vector pattern fills, in every version that has
  them, show a stand-in of grey stripes instead of nothing.

- `.cdr` files of versions 3 and 4 open properly (they opened blank or
  out of shape): version 3 objects with their own matrices, fills,
  fountains, outlines with caps and arrowheads, groups and artistic text;
  version 4 matrices and colours; arrowheads of both; curves of glyph
  outlines. Numbered library colours become spot inks coloured from the
  file's thumbnail.

- File > Print Preview shows the sheets as they print: imposition,
  marks, mirror, invert and each separation plate, with the print options
  beside them and zoom (it showed the page full screen).
- Print Merge has a record grid: fields and records are added, edited
  and removed in the dialog and saved as CSV; fields can be counters,
  pictures or QR codes. Merged pages follow the records' order (they came
  out reversed).

- `.cdr` files of versions 1 and 2 (the `WL` files before RIFF) open:
  rectangles, ellipses, curves, text with its font, size, alignment and
  line spacing, uniform fills and outlines in the stacking order of the
  file (they were refused).

- An outline set behind the fill is drawn under it also when it is the
  object's only outline (it was drawn over the fill, so thick white
  outlines thinned or hid letters).

- CorelDRAW templates (`.cdt`, `.cdrt`) open as untitled drawings and can
  be imported.
- Save As says that programs from 2021 on open `.cdr` version 15 and newer
  only.

- `.cdr` form-type letters skip `O` too: a `CDRR` file is version 25 (it
  was labelled 26).

- The B-Spline tool places control points: the curve runs smooth near
  them, with the dashed control polygon shown while drawing (it drew
  straight segments through the points).

- `.cdr` files of versions 5 and 6 show their text (it was skipped), and
  version 5 bitmaps their picture; version 3 files open (assumed layout).
- Graded transparency is saved as a gradient in `.cdr` versions 16 and
  later (it was saved as a uniform one).
- `.cdr` reading caps what a file may inflate to (512 MB a member, 1 GB of
  compressed blocks in all), so a decompression bomb cannot exhaust
  memory; the reader forbids unsafe code.

## 0.9.3 (2026-10-17)

- Wireframe view draws every object as a hairline: no outline widths,
  fills, opacity or effects, bitmaps as grey boxes (#122).
- A fill or outline colour given to a group reaches the objects in it,
  so a traced bitmap takes a palette colour with one click (#123).
- Outline widths can be typed: the property bar's width is a box with
  the usual widths in its list, and the Properties width box applies
  what is typed (#124).
- Selection handles hide while text is typed or edited (#125).
- The first built-in gradient runs between the current colours, the
  last fill and outline colours picked (#4).

- Wireframe hairlines take each object's outline colour (black without
  one), as in the reference screenshot of #122.
- Tracing a black and white logo keeps white shapes white: the colour
  quantising no longer averages black and white to grey.

- The Transformations panel has a new layout:
  spin boxes with the unit after them, the reference point under the
  check box with the direction picture beside it, a line before Copies.

## 0.9.2 (2026-10-17)

- `.cdr` files: graded transparency (version 16+ files) is read as a
  transparency mask, so highlights fade as drawn, and colours of model 25
  (library colours) take their look from the file's preview instead of
  black. Save As marks the `.cdr` versions before 16 experimental: their
  files may not open in other programs yet.
- Clip containers of version 12 `.cdr` files show their contents (the
  objects drawn inside a shape were missing).
- Vector pattern fills of `.cdr` files are read: the pattern's drawing,
  stored in the file as a CMX file, tiles the shape (they had no fill).

## 0.9.1 (2026-10-17)

Version 15 `.cdr` files read their text in full, `.cdr` files saved for
versions 12 to 20 carry the configuration record those versions write
(text saved in versions 16 to 22 now opens in other programs), and the
Transformations panel's Position page picks a direction for copies.

- `.cdr` files of versions 13 to 20 saved by TraceDraw carry the whole
  document configuration record those versions write (it held only the
  page size, and other programs refused the file), version 12 files the
  8152-byte record of that version, and version 16 text has the record
  header that version writes (#98). Text saved in versions 16 to 22 and
  groups in 17 to 22 open in the program that wrote the samples;
  versions 7 to 15 do not open yet.
- Version 15 `.cdr` text reads in full: style records with the 0x20 flag
  byte no longer cut the text off (55 text objects of one file were
  dropped), each letter takes the style its record names (sizes and
  fonts of mixed runs) and artistic text whose coordinates hold an id
  is no longer placed 41 m away (#116).
- Transformations panel, Position page: with Relative position on, the
  3 x 3 boxes pick a direction and fill X and Y with the selection's
  width or height that way, so Apply with copies lays them side by side;
  the values follow the selected object. With nothing selected the whole
  page is pale (#115).
- Version 16 `.cdr` text is written one byte a letter, as files saved for
  that version hold it (UTF-16 letters were read as half the text), and
  such files are read correctly (#98).

## 0.9.0 (2026-10-16)

`.cdr` files of versions 16 and later saved by TraceDraw with text and
curves now open in other programs: a drawing saved as version 21, with
text, text on a circle and curves, opens complete and as drawn. Version
7 to 13 files are compressed and summarised as those versions write
them, and a drawing's CMYK profile is stored in its package. Also:
outlines take every fill kind as their paint, gradients repeat, mirror,
invert and follow a shape's outline, five landscape gradients, and AI
options with one service for drawings and pictures.

- `.cdr` files of versions 7 to 13 saved by TraceDraw are compressed
  (`cmpr`) and end with the summary (`sumi`), as files of those versions
  are (#98).
- `.cdr` files of versions 16 and later store the drawing's CMYK colour
  profile in the package and name it (#98).
- `tracedraw-cli` ends quietly when the program reading its output stops
  reading (a pipe into `head`, say).
- `.cdr` files of versions 16 and later saved by TraceDraw open in other
  programs again: text frames are numbered as their objects, text
  carries its complete default style and run records with their letter
  counts, and curve nodes carry the marks those versions write, so curves
  also keep their fill (#98).
- `.cdr` files of versions 11 and later saved by TraceDraw give every
  text the line breaking tables files of those versions carry after it
  (`txtj`), as other programs write them (#98).
- `.cdr` files of versions 18 to 22 saved by TraceDraw give every
  object's transform the tag files of those versions carry (#98).
- Outlines take every fill kind as their paint (none, uniform, gradient,
  pattern, texture, mesh) with the fill editors; SVG and PDF export them
  (issue #4).
- Gradients: repeat (sawtooth or mirror, with a cycle count), bilinear,
  invert colours and a shaped type that follows the object's outline;
  `.cdr` files get the closest plain gradient (issue #4).
- Five landscape gradients among the built-in ones (issue #4).
- Tools > Options > AI has one AI service instead of separate drawing
  and image services: the Service page sets the preset, address, text
  model, one API key and, when the service can also make pictures
  (OpenAI, Google Gemini, or an OpenAI-compatible address of your own),
  the image model. With a service that makes drawings only (Anthropic,
  Ollama, LM Studio, llama.cpp, vLLM) AI Generate's pictures, Remove
  Background and AI Art Styles stay greyed, and their tip says why.
  Preferences with the old separate services are folded into the one
  service on start, keeping remembered keys.

## 0.8.0 (2026-10-15)

`.cdr` files saved by TraceDraw now carry everything files of their
version hold, so other programs can open them (a version 17 file was
reported unreadable); version 12 text, centring, line spacing and text
on circles, radial and square gradients and brushed strokes open as
drawn, and a large file that took minutes opens in a fraction of a
second. Also: the Pixels view shows document pixels when zoomed in, AI
Generate gathers all AI drawing in one panel, Window > Panels in groups,
Bitmaps > Resample, bitmap effects as lenses, Put on Plane in
perspective drawing, Save Bitmap As, a frame around the selection and
renaming a page from the Objects panel.

- `.cdr` files saved by TraceDraw carry what files of each version hold
  so other programs can open them: the document preferences and default
  styles (versions 16+), the summary, names and thumbnails of versions 14
  and 15, and the picture and info lists of RIFF files (before 14). A
  version 17 file saved this way was reported unreadable (issue #98).
- Save Bitmap As in a bitmap's context menu writes its pixels to a PNG.
- Shift + double-click on the Rectangle tool draws a rectangle around
  the selection.
- `.cdr` radial and square gradients are no longer inside out: files
  list their colours from the edge to the centre (written that way too).
- `.cdr`: strokes an artistic brush draws along a curve (linked groups)
  are placed with their group's transform; in a version 10 logo they
  floated far from the drawing.
- `.cdr` files saved as version 16 or later are complete packages: a
  media type member first, the manifest naming the drawing, metadata,
  layer and object names, colour settings and a picture of each page.
  Files saved without them were reported not to open in other programs.
- Opening a `.cdr` file with long curves is fast again: a 4 MB tutorial
  file took over three minutes, now a fraction of a second (node types
  were worked out once a node instead of once a curve). Saving `.cdr`
  and showing nodes in the Shape tool had the same slowdown.
- View > Pixels shows the document's pixels when zoomed in: the drawing
  is rendered at the document resolution and enlarged as square blocks,
  lined up with the pixel grid, so edges look as they will once exported.
- Objects panel: double-click the page (or its context menu) to rename
  it.
- `.cdr` version 12 text: accented and other letters stored as two bytes
  among one-byte letters are read (such text used to come out as Chinese
  characters); centred text is centred (it was read as right-aligned);
  line spacing given as a distance and negative character spacing are
  read, so multi-line text no longer collapses onto one line; text fitted
  to a circle without readable settings follows the circle its box rests
  on instead of running straight.
- `cdr_preview_check` example: puts a `.cdr` file's own preview picture
  beside our render of the same area, for checking imports.
- Window > Panels in groups, in the usual order of a Dockers menu
  (Properties, Objects, Pages, Comments... then Transformations, Guidelines,
  alignment, shaping, text, colour), with the effect panels in an Effects
  submenu; the menu no longer runs off the screen.
- AI in one place, as in the AI Generate docker of other editors: AI
  Drawing is now its fourth mode, Draw with words (also Object > AI
  Generate > Draw with Words), and no longer a separate menu item and
  panel.
- Bitmaps > Resample: width and height in pixels or percent, the
  resolution, aspect ratio kept or not, and four methods, among them Hard
  pixels for pixel art and Enlarge with detail.
- Draw in Perspective: Put on Plane lays existing objects onto the
  active plane.
- Bitmap effects as lenses: a Bitmap Effects lens runs any of the bitmap
  effects over what lies beneath it (Lens panel, or Effect Picker >
  Apply as a lens).

## 0.7.0 (2026-10-15)

Start faster, bring your own AI and draw in new ways: a New From
Template dialog with designed templates; AI Generate (create, remix and
vectorize pictures), AI Art Styles and Remove Background through the
online or local services you connect with your own keys; painterly
brushes, variable-width outlines and drawing in perspective; focus
mode, a Multipage View, review comments, an Export panel for many
assets and a Font Sampler; the desktop's text size and high contrast
themes followed; a darker dark theme; and `.cdr` files saved by the
newest versions opening with their fills and spot colours.

- Faster textures and other per-pixel fills on screen (#4): worked out
  on all processor cores, on a coarser grid when they cover much of the
  window (exports keep every pixel), with their end colours converted
  once instead of per pixel, and kept between redraws, so dragging other
  objects or changing the selection does not compute them again. A
  window-sized cloud texture draws about three to four times faster on
  two cores, and again from memory when unchanged.
- AI, with the user's own API keys: Tools > Options > AI turns AI
  features on or off and sets up a drawing service and an image service
  (Google Gemini API or OpenAI Images API or compatible), each with Test
  Connection. The AI Generate docker (also a button on the standard
  toolbar) creates pictures from a prompt with aspect ratio, style and
  palette presets, remixes the selected bitmap, and creates vectors (a
  generated illustration traced into curves). Remove Background (Bitmaps
  menu, property bar, context menu) keeps a picture's subject and makes
  the rest transparent. AI commands are greyed out, and the dockers show
  a banner with Set Up AI, while their service is not ready.
- Fountain fills offer 24 built-in gradients (metals, sunsets, skies,
  pastels and more) as swatches in the fill editor (#4).
- Texture and other procedural fills work out only the part in the
  window, so zooming into a cloud-filled shape stays quick (#4).
- Outlines can be painted with a gradient (Properties docker, outline
  Paint: Gradient), without converting them to curves first (#4).
- Options > Customization > Appearance: Shadows under menus and dialogs
  can be turned off (#4).
- AI Generate docker, more complete: mode buttons and an information
  page (services, models, costs, what is sent); a reference picture from
  the selection or a file (the source of Remix, an example for Create)
  with "match reference image"; ready-made remix requests (mockup, photo
  restoration, colorize sketch, replace background, change season,
  recolor); model notes; Reset per section; more aspect ratios; vector
  styles; Create vectors also as an SVG drawing from the drawing service,
  imported as editable shapes; Results replaced by each Generate, with
  Insert into drawing; Generate kept at the bottom. Object > AI Generate
  has every mode and Remove Background.
- AI service presets (Anthropic, OpenAI, Google Gemini, and Ollama, LM
  Studio, llama.cpp and vLLM on the same computer for AI Drawing; LocalAI
  for pictures) fill in the address, which stays editable. A new "AI
  features" topic in the Hints panel explains online and local services.
- Focus mode: double-click a group or clip frame, or Bring into Focus
  in the context menu, to edit its objects with the rest of the drawing
  faded (Tools > Options > TraceDraw > Edit sets how much); a breadcrumb at the
  top of the window goes back up, as does Esc.
- Grouping objects from several layers keeps their drawing order.
- Text > Font Sampler: compare fonts with your own sample text, side by
  side, and apply one with a click.
- Bitmaps > AI Art Styles: repaint the selection as an oil painting,
  watercolour, pencil sketch, ink drawing, chalk pastel, pop art, woodcut
  or stained glass through your image service.
- View > Draw in Perspective: one-, two- and three-point grids; what you
  draw lands on the ground, a wall or the ceiling in perspective and
  stays editable.
- View > Multipage View shows every page in a grid; click any page to
  work on it in place.
- Painterly Brush tool (Brush Strokes flyout): strokes that look painted
  with chalk, charcoal, pastel, ink wash, watercolour, dry brush,
  bristles, fur, spray, speckles, clouds or silk ribbons; still curves
  that the Shape tool reshapes.
- Appearance > Coloured tool icons: each toolbox group in a colour of
  its own (#4).
- The interface follows Windows' text size and high contrast themes: a
  High Contrast theme in the desktop's colours, also selectable by hand
  (#4).
- Effects > Effect Picker: a floating window with every bitmap effect
  as a picture, searchable, by group or recently used; a click applies
  it (#4).
- Comments docker: review notes pinned to a point, a box or an arrow on
  the page, with replies, resolve and reopen; saved with the drawing.
- Export docker: list pages and selections as assets, each with its
  own name, format and resolution, and export them all at once.
- Variable Outline tool (Shape tool flyout): drag across an outline to
  make it thicker or thinner at that point, one side with Alt; widths on
  the property bar.
- The dark theme's picked items show white text on blue.
- README: new screenshots (templates, AI Generate, painterly brushes,
  drawing in perspective) and some in the dark themes.
- File > New From Template is a dialog: search, a list of where
  templates come from and one of categories, thumbnails with a size
  slider, and a details pane. Besides blank pages it offers designed
  templates (business cards, flyers, a poster, a letterhead, a
  certificate, an invoice, a banner, a social post and a label) and the
  user's own `.tdt` files. The Welcome Screen's Templates page shows them
  as thumbnails.
- Dark theme: darker bars and menus, a grey desktop around the page
  (unless another desktop colour is set) and blue for the tab, row and
  tool in front.

## 0.6.0 (2026-10-15)

Closer to everyday drawing work: menus with pictures and access keys,
themes from light to dark, a Customization dialog with command bars,
commands and their shortcut keys, bitmap effects that keep vector
objects editable, symmetry editing, the Shape tool cropping pictures,
drawings opened from and saved to GitHub, and `.cdr` files saved by the
newest versions opening with their fills and spot colours.

- `.cdr` files saved by the newest versions (also when saved for older
  versions) keep their fills: text and backgrounds no longer go missing.
  Colours from spot colour libraries, which the file names by number
  only, take the look the file's own page picture shows instead of
  black, and character spacing is counted in spaces, so widely spaced
  headings fit their frames (#67).
- Fountain fills: Import Gradient reads a `.ggr` gradient file into the
  stops (#4).
- `.cdr`: an object embedded from another program with no picture to
  show (version 21 media objects) gets its own warning instead of
  "object type 0x0a not modelled yet".
- Menus, lists and dialogs open at once, without fading in, and a
  folder called `tracedraw-settings` next to the program makes a
  portable copy that keeps its preferences there (#4).
- Shape tool on pictures (#60): a picture shows a node at each corner;
  dragging one crops the picture to the reshaped outline (a clip frame
  whose frame keeps editing as a curve). A double click no longer turns
  a picture, text, group or clip frame into a curve, which made a
  picture disappear.
- Export: the pixel size of bitmap formats can be typed (the resolution
  follows), and resolutions go down to 1 dpi, for small icons (#4).
- Pictures on the rows of every menu: File, Edit, Layout, Object, Effects,
  Table, Tools and Help join Text, Bitmaps and Window (submenu rows
  too). Each bitmap effect's row shows the effect applied to a small
  landscape. Welcome Screen lost a picture that did not belong to it.
- Themes: Light, Medium Light, Medium Dark and Dark in Options >
  Customization > Appearance > Color, and a light / dark button at the
  right end of the status bar. Turning proof colours on or off says so
  in the status bar.
- Menus have access keys: one letter of each title and row is
  underlined; with a menu open, pressing it picks the row (or opens its
  submenu), and Alt with a title's letter opens that menu (except the
  Alt letters commands use: E, S, X, Q, Z). Shortcuts wait while a menu
  is open.
- `.cdr` version 12: guidelines are read into the pages' guides (they
  were skipped objects), pictures embedded from a paint program show in
  their place, spot library colours show as black at their tint instead
  of solid black, and a text on a path finds its path when its box ends
  just short of the path. The empty compressed block every version 12
  file starts with no longer logs a warning.
- Options > Customization > Commands: a category list (every menu, the
  toolbox, or all) and a search above the commands with their pictures;
  beside them a General tab (name, where the command is, its keys) and a
  Shortcut Keys tab (press the new keys, the current ones with Delete,
  Reset to Defaults). Tool shortcuts are set the same way.
- Options > Customization: pages in the order Appearance, Command Bars,
  Commands, Color Palette. Appearance has Center Dialog Boxes (off,
  dialogs move by their title bar and open where they were left), a
  size slider with a percent field and Reset to Default, and a Color
  group. Command Bars lists the bars in a box with Reset, and Lock
  Toolbars beside it. On the palette, with a right click setting the
  outline colour, holding the right button for a second opens the
  palette menu.
- Effects menu: the groups in alphabetical order, Picture Mosaic before
  Vector Mosaic, and Effects > Blur > Smart Blur (smooths flat areas,
  keeps edges).
- Text, Bitmaps and Window menus: pictures on the rows; Tabs, Columns,
  Bullets and Drop Cap work with no text selected and set what new
  paragraph text gets; Fit Text to Path with no path selected waits for
  a click on the path (and starts a new text on it when no text is
  selected); Change Case is a dialog; Show Text Frames moved to
  Options > Text; the trace submenus are greyed without a bitmap; the
  drawing in front has a dot in the Window menu.
- Help > Updates checks in the browser too (it opened the releases
  page), and the check shows a turning ring with Cancel.
- Bitmap effects (Effects > Blur, Texture, Creative and the others) work
  on vector objects: the object stays editable and is drawn with its
  effects; Flatten Effects makes it a bitmap.
- Symmetry: Create New Symmetry goes straight into an editing mode (the
  original outlined in red, a bar with Finish, mirror lines, centre and
  angle at the top left of the window) instead of a dialog; Edit
  Symmetry (Alt+E), Finish Editing Symmetry (Alt+Shift+E), Remove
  Symmetry (Alt+Shift+S) and Break Symmetry Link (Alt+X) in the menu.
- File > GitHub: open a drawing from a GitHub repository, save it back
  as a commit with a message, and see and open its earlier versions.
- `.cdr` version 22: connector lines (object type 0x26) are read; they
  were skipped.
- `.cdr` versions 7 to 12: arrowheads are read and placed on their lines,
  artistic text stands where the file puts it (it was drawn far off the
  page), and version 12 text stored as one byte a character reads right.
- `tracedraw-cli inspect file.cdr <chunk>` prints the data of every
  chunk with that id in hex.
- The browser version says at each start that it is the same Rust
  program as the desktop app, compiled to WebAssembly, slower in a
  browser, and links to the desktop app.

## 0.5.0 (2026-10-14)

More for print work, and better `.cdr` files:
spot colours through to separations, opacity masks, several fills and
outlines on one object, an effect stack, 3D extrude with lights and
bevels, CMX import, session recovery after a crash, and scripts that
add file formats and can be debugged. Pictures stored in CMYK inside
`.cdr` files now open with their real colours (in 0.4.0 they looked
like negatives).

- Script debugger: breakpoints in the Scripts panel's margin, Debug,
  Continue, Step, Stop, and expressions evaluated where the script
  paused.
- Help > Updates: a Linux AppImage installs the new release in place
  (Install), after checking the download's size and SHA-256 digest.
- Script filters: a script that defines `TraceDrawFilter` becomes a file
  format of File > Import and File > Export (Scripts panel, Install as
  Filter); `Shape.PathData` gives an outline as SVG path data.
- Help > Highlight What's New marks the menu rows new in this version.
- `.cdr`: arrowheads of newer files (`arrx` records) are read, and the
  arrowhead ids of outlines are read from their right place, so dimension
  lines and arrows keep their heads.
- Colour books: the spot entries of an `.ase` palette are spot colours;
  applying one adds its ink to the drawing.
- Welcome Screen, What's New: the news covers the effect stack, 3D
  extrude, spot colours and opacity masks, and the language setting's
  place (Tools > Options > Global > General).
- Effects docker: the selected object's effect stack, each effect
  switched off and on, moved up or down, or removed.
- Several fills and outlines on one object: Add Fill and Add Outline in
  the Properties docker paint further layers over the object's own.
- Opacity masks: Object > Clip to Frame > Opacity Mask makes a clip frame's
  brightness set how much of its contents shows (Invert Mask the other
  way round), on screen and in SVG and PDF output.
- Shelf: Add Colours keeps a drawing's document palette and colour
  styles as an item, which placing adds to another drawing.
- A user guide in `docs/guide/`.
- Scripts: `Shape.Effects.Add` adds any effect in its `.tdraw` form,
  with `AddExtrude` and `AddBlockShadow` as shortcuts.
- Help > Updates checks for a newer release and offers its download;
  the same check runs once a day at start (Options > Global > General).
- Spot colours: named inks in the Colour panel, applied at a tint,
  written to PDF as Separation colour spaces and printed on a plate of
  their own in separations.
- `.tdraw` files record their format version; older files are brought up
  to date on opening and newer ones open as far as this version knows
  them (see `docs/tdraw-format.md`).
- Every object has a stable identity, kept across saving, undo and
  moves between drawings; copies get their own.
- Scripts: `Shape.Curve` gives the curve's subpaths and nodes (position,
  type, move, delete, add), `Shape.Effects` lists, removes and clears
  effects.
- Session recovery: drawings auto-backed up by a session that ended
  without closing are offered at the next start; a clean exit removes
  its auto-backups.
- Extrude: the body turns in space (x, y, z), takes up to three lights
  with their elevation, and a bevel on its front edge (panel, and the
  bar's 3D menu).
- CMX import: text comes in as artistic text at its place, font, size
  and colour.
- Options > Tools > Pick: Marquee selects touched objects (off by
  default) makes a marquee take the objects it only touches, Alt then
  taking only those wholly inside (#2).
- Window > New Window: another window of the same drawing, "name:1" and
  "name:2", each with its own view and selection; edits, undo and saving
  are shared, and closing one window keeps the drawing open in the other.
- CMX import: arrowheads come in at the size and place the file gives
  them, open heads as lines and start heads the right way up, as the
  files' previews show.
- `.cdr` files: node types (cusp, smooth, symmetrical) are read and
  written, so a cusp node with aligned handles stays a cusp.
- Shape tool: a node's type is kept with the curve once chosen, so C and S
  switch it both ways and a cusp node with aligned handles moves its
  handles apart.
- Node tracking (Options > Display, off by default): the Pick tool drags
  the nodes of a selected curve.
- Pick tool: Ctrl+click goes one group level further in at each click;
  Ctrl keeps the marquee square; Space drops a copy while scaling, turning
  or slanting too. Ctrl+Space switches to the Pick tool from any tool.
  Selection groups: Ctrl+number stores the selection, the number selects
  it again (twice: and zooms), Alt+number adds; saved with the drawing.
  Ctrl keeps the Freehand Pick selection a rectangle.
- Shape tool: the arrow keys move the chosen nodes; Home and End choose
  the curve's first and last node; C and S switch a node's type back and
  forth (cusp and smooth, symmetrical and smooth).
- The interface in six more languages: Chinese (Traditional), Italian,
  Dutch, Polish, Turkish and Czech, eighteen in all. Taiwan, Hong Kong and
  Macau locales choose Traditional Chinese.
- Import CMX exchange files: pages, layers, groups, curves, rectangles,
  ellipses, uniform and fountain fills, outlines with dashes and
  arrowheads, embedded images and clips. Text, lenses and bitmap fills
  are not read yet; the import report names what was skipped.
- Snapping: the names beside snap marks are in lower case (node, edge,
  midpoint), and the edge a drag snaps to lights up in blue.
- Shape tool: a curve being edited shows its outline in the node colour
  and its first node as a triangle; the direction arrow is off by default.
  Control handles are dashed lines ending in solid arrow heads, also
  while a curve is drawn.
- Preferences saved by older versions turn on treating all objects as
  filled once, so a click inside an unfilled object selects it there too.
- Dragging the selection (move, scale, turn, slant) leaves the objects in
  place and shows their outline where they would go, on a white halo that
  stays visible over dark fills. A move grabbed on an edge mark carries
  that point along other objects' edges; turning and slanting snap to
  other objects' points like scaling does.
- Starting to draw a new shape lets go of the selection at once.
- Selection handles: solid black squares just outside the selection box,
  which has no frame any more, so the objects' outlines stay in view; a
  handle scales from where it was grabbed. In rotate mode the arrows have
  solid heads and stand outside the box too.
- The Pick tool marks the selection's node, midpoint, centre or edge under
  the resting pointer, with its name, as during a drag.
- The Pick tool treats all objects as filled by default: a click inside
  an unfilled object selects it. Options > Tools > Pick and the property
  bar turn it off.
- Shape tool: a dragged node, control handle or shape node snaps to the
  grid, guidelines, the page and other objects like the drawing tools
  do; it ignored snapping (#3).
- Linux: after the splash screen the main window can be maximized and
  resized; it kept the splash screen's size as its maximum (#1).
- Linux: the interface is drawn a quarter larger at the default interface
  size (100 % is one interface point per pixel at the system's scaling);
  text was too small (#1). Tools > Options > Customization > Appearance >
  Interface size still sets it from 80 % to 200 %.
- Opening .cdr files: letters beyond Latin-1 (Czech, Polish, Turkish and
  other accented letters) in text of version 17 and later files read
  right; they showed as boxes and could break lines in the middle of a
  word. CMYK pictures show their own colours, converted through the
  colour profile the file carries; they looked inverted. Cut-out
  pictures keep their transparency instead of a white box. Paragraph
  text in a frame that was resized keeps its size and starts at the
  frame's top instead of being squashed or drawn above it. Rounded
  corners of rectangles in version 15 and later files have their real
  size, and shapes of version 14 and 15 files that have no outline of
  their own take their style's. Invert lenses of version 13 and later
  files show what lies beneath in its complementary colours instead of
  a solid box.
- Files with many large pictures open more than twice as fast (a 297 MB
  flyer: 20 to 8.6 seconds on two cores): pictures are decoded side by
  side and kept compressed more lightly, and a page keeps its decoded
  pictures between draws instead of decoding them again every time.
- Contour: the tool's drag draws a live contour (outside when dragged
  away from the object's middle, inside toward it); its bar used to wait
  for Apply and then made a static group of copies. The bar and the
  Contour panel share the controls: To centre, Inside, Outside, steps,
  offset, the corners (mitred, round or bevelled, round only before),
  the colour direction, the end colours and object and colour
  accelerations. To centre now spreads the colours over all its steps.
- Object > Copy Effect copies perspective, distortion, extrusion,
  contour, blend settings, lens, bevel and block shadow besides drop
  shadow, transparency and envelope; a distortion's centre, a
  perspective's corners and a vanishing point locked to the object sit
  the same way on the target.
- Blend keeps the end object as an object of its own: it can be moved,
  resized or recoloured and the steps follow, and selecting it picks the
  blend for the bar and panel. Blending used to delete the end object
  and keep a copy only inside the effect, so it could not be selected
  again; Clear Blend lost it.
- Segment Delete removes the stretch of an outline between the places
  where it crosses other outlines or itself, so overlapping lines can be
  trimmed back to their crossings; a drag removes every such stretch
  inside it. It used to remove one segment between two nodes, and a
  closed shape could lose its closing side by mistake.
- Crop: the area stays on the page with handles to move and resize it
  (and fields for its position and size) until a double-click inside,
  Enter or Crop applies it; Escape or Clear Crop Area drops it. It used
  to crop at once on release. Bitmaps are cropped (into a clip frame;
  Crop Bitmap led to a tool that skipped them), groups member by member
  (they were turned into one plain curve, losing their members), and
  open curves keep the parts inside instead of being closed.
- Extrude: six types (small and big, back and front, toward a vanishing
  point that can be locked to the object or the page, and back or
  front parallel), each with its depth; the body is coloured from the
  object, one solid colour or a shade per face, lit by one light. Only
  the faces that face the viewer are drawn, a curved side is one
  outlined surface instead of a row of slats, and every subpath is
  extruded (only the first was, so text and holes came out wrong). The
  tool's drag runs a parallel body along it or puts the vanishing point
  where it ends, its handle can be dragged, and the bar (its Apply used
  to add a separate static extrusion) and the Extrude panel share the
  same controls, showing and editing the selected extrusion.
- An outline that ends where it started (an ellipse) is treated as
  closed by Distort and Extrude.
- Distort: Push and Pull moves the nodes and bows the segments the other
  way (spikes or petals); it only scaled the object before. Zipper puts
  teeth on every segment, with Random, Smooth and Local; Twister takes
  whole turns, degrees and the direction. A drag is centred where it
  starts (the centre can be dragged afterwards), a Twister drag follows
  the turn of the pointer, and every distortion keeps the object's
  subpaths and open curves (they were merged into one closed outline).
  The bar shows and edits the selected distortion, with Center
  Distortion, Clear Distortion and Convert to Curves; its Apply button
  used to rewrite the object's outline instead of adding the effect.
- 3-point callout: a drag from the tip to the elbow works besides two
  clicks (the hint said so; only clicks were taken), the text goes on
  the side of the elbow it was placed on and opens with its sample text
  selected, so typing replaces it (it was typed after it). The leader
  and a new shape round the text (underline, rectangle, rounded
  rectangle or ellipse) follow the text as it is typed, resized or
  moved; a tip or elbow moved with the Shape tool stays. The tool has a
  property bar (outline width, line style, head at the tip, shape, gap),
  also for callouts selected with the Pick tool.
- Knife: a freehand cut besides the two-point line, a gap along the cut,
  and the pieces kept apart or as one object, from its property bar.
  Every separate piece becomes an object of its own, and open curves are
  split where the cut crosses them (they were closed into filled pieces).
- The Blend tool's bar sets the rotation and the object and colour
  accelerations besides the steps, for the next blend and live for a
  selected one, with Clear Blend and Break Apart. Dragging a slider in an
  effect panel (blend, contour, extrude and the others) or the drop and
  block shadow drags made an undo step per movement; a run of changes is
  now one step.
- Shape Recognition keeps shapes as drawn: a tilted rectangle or ellipse
  stays tilted (they came out level in the stroke's box), triangles and
  other four-sided shapes keep their corners (they became regular
  triangles and rectangles), and circles are told from squares by their
  area. Its property bar sets the recognition and smoothing levels;
  Sketch's sets joining to the selected curve, the distance and the
  smoothing.
- Connectors stay attached: move, scale or reshape an object and the
  connectors joined to it follow, in the same undo step; moved on its
  own, a connector is let go. They used to be plain lines. The connector
  tools have a property bar: straight, right angle or rounded (changing
  selected connectors too), the corner radius, and the outline of the
  selected ones.
- Dimensions: the tools have a property bar (precision, units, show
  units, prefix, suffix, text above, in or below the line, label size),
  shown also when dimensions are selected, and changing it redraws them.
  Dimensions are dynamic: scaled or skewed, they read the new length with
  upright text. The label is centred (it sat a quarter of the length to
  the left), extension lines run past the line on its side, and the one
  being placed follows the pointer. Parallel and Horizontal or Vertical
  take a drag as their hints said; Segment measures the segment clicked.
- The shaping brushes have their own bars: Attract and Repel, Smudge and
  Roughen showed only their name and could not change the nib size.
  Smooth has a rate, Smear and Smudge a pressure, Twirl a speed and its
  turn, Attract and Repel a speed and the mode, Roughen the spike
  frequency and size. A stroke is one undo step (two strokes in a row
  used to merge). The brushes work on each subpath they reach: before,
  brushing one part of a compound object joined all its parts into one
  outline, and Roughen closed open curves.
- Mesh Fill tool: its property bar sets the rows and columns (new meshes
  take them; the selected mesh changes and keeps its colours), Smooth
  colour, the selected node's transparency and Clear Mesh. A palette
  colour now colours the selected node; before, it replaced the whole
  mesh with a flat fill. Delete removes the grid lines through the
  selected node. Smooth colour now draws (it was only a checkbox): the
  colours flow through the nodes. The tool stays the Mesh Fill tool
  after making a mesh.
- Area Fill: the property bar chooses the fill (default, a colour, none)
  and the outline (default, a width and colour, none) of the objects it
  makes; objects with live effects count as drawn.
- Envelopes rebuilt. The Envelope tool keeps its own property bar
  (presets, the four modes, Add New, mapping, Keep Lines, Copy, Clear)
  instead of the distortion controls, and edits the frame itself. The
  modes now shape the frame (they were only buttons): Straight Line,
  Single Arc, Double Arc and Unconstrained, where the selected node's
  handles bend a segment alone. Double-clicking the frame adds a node,
  a node removes it. The mappings Horizontal, Original, Putty and
  Vertical change how the object fills the frame. Keep Lines and the
  mapping change the selected envelope at once. Envelopes from earlier
  files look as they did.
- Objects with live effects are picked and boxed where they are drawn:
  a click on the arch of an enveloped object selects it, and the
  selection box, the property bar's size and alignment take the warped
  outline and the objects the effects add.
- View > Proof Colors changes the drawing on screen: its colours go to
  CMYK and back, through the CMYK profile in use or the built-in model,
  so colours CMYK cannot print show duller. Before, the button only
  lit up.
- The object context menu's Frame Type works: Create Empty Clip Frame
  makes clip frames that stay in place when the object was turned (they
  were turned twice), No Frame turns empty clip frames back into their
  objects (it used to remove the outline), and Create Empty Text Frame
  replaces the object with an empty paragraph text frame of its size (it
  did nothing).
- Shaping: Trim keeps the objects that cut and Intersect keeps the
  originals, as their pictures always showed; Weld and the others
  replace them. The Shaping panel's "Leave original" boxes now do what
  they say (they were not used), each operation keeping its own choice,
  and the panel shows the operations as pictures with a large picture
  and a description of the one chosen.
- The Text docker has Character, Paragraph and Frame sections with
  header bars and buttons to open and close them (kept), the size list
  and bold, italic and underline as on the property bar, alignment
  pictures, and fields in rows. The new Frame section sets columns and
  their gutter, bullets, drop caps, hyphenation, fitting the text to the
  frame and where paragraph text sits from top to bottom: at the top, in
  the middle or at the bottom of its frame (new, saved with the drawing).
- Window > Cascade, Tile Horizontally and Tile Vertically put every open
  drawing in a window of its own, with a title bar, a button back to the
  tabs and a close button; the active one has its rulers and tools, the
  others show their page, and a click makes one active. Combine Windows
  and Dock Window bring the tabs back. Each drawing keeps its rendered
  page and its place in its window.
- Preferences are saved as they change (looked at once a second), not
  only when TraceDraw closes, so they last in the browser too (in its
  local storage) and after a crash. More of them are kept: the tool
  settings new objects take (polygon points, star sharpness, corners,
  spiral revolutions, table and graph paper size, eraser, text font and
  size, ...), what the window shows (grid, guidelines, page border,
  bleed, printable area, pixel and baseline grids, view mode, ...), all
  snapping toggles, and the Color and Align and Distribute dockers'
  choices.
- The Color docker has a color viewer (a saturation and brightness
  square beside a hue strip), the sliders and a palette's swatches, each
  shown or hidden by a button at the top; the selected object's colour
  sits beside the new one, and the new colour follows the selection. The
  HSB and HSL hue sliders count 0 to 360 degrees (they took the hue for
  a fraction of a turn before).
- The Align and Distribute docker has picture buttons. Align To (the
  active object, the page's edges, the page's centre, the grid) now does
  what it says, where before every choice aligned to the active object;
  Use Outline aligns the outer edges of the outlines. Distribute adds
  left, right, top and bottom edges to centres and gaps, and can spread
  the objects across the page (two objects are enough there); the
  Object menu lists the new ones too.
- The Transformations docker has picture buttons for its pages
  (Position, Rotate, Scale and Mirror, Size, Skew), a 3 x 3 reference
  point, Copies and one Apply: copies are made in one undo step, each the
  one before transformed again. Position and the rotation centre can be
  relative or absolute (from the ruler origin), the mirror buttons are
  applied with the scale, scale and size can stay proportional, skew can
  use the anchor point, and the size and absolute fields follow the
  selection. Alt+F8, Alt+F9 and Alt+F10 open Rotate, Scale and Mirror
  and Size as Alt+F7 opens Position, and Object > Transformations lists
  the five. Relative Position can be turned off again (it came back on).
- The Objects docker shows the page and the Master Page as a tree: the
  master layers, the guides (show, lock all) and the document grid (show)
  under the Master Page; each layer has a printer to turn printing and
  export on or off; the active layer, where new objects go, is in bold
  and is picked by clicking a layer (or an object on it); New Layer, New
  Master Layer (all pages, or odd or even from its context menu) and
  Delete (the selection, else the active layer) are at the bottom. A
  master layer can be moved, deleted and set to all, odd or even pages
  from its context menu, and its objects can be clicked, marquee
  selected and snapped to on every page it shows on.
- The Properties docker: its sections come open, in the order Outline,
  Fill, Transparency (new there: the Transparency tool's settings), the
  kind's own (corners, effects, character) and Summary, each with a header
  bar, and a row of buttons opens and closes them; the sections closed
  are kept. The fill kinds are pictures.
- The text property bar: the font size has a list of the usual sizes, the
  alignment is one button with its picture and a list, and Bullets, Drop
  Cap, Text Properties and Edit Text have buttons. The shape tools' bars
  (rectangle, ellipse, polygon, star) with an object selected end with
  Wrap Paragraph Text, To Front, To Back and Convert to Curves, as the
  Pick tool's bar does (Wrap Paragraph Text is new there too).
- The document palette stays a row under the drawing window wherever the
  colour palette is (on a side it was a column beside it). The colour
  palette starts at the bottom; moved to a side, along the top or
  floating, it stays there across restarts, in the browser too. Nightly
  builds that stood it at the right edge by default put it back at the
  bottom once.
- The browser version offers every family of the google/fonts collection
  (about 2,000 more), fetched from GitHub the first time a drawing's text
  needs one or the pointer rests on it in the font list; they show in
  the font lists with the download mark.
- A new font list in the property bar, the Text and Glyphs dockers and
  Options: type in the field to find a family (the name, then names
  starting with it, then the rest; Down, Up, Enter, Escape), the number
  of fonts and what to show (all, installed, in this drawing, to
  download), the five used last, every name drawn in its own font with
  an icon of how it is built (TrueType, OpenType, variable, came with the
  drawing, to download), the styles of each family under it, a slider for
  the sample size and a preview of "AaBbYyZz" with its guide lines.
- Dockers: the close button and a tab's Close Docker take the docker out
  of the tab column (Hints, Properties and Objects could not be closed),
  and the column with the docker in front is kept when TraceDraw closes,
  so a docker closed stays closed the next time. Help > Hints closes
  Hints when it is ticked.
- `.cdr` writing, made to match the sample files table by table: each
  page now lists the master page's guides, desktop and grid layers by the
  master layers' own GUIDs (they had GUIDs of their own, and the grid
  entry was missing), and the style table numbers its entries once and
  gives every style a fill, an outline, fonts for all three scripts,
  spacing and decoration settings that exist (styles named entry 0
  before). Text objects get numbers of their own and graphics a graphic
  style. These are the likeliest reasons written files closed the
  program that opened them.
- The browser version loads behind the desktop app's splash screen on
  the grey of the drawing window, with the download's progress in
  megabytes and percent on its bar, and under it what TraceDraw is (an
  open-source vector editor that opens `.cdr` files, alpha, written in
  Rust, MIT or Apache-2.0), which screens it suits and links to the
  desktop downloads, the code and the issues. Once up it simply runs:
  the demo message at the start, the menu bar's demo link and the
  Welcome Screen's demo note are gone.
- The `.cdr` issue form asks for the file's version and what the drawing
  uses (B-splines, lenses, live effects, pattern fills, paragraph
  settings, fitted text, pictures, pages).
- With two or more objects selected (or a group, or a curve of several
  parts) the Pick tool's property bar shows Group, Ungroup, Ungroup All,
  Combine, Break Apart, the shaping operations (Weld, Trim, Intersect,
  Simplify, Front Minus Back, Back Minus Front, Boundary) and Align and
  Distribute.
- `.ai` files written as PostScript (the format's early versions) open
  through the PostScript interpreter instead of failing as PDF.
- PostScript: `.prn` print files open and import (a printer job header
  before the program is skipped), multi-page `.ps` and `.prn` files open
  with one page per page of the file at its own size, and File > Export
  writes PostScript (`.ps`) and PRN (`.prn`) with every page.
- `.cdr` files: fonts embedded in the file are used for its text when
  they are not installed (and their licence allows it), so text keeps its
  letters instead of falling back to another font; the drawing carries
  them, saved in `.tdraw` files, and the Fonts docker marks them
  "embedded". Coloured runs of text in files from recent versions take
  their colour (stored inline in those files).
- The missing-font check also looks at text inside groups, clip frames,
  table cells and symbols.
- A missing font is drawn with a stand-in of the same widths when one is
  installed (Liberation Sans for Arial and Helvetica, Liberation Serif
  for Times New Roman, Liberation Mono for Courier New, Carlito for
  Calibri, Caladea for Cambria, Gelasio for Georgia, the TeX Gyre faces
  for the classic PostScript ones), else of the same style (serif,
  monospaced, casual or sans-serif), so text keeps its line breaks; the
  messages name the stand-in, as in "Arial (Liberation Sans)".
- `.cdr` files, checked against the previews files carry (public samples
  of versions 11 to 22 and files of the project's author): line breaks in
  artistic text of recent versions (they showed as boxes and the text ran
  on one line), centred lines, bold, semibold, italic and underlined runs
  from the style strings, rectangles of version 15 and later at their own
  size (the scale was applied twice) with corners in the right unit,
  polygons and stars at their size (they came out as a point), the size
  of each page, layer names, uniform transparency, and CMYK colours shown
  through the profile the file carries (the colours of the previews).
  Every face of an embedded family is used (a bold face next to the
  regular one), fonts are found by legacy names such as "Arial Narrow",
  and the palette keeps colours in the CMY, HSB, HLS, Lab and YIQ models.
- `.cdr` files: text fitted to a path follows its path when the two are
  not grouped (it was drawn straight, or off the page), at the offset and
  distance the file sets, mirrored text reading upright on the other side
  of the path. Objects of lists nested in a layer keep their stacking
  order.
- Text on a path: the letters stand on the path moved by the distance,
  spaced along it; on closed paths text runs on past the start, and
  mirrored text reads the right way round on the other side.
- Outlines thinner than a pixel are drawn one pixel wide on screen and in
  bitmap exports instead of fading out.
- `tracedraw-cli`: `TRACEDRAW_PAGE` chooses the page a bitmap or single-page
  export writes, and `TRACEDRAW_OBJECTS=1 tracedraw-cli info` lists every
  object the reader made.
- Letters a font lacks are drawn with a font that has them, so Chinese,
  Japanese, Korean, Arabic, Hebrew or Thai text set in a Latin font shows
  its letters instead of empty boxes.
- Variable fonts draw bold and italic text with their weight and slant
  axes instead of the default instance.
- The browser version offers about a hundred free font families (the
  stand-ins of the same widths for Calibri, Cambria, Georgia, Palatino,
  Century Gothic and others, popular families such as Open Sans, Roboto,
  Montserrat, Lato and Poppins, and fonts for Chinese, Japanese, Korean,
  Arabic, Hebrew and Thai). The font lists show them; a family's files
  are fetched the first time a drawing's text needs it.
- `.cdr` files before version 12: text, font names and layer and object
  names written in Cyrillic, Greek, Turkish, Central European, Baltic,
  Hebrew, Arabic, Thai, Vietnamese, Japanese, Chinese or Korean read in
  their own code page (they came out as Western European letters).
- File > Save As opens a Save Drawing dialog of its own: folder bar,
  shortcuts (home, desktop, documents, downloads, pictures, drives), the
  folder's contents with dates, types and sizes, file name, type, the
  `.cdr` version and the document's title, subject and rating, asking
  before replacing a file; in the browser it keeps the name, type,
  version and properties of the download.
- Save As `.cdr` writes any version from 7 to 22, chosen in that dialog
  (programs open files of their own version and older ones). Versions 7
  to 13 are RIFF files, 14 and 15 ZIP archives around one, 16 on ZIP
  archives with the chunk tree apart from its data, each with the
  records, tables, master page and layers of that version. A drawing
  opened from a `.cdr` file saves in its version unless another is
  chosen; Options > Global > File Formats sets the version of new
  drawings (12 at first). `tracedraw-cli` reads the
  version from `TRACEDRAW_CDR_VERSION` and the MCP `save` tool takes a
  `version`.
- `.cdr` writing: files of versions 21 and 22 carry the configuration
  record of those versions (page orientation, ruler origin, grid,
  units, nudge, duplicate offset, drawing scales) instead of an empty
  one, and every file has a font table for its styles.
- `.cdr` files: version 13 and later outlines read their nib angle and
  width scale from the right place, guide and grid layers stay hidden,
  and empty master layers no longer show up as layers of their own.
- File > Close All closes every drawing (it closed only the active one).
- AI Drawing (Tools > AI Drawing): describe what to draw or change and
  an AI service you set up with your own key (the Messages API, or any
  chat completions service, local model servers included) writes a
  script for the object model; it runs as one undo step, without file
  access and all or nothing, and a failing script is sent back once with
  its error. The key stays in memory unless you ask to remember it.
  Works on the desktop and in the browser. While a service is set up, a
  prompt bar above the colour palette takes requests (Enter draws) with
  a model list fetched from the service; Window > Toolbars hides it.
- Scripts: `CreatePath(d)` draws curves from SVG path data.
- AI agents can edit the drawings of the open window: with Options >
  Global > General > "Let AI agents on this computer edit the open
  drawing" on, an MCP client that starts `tracedraw --mcp-window` gets
  the tools of `tracedraw --mcp` on the window's drawings, with undo and
  shown at once. The window listens on this computer only, with a token.

- Ctrl+C, Ctrl+X and Ctrl+V copy, cut and paste objects in the desktop
  app (the windowing layer delivers them as clipboard events, not as
  key presses, so they did nothing); Ctrl+Shift+V pastes in the middle
  of the view.

**Known limits**
- Downloads are not code-signed yet: Windows SmartScreen asks for "More
  info > Run anyway", and macOS needs right-click > Open the first time.
- The `.cdr` reader is written from public notes and sample files; some
  files open with objects missing or looking different. Reports with a
  file are very welcome (there is an issue template for them).
- Installing an update in place works for the Linux AppImage only;
  elsewhere Help > Updates offers the download.
- Gaps are listed in [`docs/blueprint-gaps.md`](docs/blueprint-gaps.md).

## 0.4.0 (2026-10-11)

The Pick and Shape tools now behave as the keys and the pointer lead
you to expect: modifiers while dragging, choosing inside groups,
bending segments, a pointer that shows what a press will do.

**Selecting and transforming (Pick tool)**
- Shift scales from the centre, Ctrl scales in 100 % steps (dragging
  across gives the mirror image at the same size), Alt frees a corner,
  Ctrl keeps a move straight; a right click during a drag leaves the
  original, Space drops a copy, Esc cancels.
- In rotate mode the side handles skew, and the centre of rotation can
  be moved; it stays with the object (selecting it again finds it where
  it was put, and it moves with the object).
- Ctrl+click chooses an object inside a group (round handles; it moves,
  takes colours and is deleted on its own); Alt+click digs to the object
  beneath; a marquee with Alt takes what it touches.
- A click on any selected object, or on the centre mark, turns the
  handles; the centre mark can be dragged; scale handles snap to other
  objects, guidelines, the grid and the page; `+` duplicates in place.
- The Freehand Pick has the Pick tool's handles and takes what its lasso
  holds wholly (with Alt, what it touches).

**Curves and nodes**
- Shape tool: dragging a segment bends it (a straight one becomes a
  curve, smooth nodes stay smooth); a click on a segment marks where Add
  Node (or `+`) puts the new node; C, S and Y make the chosen nodes cusp,
  smooth or symmetrical; `-` deletes them; a right click chooses the
  node or segment under the pointer before the menu opens.
- A line drawn from (or to) an end node of the selected curve continues
  that curve, and closes it when it meets the other end.
- Ctrl keeps lines and Bezier nodes to steps of the constrain angle;
  with Freehand a click and then another click draw a straight line.
- The curve tools' property bar has the outline width, line style,
  arrowheads and Close Curve, and Freehand its smoothing (also in
  Options > Tools > Curve tools).

**The pointer**
- It shows what a press would do: resize arrows on the handles, move
  arrows on objects and the centre mark, turning and slant arrows in
  rotate mode, arrows across guidelines, the copy arrow when a move will
  leave the original.
- The Shape tool's own arrow marks nodes, control points and segments,
  and shows the resize, turning and slant arrows over the handles that
  frame selected nodes.
- Drawing tools show their icon by the cross hair, with a sign where a
  line would continue the selected curve or close the one being drawn;
  the Eraser and the brushes show their nib at its size; the Text tool
  shows the I-beam only over text; Interactive Fill shows the move
  arrows over the fill's handles.

**Text**
- Paragraph text frames show as dashed lines (Text > Show Text Frames;
  red with a down arrow when the text does not fit), the handles sit on
  the frame, and scaling it with the Pick tool resizes the frame so the
  text flows anew instead of stretching the letters. Lines that do not
  fit are no longer drawn over the others.

**Colour**
- Drag a colour from the palette onto an object: inside it fills it, on
  its outline it colours the outline; onto the document palette it joins
  it. Objects dropped on the document palette give it their colours.

**Interface**
- Interface size 100 % now draws menus, bars and text as large as other
  programs at 100 % (a quarter smaller than in 0.3.0 on a monitor the
  system scales by 125 %); drawings keep their real size at zoom 100 %.
- Touch workspace (Window > Workspace > Touch): a larger interface for
  touch screens; two fingers pan and pinch zooms.
- Shelf panel: keep objects for any drawing and after restarts, place
  them by double click or by dragging onto the page.
- Options > Global: Printing and File Formats set what the Print and
  Export dialogs start with (and can remember the last ones), where
  imported objects go, Bitmap Tracing sets Quick Trace's preset, and
  Warnings turns messages on and off.
- The status bar tells how far, how much and what angle while dragging.
- A double click on the drawing with the Rectangle tool no longer adds a
  page frame (double-click the tool's button for that).

**Files**
- Opening a `.cdr` file with objects that could not be read, or a
  drawing with fonts that are not installed, now says so in a message
  (with a Report button for the file).
- The browser demo's loading page says only that it is loading; the demo
  message shows each time the page opens.

**Known limits**
- Downloads are not code-signed yet: Windows SmartScreen asks for "More
  info > Run anyway", and macOS needs right-click > Open the first time.
- The `.cdr` reader is written from public notes and sample files; some
  files open with objects missing or looking different. Reports with a
  file are very welcome (there is an issue template for them).
- Gaps are listed in [`docs/blueprint-gaps.md`](docs/blueprint-gaps.md).

## 0.3.0 (2026-10-11)

The first release meant for everyone to try. It is an alpha: real
use will turn up bugs, and "Known limits" below lists what is known.

**Working in the window**
- Several drawings open in tabs; a Create a New Document dialog.
- The toolbox, palette, toolbars and status bar dock on any side or
  float, and the window remembers where they were.
- Keyboard shortcuts of your own for every menu command (Options >
  Customization > Commands), shown in the menus.
- Workspace files: export and import shortcuts, bars and toolbox,
  palette options and appearance, part by part; the bars shown come back
  after a restart.
- The interface size follows the scaling the system gives each monitor,
  like other programs (100 % by default, 80 % to 200 % relative to it).
- A splash screen, an About box, an interface size option, tool cursors,
  a Hints panel with a page per tool, typed number fields and a status bar
  with object details and coordinates.
- Right-button drag of objects with a drop menu; menus switch as the
  pointer moves along the menu bar.

**Drawing and shaping**
- Shape tools draw a live preview while dragging; rectangle corners
  (round, scalloped, chamfered) per corner; complex stars.
- Common Shapes: 71 shapes in five sets (basic, arrows, flowchart,
  banners, callouts).
- Shape tool node transforms, reflection and segment copy; Corners, Join
  Curves and Coordinates panels.
- Snapping modes with marks and screen tips; a drawing scale and ruler
  calibration.
- Linked object styles and default properties for graphics, artistic text
  and paragraph text.

**Text**
- Drop caps sit on their lines and hyphenation breaks where it should;
  character offsets and angles; the Shape tool on text.

**Bitmaps**
- Colour modes, bitmap effects kept as an editable list apart from the
  pixels, image adjustments with tone curves, Straighten Image, Manually
  Inflate Bitmap and Convert to Bitmap options.
- Vector Mosaic and Picture Mosaic.

**Print and PDF**
- Preflight checks before printing; imposition and printers' marks.
- Publish to PDF with bookmarks, document properties, links and
  passwords (AES-256); protected PDFs open after asking for their
  password.
- Time Tracker: time spent on a drawing, task by task.

**Platforms**
- The whole editor runs in the browser (WebAssembly), as a demo that
  says what it leaves out and links to the desktop app.
- Installers for Windows, macOS and Linux on every release, a Microsoft
  Store package in the build, and code signing ready to switch on.

**Known limits**
- Downloads are not code-signed yet: Windows SmartScreen asks for "More
  info > Run anyway", and macOS needs right-click > Open the first time.
- The `.cdr` reader is written from public notes and sample files; some
  files open with objects missing or looking different. Reports with a
  file are very welcome (there is an issue template for them).
- Gaps are listed in [`docs/blueprint-gaps.md`](docs/blueprint-gaps.md).

## 0.1.0 (2026-10-09)

First tagged version: the document model and command engine, the desktop
editor, the `.cdr` reader and writer, SVG, PDF and the other formats,
effects, text and the command-line tool.
