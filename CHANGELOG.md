# Changes

Versions follow the tags of this repository; every push to `main` also
publishes a nightly build. [`docs/features.md`](docs/features.md) has the
full, row-by-row status.

## Unreleased

## 0.19.7 (2026-10-27)

- Symbols: a symbol instance copied in one drawing and pasted in another brings its symbol along and shows the same objects there. It kept only the symbol's number in the list, so in a drawing without symbols it showed nothing, and in one with symbols of its own it showed one of those; symbols that show other symbols come along too, and pasting again reuses the symbol already brought (#678).
- Rotating with the Pick tool: in rotate mode a node of the selection (or a segment's midpoint) dragged turns it about the centre of rotation instead of moving it, and let go near another object's edge the node lands exactly on it, so an arc turned about one end can be laid onto a line; the pointer shows the turning arrows over such a node. Also, the turn follows the pointer when the rotate handle passes over the edge of another object or of the page. The pointer snapped to the edge, so the object stuck at one angle and then jumped while the handle crossed it; the handle still snaps to other objects' nodes, midpoints and centres to aim the turn at one (#933).
- Variable fonts: the Text docker's Character expander shows a slider for each axis of the selected text's variable font (weight, width, slant, optical size and the font's own axes). The values set go on the selected characters, are saved in .tdraw files and are drawn in the canvas, exports and PDF; Default axes takes them off (#870).
- 3-point curve, rectangle and ellipse: holding Ctrl while dragging the first line keeps it to steps of the constrain angle (15 degrees), as with the other line tools, so an arc drawn with Ctrl comes out level. Ctrl did nothing there, and the arc kept the slant of the hand. Ctrl held while placing the third point of the 3-point curve now draws a bow, the arc of the circle through the three points, which stands evenly on the line wherever the point is (#934).
- Envelope: the frame and the enveloped object stay smooth. The frame was drawn as 16 straight lines a segment and showed as a row of lines when zoomed in; a hard bend drew the object in visible facets; and Horizontal and Vertical followed a polygon through the frame instead of its curves. The frame is now drawn within a quarter pixel at any zoom, the object gets more points only where the envelope bends hard, and its sides lie on the frame (#881).
- Font lists: a Filter beside what to show picks families by the kind of letters (serif, sans serif, script, decorative) and by what they have (monospaced, symbol, variable, OpenType features), read from the fonts' own tables; the list opens as fast as before and, with thousands of fonts, fills in over a few frames the first time a box is ticked (#867).
- Opening `.cdr`: blends from versions 9 and X3 look as they did where they were made. The steps now turn from one end object into the other, curving or folding through the middle as the file's nodes pair up, and lie between the two objects instead of on top of the upper one. Before, every step kept the same upright shape on a straight line (#597).
- Properties panel: the outline's mitre limit, nib stretch and nib angle sit under an Advanced header after the common settings (caps, corners, arrows, dashes, position, behind fill, scale with object), closed at first unless one of them is set, so the outline section is shorter and its basic options are found at a glance (#487).
- Edit Fill: the gradient preview shows the start, end and skew nodes, and dragging them moves, turns and stretches the gradient as on the canvas (Ctrl turns in 15 degree steps); the preview has the object's proportions, the rotation and centre fields move nodes placed freely too, and OK now keeps nodes placed (or reset) in the dialog (#939).
- Dimensions: the bar's Options menu can join the extension lines and the dimension line (or an angle's arc) into one line that turns a corner instead of running past it, and a corner radius rounds those corners. Both are kept in the dimension and in saved files (#641).
- Drop shadow: making or editing a shadow follows the pointer smoothly, and a click on a shadow picks its object for editing. Each frame blurred and shaded a layer the size of the whole window for every shadow, however small; a shadow is now worked out on just the pixels it reaches, the picture unchanged, and in a 1168 x 733 window a redraw went from about 119 ms to under 10 ms. A click on the shadow with the Pick tool (or any tool that selects on a click) selects its object; with the Drop Shadow tool it shows the shadow's handles, and a drag on the shadow moves it (#935).
- Dialogs (Outline Pen, Fill, the effect dialogs and the others) can be dragged anywhere by their title bar, also partly out of the main window, so they no longer have to cover the drawing. A dialog was held wholly inside the window, so one nearly as tall or wide as the window, as on a laptop screen, could hardly move at all; now its title bar stays below the window's top edge and at least 100 px of it stays inside the other edges. Dialogs still open centred, or where they were left with Center Dialog Boxes off (#908).
- Pick tool: the pointer shows the move arrows only over what is selected. Over any other object it keeps the plain arrow (or the cross hair), as a press there picks that object first, so starting a marquee or picking objects that lie inside a frame or over other objects is no longer hidden under the move arrows. Dragging an unselected object still picks and moves it, and Alt+click still picks the object beneath (#885).
- Save As .cdr with Embed fonts: the embedded fonts are written as files that embed fonts have them. Each font in the file's font list now carries the number the text uses for it (the faces of a family share it), the embedded fonts follow those numbers, and their headers have the names, character set and checksums such files hold, so other programs can match the text to its embedded font; before, the fonts were numbered 1, 2, 3 in turn, which only TraceDraw itself did not need (#352).
- Shape tool: a corner rounded with its corner widget can be rounded again. Selecting the curve later shows a widget on each rounded corner, and dragging it changes the radius from the one it has, further in or back to a sharp corner; the widget had disappeared once the corner was rounded. This works for corners rounded between straight sides, also in saved drawings and with the Corners panel (#872).
- Shape tool: the rings inside the corners of a curve being edited, dragged to round them, are now a preference, Options > Nodes and Handles > Show corner rounding rings on curve nodes, and are off at first, so they no longer get in the way of editing nodes. Turn it on to round the corners of curves by dragging again; rectangles round their corners as before (#884).
- Print Merge: a Number field has a Format: 1, 2, 3, the letters A, B, C or a, b, c (Z is followed by AA), or Custom, which puts a text of your own before and after the number, for tickets and serial numbers such as "Ticket T-0098". Chosen in Add Column and in Configure Columns (#930).
- Open Palette: XML palette files open, with their colours' names and CMYK, RGB or Lab values (spot colours are skipped, and a file with nothing readable says so), and the file filter lists the extensions it accepts. `.xml` files were not offered at all, and the filter showed only "Palettes" (#882).

## 0.19.6 (2026-10-26)

- Multipage View: zooming with every page in view stays smooth, and the other pages scale with the zoom instead of jiggling and stretching. Each page was drawn anew on every frame of the zoom, and the picture shown meanwhile was stretched over the page's visible part; now the last picture follows its page until the view rests, and the pages are then drawn a few per frame. On nine pages of 1500 curves each, a zoom frame went from about 300 ms to under 1 ms (#903, #902).
- Blend: the objects acceleration slider moves the steps toward the start or the end object, also when the blend follows no path. It changed only their sizes (with Apply to sizing) and turns, so moving it seemed to do nothing (#910).
- Print Preview: the options at the left are wider and their edge drags wider, and what still does not fit scrolls sideways, so longer languages (Spanish and others) are no longer cut off at the right (#862).
- Print: the Layout tab has the paper margins (left, right, top, bottom), as Print Preview does (#909).
- Text panel, Paragraph: the icons beside the indent and spacing fields are drawn, so they show on every system; on Windows they were empty boxes, the interface fonts there lacking those arrows (#863).
- Color Management Settings and Document Properties read in the dark themes: the page under their tabs is drawn in the theme's field colour instead of white, under the light labels (#904).
- Envelope on text: letters given their own colour (or outline) keep it when the text is wrapped in an envelope, and the same holds for perspective and distortion. The warped text was drawn as one curve in the object's colour; it is now drawn as one curve for each colour, warped together as the whole text is (#912).

## 0.19.5 (2026-10-26)

- Outlines: new outlines start with a mitre limit of 45 degrees instead of 5, so a thick outline on text or on shapes with narrow turns bevels those corners instead of growing long spikes. Outlines already in drawings keep their limit (#894).
- Artistic and paragraph text: letters given another colour keep the font's kerning with the letters beside them. A text was shaped one colour at a time, so changing the colour of the "V" in "VA" dropped the pair's kerning and moved the "A" away; letters of the same font, size and placement are now shaped together whatever their colours, and no ligature joins two letters of different colours (#911).
- Drop Shadow on a clip frame or a group: the shadow now shows, beneath the frame and everything placed inside it. The tool gave the frame a shadow but it was never drawn, so only an object inside the frame could have one. The shadow follows what the frame shows: its fill and the contents within it, not the parts of the contents outside the frame; groups cast the shadow of each of their objects, and Break Drop Shadow Apart makes the same picture (#873).
- Text in a semibold face (or a light, medium or black one) shows that face everywhere: picking SemiBold under a family in the font list sets "Titillium Web SemiBold" instead of the family with Bold on, so the property bar and the Character docker no longer show Bold for it, and the docker's style list names the family's own faces (Regular, SemiBold, Bold ...). Such a face is no longer reported as a missing font substituted when its family is installed (#892, #624).
- Save Drawing: New folder stays in the folder shown and starts renaming the new folder; any folder or file in the list can be renamed with F2 or a right click; typing letters in the list jumps to the entries starting with them, and Enter opens the folder picked (#913).
- Chinese font names: a .cdr file saved in version 17 or later that names a Chinese font (方正黑体_GBK, 方正大黑_GBK) shows the name as written instead of garbled letters, and finds the installed font instead of reporting it missing; drawings TraceDraw saves keep such names too. Installed fonts that give their Chinese, Japanese or Korean family name only in a GBK, Big5, Shift JIS or Wansung name record are found by that name and listed under it (#898, #860).
- Contour, blend and extrusion keep the CPU low while dragging and pointing: a frame no longer works the effect out again to draw, to measure and to pick the same object. On a line of letters with a contour, a hit test with the pointer resting went from about 22 ms to 0.1 ms, and a drag works the contour out once for each move (#897).
- Save As: Embed fonts starts off, for `.tdraw` and `.cdr` files alike. It started on, so every drawing saved carried its own copy of the fonts its text uses; tick the box for the files that are going to another machine, and the choice stays for the next Save As (#886).
- Area Fill: a drawing of many lines that nearly meet (a line stopping a few ten thousandths of a millimetre short of the next, as files carry from rounding) now fills the cell under the click, not the whole drawing. Ends of open lines within 0.01 mm of an outline, or of their own start, count as meeting it (#899).
- Start on Windows: a start whose graphics driver hangs while the window is set up no longer stays open with no window. After 20 seconds without one, TraceDraw starts again with the next renderer by itself (OpenGL, then the software adapter), so the first start opens instead of only the one after it, and a drawing opened meanwhile gets a window of its own instead of going to the one that never showed (#570).
- Font Manager: the fonts of a folder added to the libraries can be used in drawings straight away and after every restart, without installing them one by one; removing the folder takes them out again, except fonts installed on their own (#866).
- What is being drawn shows over a black or dark fill: a freehand stroke, a polyline, 2-point line, B-spline or 3-point curve in progress and the 3-point tools' base line are a black hairline on a white line, the Bezier and Pen segments placed so far lie on a white line under their outline colour, the connector line and the zoom, text frame and rubber band boxes are on a white line too, and the node squares of a curve in progress have a white edge (#672).
- Shaping with open curves: an open curve or line as the target of Weld, Trim or Intersect is now cut by the closed objects and stays open (Trim keeps the parts outside, Intersect the part inside, Weld adds the parts outside to the welded shapes). An open curve acting on a closed object no longer distorts it: it takes no part, and the status bar points to the Knife tool for cutting a shape along a line (#828).
- The main window starts as it was left: maximized when it was left maximized (a maximize the window system dropped while the window opened is asked for again, and the first frames no longer overwrite the kept state), and a kept size that comes back the same with a larger interface or system text size, where it shrank at every start. A kept size larger than the monitor starts maximized (#787).
- Print Preview shows the printer choice at the top of its options, with the gear button that opens the chosen printer's Properties, its status and the number of copies with Collate, so the printer can be changed without going back to the Print dialog (#792).
- Print Merge docker: On every page is offered for Text and QR code fields too, not only pictures. Insert Field then puts the field in the same place on every page of the open drawing, and Create Merged Document fills those pages, one record each, instead of making a new drawing (#793).
- Fonts embedded in an opened `.cdr` file (or carried by a `.tdraw` file) that are not installed say "Embedded" at the end of their row in the font lists and after the name in the font box, so it is clear the text is drawn with the copy that came with the drawing (#518).
- A single click on an object with a drawing or shaping tool (the rectangle, ellipse, polygon and other box tools, Spiral, Common Shapes, Table outside a table, Knife, the connectors, Blend, Block Shadow, the shape brushes and others whose click had no job) selects it, Shift adds or takes it away, Ctrl reaches into groups, and a click on empty space lets go; a drag still draws. The line tools and Text keep their clicks (#271, by zalmanleib).
- The shape brushes (Smooth, Smear, Twirl, Attract and Repel, Smudge, Roughen) change only the selected objects: with nothing selected a drag no longer takes the object under the pointer, and the status bar says to select one first (#271, by zalmanleib).
- While typing text, Ctrl+L, Ctrl+E, Ctrl+R, Ctrl+J, Ctrl+H and Ctrl+N align it left, centre, right, justified, force justified or not at all, and a new alignment keeps text typed at a point in place at the reference point of the X and Y fields instead of moving it (#271, by zalmanleib).

## 0.19.4 (2026-10-26)

- Print Preview: a page smaller than the paper repeated on it (step and repeat, several pages per sheet) shows every copy. Only the copies in one band of the preview were drawn and white covered the rest; any group or clip frame moved by its own transform was left out the same way in large renders (#848).
- PDF import: Hindi written by office suites (the size set with the font, each glyph placed with a plain text matrix) comes in as whole words. A vowel sign placed a fraction of a point off the line, or the next syllable starting just before a sign's end, began a new text in the middle of a word, so क्षेत्र, शुद्धि or प्रार्थना came in as two texts and broke apart when edited; all 59 words of the test corpus and whole sentences now come back as typed (#771).
- Toolbars move into rows: the standard toolbar, the property bar, the Text, Zoom and Transform toolbars and the time tracker, docked at the top or the bottom, can share a row. Drag a toolbar's grip onto another toolbar's row to put it beside that one, or onto the line between two rows to give it a row of its own; the Text, Zoom and Transform toolbars and the time tracker now have grips and float or dock like the others. The rows are kept with the preferences (#837, #855).

- Print Preview with Print separations: the inks unticked under it no longer print; every plate printed before (#846).
- Number fields keep the mouse wheel: turning it over a field on the property bar (the angle, the outline width, any spin box) or in a docker or dialog steps the value notch after notch. The bar or panel around the field slid on with the wheel after the first notch and carried the field away from under the pointer, so the wheel stopped working on it. Parts of a notch from a touchpad now add up to one step instead of a step each (#783).
- Pick: an object chosen inside a group with Ctrl+click is dragged on its own from anywhere on it. Only a press on its centre mark kept it; anywhere else the drag took the whole group (#849).
- Clip frames, Edit Contents: the faded objects outside the frame can no longer be picked (click, marquee, Select All, Tab) while the contents are edited. Undo stops at the start of the edit: undoing past it leaves the edit with the frame as it was, instead of undoing the drawing behind it while the edit still showed; redo enters the edit again, and undoing Finish goes back into it (#851).
- Pictures look right at every zoom: pictures laid side by side (a map in tiles) no longer show pale lines where they meet, and a picture zoomed far out is drawn from a smaller copy of itself, so it shows its colours evenly instead of scattered, broken-up pixels. Pages of pictures zoomed out also draw faster (200 tiles at 21 %: 9 ms instead of 19 ms), so the new picture replaces the one shown while zooming sooner (#853).
- Variable Outline: a wide node at a corner no longer leaves a round bulge and a notch on the inside of the corner where the width falls away; the inside follows the width smoothly. Round corners and quick changes of width are drawn in finer steps, so the band's edges look smooth even when it is very wide (#776).
- Mesh Fill: a double-click inside the mesh puts a new node exactly where it was, on a new row and column through that point, and selects it; the new line went halfway between the nearest nodes instead, away from the click (#804).
- Shape tool: a node being dragged snaps to the other nodes and segments of its own curve too, not only to other objects (#673).
- PDF/X: with no CMYK profile set in Colour Management (or `TRACEDRAW_ICC` for `tracedraw-cli convert`), the output intent now embeds the CMYK profile the drawing carries, the profile its CMYK values are in, and an empty output condition takes that profile's name. It used to say "CGATS TR 001" with no profile, the name of another press (#643).
- PDF and SVG export: an outline set to scale with its object is written as wide as the drawing window draws it, its width times the object's scale. It was written at its own width, so a 10 mm outline on an object scaled to a quarter came out 10 mm wide instead of 2.5 mm, in .cdr files and in drawings scaled here alike.
- Opening .cdr files: objects of type 0x26 that keep a list of points (seen in version 22 files as the frames of PowerClips) are read as the closed outline through those points. They were read as a two-point connector line, whose points then fell far off the page, so the pictures clipped to such a frame did not show at all.
- Paragraph text in Chinese and Japanese breaks lines between characters by the line-start and line-end rules: punctuation such as ，。、；：？！ and closing brackets and quotes no longer start a line, and opening brackets and quotes no longer end one; the character before goes to the next line with it (#847).

## 0.19.3 (2026-10-26)

- Blend panel: the Color Blend choices (Direct, Clockwise, Counterclockwise) show again; the Reset button's row took the rest of the panel and pushed them out of sight. An Apply button blends the two selected objects with the panel's settings (#815).
- Colour pickers have an eyedropper: in the picker of every colour swatch (Outline Pen, the Interactive Fill colours, the dockers) and in Edit Fill, press it and click the drawing to take the colour there; Esc stops it. It works while a dialog is open (#814, #817, #819).
- Shift+F3 opens Text > Change Case for the selected text, and the menu row shows it (#801).

## 0.19.2 (2026-10-26)

- Dimensions hold to the object they measure: a dimension whose ends were placed on nodes of an object (its corners, say) follows it when the object is moved, resized or reshaped, and its value changes with it. Moving the dimension by itself lets it go (#838).
- Opening a .cdr file with photos takes far less memory: the pictures are kept packed about half as small (22 MB instead of 40 MB for a 4000 x 3000 photo), and a picture placed many times is kept once instead of once per object (also in .cmx files). Saving a .cdr no longer takes several times a bitmap pattern fill's size in memory while the file is written.
- Outline Pen dialog: the selected objects now show each change (width, colour, style, corners, caps, arrowheads, calligraphy, variable widths, overprint) live while the dialog is open; OK keeps it as one undo step and Cancel or Esc puts the outline back (#741).
- Shape tool: two or more neighbouring nodes deleted together leave one curve between the nodes either side, fitted to the old outline, so it keeps its general shape. Line nodes deleted together left a straight line (#675).
- Ungroup keeps a group's perspective: each object takes a Perspective effect of its own, first in its stack (folded into a perspective it had), and stays drawn as it was in the group, its own contour or distortion included. The perspective disappeared when the group was ungrouped (#674).
- Contour: the offset is now the distance on the page also for a scaled, stretched or rotated object, so an 80 mm inside step lies 80 mm from the edges instead of 47 mm (#754).
- Area Fill: the new object now follows the outlines around the clicked area exactly, with their own curves and nodes only where they cross, instead of a jagged outline with many extra nodes traced from pixels; an overlap of two objects is filled exactly (#734).
- Hindi half letters and conjuncts with vowel signs: a syllable is never cut when text is shaped or broken into lines. A vowel sign or the second half of a conjunct in another colour or style, a syllable whose letters the font only partly has, and a word broken at the end of a line no longer show a halant under a half letter, a dotted circle or a sign on its own. PDF import reads Hindi as the typed words when the file places vowel signs apart (moved down or back), draws a vowel as two glyphs (ा + े for ो), draws a reph glyph at the end of a word, or gives the text as `ActualText`; Publish to PDF writes `ActualText` for such clusters, so its Hindi comes back exactly (all 59 words of the test corpus, 35 came back wrong before) (#599).
- Pen and Bezier: Ctrl+Z through the steps of a curve drawn with them goes back to drawing it. The tool comes back, and the line goes on from the last node left with a curved preview along the mirror of its previous handle, wherever the pointer is. This also works after closing: undoing the close reconnects to the last node. You can undo anchor by anchor and go on drawing at once (#631).
- Alt codes while editing text: the keypad digits typed under Alt are no longer typed too (with Num Lock on they also came as text, so Alt+0193 could give "0193" before the character), and a character the system composes as well is typed once. Fonts made for the symbol encoding (legacy script and dingbat fonts) now list their glyphs in the Glyphs panel with the keystroke that types them, and text typed in them draws their glyphs instead of another font's (#469).
- Colour picker and Edit Fill: the Model list offers the registration colour (it prints on every separation) after the models, as the reference program's list does; it could not be chosen anywhere (#286).
- Text: right-to-left words in left-to-right text (Arabic after an English word, or English inside Arabic) are now shaped and laid out right to left, joined, in the order the Unicode bidirectional rules give, instead of being laid out left to right like the English part. Digits inside right-to-left text read left to right, and paragraph text breaks such lines in reading order (#628).
- Text tool: in right-to-left text (Arabic, Hebrew) Ctrl+A and other selections now shade every selected letter, not only the first one at the right end, and the caret sits at the right of the letter it stands before, as it is read (#629).

## 0.19.1 (2026-10-25)

- Contour (outward, inward, to center) and the other offsets give clean curves: a circle's contour is a circle of four curves instead of hundreds of short straight pieces, sharp and bevelled corners stay sharp, and contours are drawn a little faster (#821).
- Convert to Curves and Break Apart of an enveloped, perspective or distorted object (text too) give smooth curves with their corners kept: a square in perspective is its four corners, a letter a few curves, where each was a row of short lines that Reduce Nodes then spoiled. Break Apart is in the canvas context menu for warped objects and artistic text. Letters an envelope pushes together stay filled where they overlap (#647, #840).
- Convert to Curves of a text keeps every letter filled: the strokes of a glyph that cross each other, as in many Chinese fonts, left hollow holes where they met once the text was curves; Break Apart of a warped text does the same (#800).
- Printing to a printer no longer freezes the window: the sheets are drawn and sent to the printer in the background, with "Printing on ..." in the status bar and the outcome there when it is done. On Windows a slow or stuck spooler left the window on "Processing" with a PowerShell window open until the program was closed (#810).
- Print Preview > Print to the system's PDF viewer opens each print in a page of its own; with the last print still open in the viewer, the new one did not open (#809).
- Edit Fill, the colour picker and the Color docker: a click near the black corner of the colour square in CMYK keeps its ring where it was put. All those points give K 100, which looks lighter on screen, so the ring jumped back up as if the click had not taken (#761).
- Layout > Insert Page Number on a master layer: one page number field goes on the master layer and shows the number of each page it is drawn on, on screen, in print and in exports, so one number manages them all. Before, nothing went on the master page; the numbers went on the pages instead (#807).
- Brush Strokes (Preset, Brush, Calligraphic, Expression): a stroke stays the path drawn, open and made of a few curves, with the brush as its live outline, so its nodes can be edited afterwards and the brush follows; it was a closed shape of many straight pieces. Apply to curve keeps the curve. The Sprayer still makes objects. Pages of such outlines draw as quickly as before (#829).
- Trim, Intersect and Weld with a picture as the target crop the picture to the result: its frame takes the shape and the picture shows inside it, also for a picture already cropped. The result was an empty curve that showed only in Wireframe (#827).
- Options > Customization > Commands: a tool or command can have several shortcuts. A new key joins the ones it has (a built-in key keeps working beside it) instead of replacing them, and each key has its own Delete (#822).
- Options > Customization > Commands has a Toolbar buttons list: every button of the Standard and Zoom toolbars (Zoom to Page, Fullscreen, Snap To and the others, and the separator). A button dragged off a bar can be dragged back from there; before, only Reset brought it back (#836).
- A right click anywhere on the Standard or Property bar, on its buttons and boxes too, opens the bar's menu, as on the toolbox (#835).
- Objects panel: every object's row has its own eye and padlock, to hide or show and lock or unlock it from the list, a locked object too (#832).

## 0.19.0 (2026-10-25)

- Opening `.cdr`: blends from versions 9 and X3 (and others storing them the same way) show their steps as a live blend between the two end objects; only the two ends were drawn before (#597).
- Print Merge: Add Column for a Number asks for its digits, starting value, "Continually increment value" with its step and an ending value; with an ending value the records from the start to the end are made at once, for serial, ticket or roll numbers without a data file (#791).
- Windows: printing to a printer from the Print dialog works. A print of one sheet, the usual case, failed before anything reached the printer, local or on the network, because the list of sheet sizes flattened in the PowerShell script; nothing came out and the spooler showed no job. The script also shows no "Printing page" box, gives up after five minutes instead of hanging, and writes what came of each print to `tracedraw.log` (#614).
- The mouse wheel zooms smoothly: each notch zooms in a glide over a few frames, as a scroll moves, instead of one jump of 15 %; a notch still zooms 15 % about the pointer (#766).

## 0.18.0 (2026-10-25)

- Large drawings with pictures use far less memory while editing: every undo step, background render and copied object now shares the picture data instead of copying it, so moving one curve in a drawing with 244 MB of pictures no longer adds 244 MB per step. Edit Fill and the font list preview no longer copy every step that can be redone. Saved files do not change.
- A Variable Length outline converted to an object (Ctrl+Shift+Q) is now a clean shape: a few real curves along each side and straight butt ends, instead of a whole row of anchors joined by straight lines. Along a curve, the band no longer has hairline cracks between its pieces, on screen or in exports (#627).
- The Glyphs panel's recent glyphs are readable again in the dark themes: their box takes the theme's field colour instead of staying white behind light glyphs (#660).
- Print Merge: with On every page, Create Merged Document leaves a picture frame whose cell is empty or whose picture does not load as it is instead of emptying it, says when nothing was left to fill (or when the pages hold no fields) instead of claiming it created pages, and Place Inside Frame on every page no longer stacks a second picture in a frame that already holds one (#618).
- The default palette of an RGB drawing is now a palette of RGB values defined directly (10% greys and multiples of 51, such as Navy Blue 0 51 153 and Purple 153 0 204) instead of values converted from CMYK at run time, and other CMYK colours with black in them convert as 255 (1 - min(1, ink + K)) per channel (#620).
- The OpenGL renderer now opens on graphics cards with OpenGL 2.1 only, such as the Intel HD Graphics of 2010 PCs: after OpenGL 3.3 and OpenGL ES it asks for a compatibility context, so such a PC draws on its card instead of the CPU's software adapter. The font list on the property bar no longer copies every font name on each frame while it is closed, which cost about 0.1 to 0.25 ms a frame with 1300 font families (#616).
- Tracing: neighbouring colour regions now overlap by 2 px under the lighter one, so no white hairlines of the page show between them; thin anti-aliased bands between two colours go to those colours instead of becoming slivers (#611).
- Save As starts at a drawing's own `.cdr` version when it was opened from a `.cdr` file, and at the version of Options > Global > File Formats otherwise; the hint there now says what the version and the Save new drawings as .cdr box do, and Save As no longer shows a meaningless "Version: v0" for TraceDraw files (#613).
- The wheel over the outline width box steps once per notch: a touchpad's small movements add up instead of running through the whole list, and a browser's notch (100 points or 3 lines) is one step, not two or three (#663).
- Editing a symmetry ends when another drawing comes to the front or a new one opens; the mode and its "Editing the symmetry" message in the status bar stayed on over the next drawing (#442).
- The Convert to Curves page says that a whole ellipse becomes four symmetrical nodes, as its checks and the test already did, and names the nodes in the drawing's Y-up order: right, top, left and bottom (#625).
- Print: the printer list is read off the interface's thread, so a slow print server no longer holds the window when the dialog opens; the printers are in name order with the default chosen where it stands, and the General tab shows the chosen printer's Status, Location and Comment, with a Properties button that opens the printer's own settings (#614).
- A combined curve with tens of thousands of nodes (a combined ornament) can be moved again: with snapping on, every frame over it compared each node with every other to find the snap points near the pointer, about 1.8 s a frame for 40,000 nodes, so a drag seemed to do nothing. Now a frame over it takes about 9 ms and a drag frame about 5 ms (#764).
- When the window is drawn on the CPU because Use hardware acceleration is off in Options, the status bar now says so, and a click on the note opens Options > General to turn it on; before, it only suggested installing the graphics card's driver (#616).
- The arrow keys nudge the selected guidelines when no object is selected: by the nudge distance, Shift by the super nudge and Ctrl by the micro nudge, one undo step per press; locked guidelines stay (#781).
- Windows: when the graphics driver crashes or hangs TraceDraw while the window is being opened, with nothing shown and the log ending at "Using wgpu", the next start now opens with OpenGL by itself (and the software adapter after that), and keeps using it; `--renderer=gl` no longer has to be typed (#570).
- A gradient fill and a gradient transparency now turn, mirror and skew with their object (the Pick tool's handles, the angle and mirror buttons, the Transformations docker, a turned group); before, the gradient kept running the same way on the page (#775).
- Print with Color: Separations now prints a plate for each spot ink, after the cyan, magenta, yellow and black ones, and the print preview offers its box again: the printed sheets had lost the drawing's list of spot inks, so only the four process plates came out (#779).
- The page tabs under the drawing slide to show the current page's tab whenever the page changes (a tab, the page navigator, PgUp and PgDn, Go To Page), and the mouse wheel over them slides them. In a drawing of many pages, a merged document for example, the tabs past the end of the row never showed before, so those pages could only be reached with Go To Page (#794).
- Drawings with much large text are smooth again: the box of a text object, asked for many times a frame by hovering, snapping and the Pen preview, copied the text's whole outline each time. It now comes from a box kept with the cached outline. On the reported drawing (45 lines of 257 pt text) a Pen preview frame went from 7.3 to 1.6 ms (#802).

- Browser version: the module is optimised for size with binaryen's wasm-opt and loses its name, debug and producer sections, 36.9 MB down to 31.6 MB (13.3 to 12.7 MB compressed). Nothing changes in what it does, and frame and script times stay the same.

- The default palette's menu has Rows > 1 Row, 2 Rows, 3 Rows, and with more than one row the swatches fill line by line (the second line goes on where the first ends) and scroll one swatch at a time (#610).
- .cdr files: text set in a semibold face (or a light, medium, extra bold or black one) now opens in that face instead of the bold or regular one, so it keeps its width on the page and in exported PDFs. The family is shown with its weight, "Titillium Web SemiBold", and draws with the installed face of that weight (#624).
- Dimensions: the label can be edited in place, with a double click on it (Pick tool) or a click with the Text tool. Text typed around the measured value becomes the prefix and suffix and the number keeps following the size; text without the value stays as typed, and deleting the label's text brings the measured value back; the label's font, size and colour are kept. Before, whatever was typed was lost at the next change. Break Apart now frees the lines, arrowheads and label as separate objects instead of doing nothing, and a new Options menu on the bar sets the extension lines' gap from the object and how far they run past the dimension line and a horizontal label; it stays open while its fields are typed in (#644).
- Blend: a new blend takes 20 steps instead of 5, so a front object blended to a copy just behind it gives a smooth side for 3D lettering instead of visible bands. A steps value already kept in the preferences stays as it is (#232).
- The Autofit Page popup no longer closes when its Live preview box or its margin field is clicked, so the preview can be switched on and off and watched; the box now sits on the margin's row (#659).
- Copy and paste no longer stall with a large picture on the system clipboard: Ctrl+C notes the text it wrote instead of reading the clipboard back, Windows tells by its clipboard change count whether anything was copied elsewhere, and elsewhere the picture is decoded only when there is no text or file to go by, or when it is what gets pasted, and sampled instead of hashed whole. With a 4000 x 3000 picture, pasting the copy here went from 177 ms to 118 ms on Linux and needs no read at all on Windows (#183).
- Properties docker: with nothing selected, the default outline and fill now sit under the same Outline and Fill header bars as with an object selected, so the two rows of paint kinds are named and the layout no longer changes on selecting; Add Outline and Add Fill show only once the object has an outline or a fill (the Outline box and the fill kinds give the first one); the Rectangle section's corner names wrap instead of being cut to "Top le..." (#487).
- A Variable Length outline, or any wide outline, inside a group or a clip frame is no longer cut flat at the edges of the strips the page is drawn in: a group is left out of a strip only when every object in it is (#626).
- A test now checks that Convert Outline to Object of a curved line with a thick butt-ended outline gives four cusp nodes and one curve per side (#622).
- .cdr files: rounded rectangles now open with Relative Corner Scaling off, like rectangles drawn in TraceDraw, so resizing one no longer stretches its corners and there is no need to switch the setting off rectangle by rectangle. A rectangle the file stores stretched shows round corners instead of oval ones (#619).
- Mask Outside Page is opaque, as in the reference program: nothing outside the page shows through, the mask is a near-black when the desktop is white, the selected objects' outlines are drawn over it in blue so their parts off the page can still be found, and the item sits in the View menu right after Pixels (#615).

- Convert Outline to Object (Ctrl+Shift+Q) on an outline with round ends or round corners: each round end is now two real curves meeting at its tip, with nodes where it meets the sides, and each round corner a curve, instead of many tiny straight segments. Curved sides also get fewer nodes (#623).

- File > Export now starts with the choices of the last export of the session (format, resolution, quality, transparency, pages and PDF settings; Selected only starts off again) instead of resetting them each time. To keep them for the next session too, turn on Options > Global > File Formats > Remember the settings of the last export, which new preferences now have on and which now keeps WebP lossless as well (#617).

- Print Preview: the box for each ink now stands under Print separations in the left panel, as asked, instead of in the top bar, where a drawing with many spot inks pushed Zoom and Close out of the window (#571).

- The Welcome Screen tab can be closed while a drawing is open: it shrinks to its house, which shows the Welcome Screen again, and stays closed after a restart until the Welcome Screen is shown (#677).
- Export: the Preset list now shows only the presets saved for the chosen format, and changing the format leaves a preset of another format unchosen. Each format has its own preset names, so saving a preset under a name another format's preset uses no longer replaces that one (#612).
- Shape tool: a click on a control handle keeps its node chosen, so the handle can be dragged next and the node stays where it is. The click used to choose the segment the handle lay on, and the drag that followed bent that segment, pulling the node's side along. The two handles a chosen segment shows can be dragged too (#639).
- Wrap Paragraph Text: the wrap style list beside the property bar button is now laid out as in the reference program: a Wrap style title, None (which stops the selected objects pushing paragraph text), the Contour and Square styles under their own headings, and the text wrap offset, greyed out while nothing wraps. Text already flows around shapes and pictures marked to wrap it (#661).

- `CONTRIBUTORS.md` lists the people who contributed code, with what each contributed, also those whose commits were recorded under another name; the README links it.
- The page size list on the property bar ends with Custom..., which opens the Page Size dialog with the page's width and height to type any size; it is the entry ticked when the page matches no preset (#650).

- The log now names the processor, its threads and the Windows build, says at the first frame how long the start took and how large the window is in pixels, and with `RUST_LOG=debug` sums up the frames every 10 s (count, input, build times, gaps and why they were asked for), so a report of a slow window shows where the time goes (#616).

- The preferences are no longer serialized on every frame to see whether they changed: only on frames with a key, text, a click or a dropped file, and at most once a second otherwise. A pointer move now costs about 0.1 µs there instead of about 20 µs, more on an older PC (#616).

- Pictures are decoded once and kept for every thread that draws the page. The canvas draws in bands on one thread per core, and each band decoded every picture it crossed again on every redraw: six large photos took about 1.1 s and 540 MB extra per redraw on four cores (more with more cores). Now a redraw after the first takes about 20 ms, thumbnails and the navigator reuse the same pictures, and the decoded pictures kept are capped at 512 MB.

- Tracing (Quick Trace and Outline Trace): outlines come out as smooth Bezier curves fitted to the shapes instead of stair-stepped polylines, with far fewer nodes; corners stay sharp (Corner smoothness rounds more of them), and stray specks and the fringes of soft edges no longer become curves of their own (fewer the lower the detail). Smoothing 0 still follows the pixel edges (#611).

- Saving a `.tdraw` and auto-backup now write the file as it is made instead of building the whole file in memory first: saving a drawing of 183 MB peaked at 2.4 GB and left about 2 GB held afterwards; now it stays near the drawing's own size. Auto-backup also runs on a thread of its own, so the window no longer stops every 20 minutes while a large drawing is backed up, and a backup cut short never replaces a whole one.

- Variable Length tool: a stroke made much wider than its own outline width is now drawn whole on screen. Parts of it reaching far from the path could be left out of the picture, so the stroke looked cut off with a flat edge on one side (#626).

- Canvas overlays show on any colour: a shape being drawn has a black hairline on a white line over its outline, so it no longer disappears into a black outline; the selection handles are black with a white edge; and the Shape tool's and curve tools' control handles are dashed on a white line, with white-edged arrow heads (#670).

- Shape tool: corner widgets. A curve being edited shows a small circle inside each corner; dragging one into its corner rounds the corners on the canvas as you drag, every corner of the curve together, or only the chosen nodes' corners when nodes are chosen. Works on open curves too; the status bar shows the radius, and the drag is one undo step (#635).

- View > Page > Mask Outside Page: an optional mode that covers everything outside the page with the desktop colour, with the page border drawn over the objects, so the page shows as it will print while you keep drawing and editing. Objects outside the page still show faintly and stay editable; the setting is kept with the other view settings (#615).

- `tracedraw-cli convert`: what the .cdr reader skipped or guessed (a fill it cannot read yet, a page size it does not take, objects left out) now goes to stderr as warnings, as for the other formats it reads. Only `info` showed it, so a converted file could miss part of the drawing without a word.

- Opening .cdr files: pages longer than 10 m on a side, such as a 22 m banner, keep their size up to 100 m, the bound the CMX reader already had. They opened as A4, with the warning that the page size looked wrong.

- Publish to PDF: a picture wholly outside the page and its bleed stays out of the file. It could not show, but its pixels were written: a photo left beside the page could make up most of the file's size (#642).

- When a software adapter draws the window (no usable graphics card driver), TraceDraw keeps to 30 frames a second, draws no shadows and uses the system's pointers instead of the drawn cross hair and I-beam, and says so once in the status bar. On an older PC every pointer move redrew the whole window on the CPU, which took every core (#616).

- With hardware acceleration on, TraceDraw no longer draws the window on the CPU's software adapter when no graphics card is usable: it tries OpenGL next and the software adapter only last, keeps going when the OpenGL set-up panics, says in a message box when no renderer can open the window, and logs every graphics adapter with why it was taken or left. An older PC sat at full CPU with the window crawling, with hardware acceleration on or off, and `--renderer=gl` showed nothing (#616).

- Print on Windows: the Printer list now shows the printers Windows has, network printers included, the default one first, and prints straight to the one chosen with the copies asked for. Before, Windows offered only the PDF viewer and PDF or PostScript files (#614).

- Property bar: the mouse wheel over the outline width box now changes the selected objects' outline to the next width of the list up or down, one undo step a notch (#663).

- Convert Outline to Object (Ctrl+Shift+Q) now gives clean shapes: curves stay real curves with handles, straight parts stay straight, and the result has as few nodes as keep its shape, instead of hundreds of tiny straight segments. The outline of an ellipse also no longer has a notch where the ellipse starts (#622).

- Print Merge, On every page: Create Merged Document and Print Merged Document now fill the pages of the open drawing, one record per page, instead of making a new drawing. With no placeholder picture but a pictures folder chosen, Place Inside Frame gives each page of the drawing its own picture from the folder, in name order, in the same frame (#618).

- Pen and Bezier: a click on an end node of an open curve, selected or not, now goes on drawing that curve as if you had never stopped. The next segment leaves along the mirror of the end's handle, so the preview is curved and the join is smooth. The curve had to be selected first, and the new segment always started straight (#630).

- Font lists: the mouse wheel over the font name box (with the list closed) now steps to the next or previous font, changing the selected or edited text at each notch (#665).

- Shape tool on rectangles: corner nodes can be chosen several at a time, with Shift+click or Ctrl+click (which also takes a chosen corner out) or with a marquee or freehand marquee over them, and dragging a chosen corner then rounds just the chosen corners, so the two top corners or any three can be rounded together. Only all four corners or a single one could be rounded before (#671).

- Tools > Options is now one window with a tab for each set of options along the top (TraceDraw, Customization, Tools, Global, Workspaces, AI and Document Options while a drawing is open), so every option can be reached without closing the dialog and opening another menu item. Each tab comes back to the page it showed last, and OK or Cancel covers the changes made on every tab (#633).

- The default colour palette can show two or three rows of swatches (columns when docked on a side), set in Options > Customization > Color Palette > Rows, so more colours are in reach without scrolling; the arrows and the wheel then scroll a column at a time (#610).

- Convert to Curves (Ctrl+Q) now works on dimensions: the dimension becomes a group of plain curves (its lines, its filled arrowheads and its label as curves) that no longer measures, each editable with the Shape tool. It used to do nothing. An angular dimension's arc is now a few smooth curves instead of dozens of short lines (#640).

- Text editing on the canvas: the caret is now a black bar with a white edge instead of the interface's text colour, which in the dark themes was a light grey lost on the white page, and selected characters get a clearly darker shade. Table cells being edited show the same caret and shade (#632).

- Font lists now follow the dark themes: the font names are drawn in the theme's text colour instead of near black on the dark list, and so are the characters of the Glyphs docker. The preview under the list and the font sampler keep dark text on their white card (#660).

- The default palette of an RGB drawing now holds clean RGB values instead of the CMYK colours as the press would print them: black is 0 0 0, white 255 255 255, red 255 0 0, and the tints are the matching RGB tints. CMYK drawings keep the CMYK palette (#620).

- Autofit Page: the popup has a Live preview check box. On (as before), the page shows the size it would get while the margin is typed; off, the page stays as it is until Autofit Page is clicked. The choice is kept (#659).

- An ellipse converted to curves is now one closed curve of four smooth nodes with equal handles, at its right, top, left and bottom. It was an open curve with a fifth node on top of the first, and large ellipses got more nodes still, so it had to be closed by hand (#625).

- Save As now starts at the `.cdr` version chosen in Options > Global > File Formats instead of always offering version 27. A new option there, Save new drawings as .cdr files, makes Save As start at the CDR type for a drawing without a file (#613).

## 0.17.0 (2026-10-24)

- Convert to Curves (Ctrl+Q) and Break Apart (Ctrl+K) now make an envelope, perspective or distortion part of the curves. Convert to Curves did nothing on an enveloped curve, and Break Apart lost the warp. Enveloped text breaks into one curve per letter, each warped as the whole text was, holes kept; a group with an envelope stays a group of warped curves. A warp after a contour, blend or extrusion stays live, and bitmaps keep theirs (#647).

- Outline Pen: the Variable Outline width nodes can be dragged along their bar, staying between their neighbours, with the fields following (a drag there moved the whole dialog). A drag on empty bar adds a node and slides it, and a typed position keeps the nodes in order (#652).

- Edit Fill: the dialog keeps its size and place, and OK and Cancel stay put, when switching between no fill, uniform, gradient, pattern, texture and mesh (#654).

- The drawing tools' cross hair and the text I-beam are now drawn black with a white edge, so they show on a page of any colour; on Windows the system ones could appear white on a white page. A shape being drawn now follows the pointer from the first frame of the drag, and the window keeps one frame queued instead of two, so what is drawn under the pointer trails it less (#655).

- Opening a file from the file manager or the command line while TraceDraw is running now opens it in a new tab of the running window, which comes to the front, instead of starting the whole program again. `--new-instance` still opens a separate window. On Linux, selecting several files opens them all in one window (#646).

- Opening a file that is already open (Welcome Screen, File > Open, Open Recent, a dropped file, the command line) now shows its tab instead of opening it again in a new one, unsaved changes kept. A .cdr or other drawing counts as that file until it is saved under another name; templates still open as new drawings (#645).

- Symmetry: after Finish, a symmetry moves, scales, rotates and mirrors as one object, with its copies and mirror lines. The lines used to stay where they were, so the copies mirrored the move and a handle rearranged them, as if the symmetry were still being edited. The drag outline shows the copies too. Duplicate, Step and Repeat and copies made during a drag take the lines along, and a symmetry inside a group follows the group. While editing, moving the object still leaves the lines in place.

- Property bars: the up and down arrows (and the mouse wheel) of the rotation angle, scale %, arc angles, dimension text size and line width, corner radius and Transform toolbar fields now apply at every click. After clicking into the box, only the first arrow click worked.

- A right click while dragging (move, scale, rotate, skew, and mirroring with Ctrl and a side handle) now leaves the original and makes the copy whichever button you release first. Releasing the left button first, or both at once, used to count as a click: on empty canvas it emptied the selection and nothing was copied. The scale, rotate and skew pointers now also show the copy's plus (#528).

- Segment Delete: dragging a box deletes every segment the box touches, whole, up to the crossings at its ends (a closed shape nothing crosses goes whole), as other programs do; it used to delete only the segments lying wholly inside, so a box across an ornament deleted nothing. A click that slips a few pixels still deletes, and a drag on a crowded page is about ten times faster (#528).

## 0.16.0 (2026-10-24)

- Text > Join Lines into Paragraph: the selected lines of artistic text (a PDF brings its text one object per line) become one paragraph text with the same first baseline and line spacing, so text typed into it flows on to the next lines (#599).

- Text: Hindi (and the other Indian scripts) after a Latin word on the same line keeps its half letters and conjuncts; the line was shaped with the Latin rules, which showed the halant under half letters. Seen in text taken from PDFs (#599).

- Print Preview: Transform shows handles around the artwork to stretch it on either axis (or the x and y percentages typed) and a drag inside moves it; Print prints it so (#595). Print: a Smart Board 16:9 sheet, Custom sheet sizes that can be saved with a name, and Stretch to fill, which fills the sheet inside the margins on each axis (#591).

- Export: WebP can be lossy (Lossless off, the default): the quality then shrinks the file as other programs do, a tenth of the lossless size at low qualities. Lossless keeps the earlier behaviour. The browser version stays lossless (#592).

- Autofit Page: while its popup is open, the page itself grows or shrinks live to the size it would take around the drawing, following the margin as it is typed (#594, #604).

- Print Merge: "On every page" beside the placeholder picture puts it in the same shape and place on every page with Place Inside Frame, in the matching shape of each page or a copy of the frame (#598).

- Edit Bitmap opens pictures in the bitmap editor of your choice (Tools > Options > Global > File Locations: GIMP or any program); empty, it uses the system's program for PNG files as before (#593).

## 0.15.0 (2026-10-24)

- Windows: the program carries the C runtime it needs (no Visual C++ redistributable required). A start that fails now says so: the log is written to `tracedraw.log` in the settings folder (`%APPDATA%\TraceDraw`, or `tracedraw-settings` beside a portable copy), and a crash shows a message and writes `crash.log` there; nothing was shown before (#570).

- PDF import in Text mode: Hindi and other Devanagari text comes back in the order it is typed. The short i sign (ि) and the reph (र्) were taken in the order the PDF draws them, so they sat on the wrong letters (#585).

- Print Preview: paper margins for each side, typed in the left panel or set by dragging their dashed lines on the sheet; the pages go inside them, clear of what the printer cannot print (#580, #582).

- Print Merge: Place Inside Frame with nothing selected waits for a click on the shape, with the Place Inside arrow as the pointer, and takes the shape clicked (also inside a group). It was greyed until a shape was selected (#577).

- Print Preview with separations: a box for each ink, always in view, instead of a list. One ink shows its plate; several show what those inks print together, to check two or three colours at once (#571).

- Duplicate (Ctrl+D) repeats what was done to the copy: duplicate an object, move, scale or turn the copy while it stays selected, and each next Duplicate takes the same step from the last copy, as in the reference program. It always used the duplicate offset (#559).

- Print Preview: a drag on the sheet moves the artwork on the paper before printing (clear of the margins the printer cannot print), with Artwork position fields and Reset on the left; Print prints it where it was put. A drag moved only the page information (#536).

- Export: presets with names. "+" saves the format and its settings (resolution, quality, transparency, selection only, all pages, PDF choices) as a preset; the Preset list puts them back, each shown with its format; presets can be renamed and deleted, and stay in the preferences (#547).

- Publish to PDF: Objects > Bitmaps downsamples colour, gray and black-and-white pictures to a resolution of your choice, compresses colour and gray pictures as JPEG with a quality, and can leave text and line art uncompressed. Gray pictures take one channel. The Web preset now makes small files (150 dpi, JPEG 75). Pictures always went in at full resolution, lossless (#550).

- Dimensions: the property bar of the dimension tools sets the label's font and colour, the line colour and width, the arrowheads (arrow, open arrow, dot, tick, none) and rounded line ends, for new dimensions and the selected ones. Lines were always black hairlines and the label took the current font (#475).

- Text: Options > Text > Display > Text units sets the unit font sizes are shown and typed in (points at first; millimetres, centimetres, inches and the other units of the rulers), for large work. A unit typed with the value counts, as in the other fields (#529).

- Text: a font or size picked on the property bar or the Text docker with nothing selected asks whether it becomes the default for new text, as the reference program does. It changed the default without a word, so new text could start in another font than Arial after a pick made by mistake.

- Fit Text to Path: while it waits for the path, the pointer near a path shows the text where a click puts it, centred on the nearest point and as far from the path as the pointer, with the distance and a red mark where the text begins; the click places it so. Nothing showed until the click, which always put the text on the path itself.

- Toolbars: a new installation starts with Small buttons (Default Button Size), as the reference program does; it started with Medium. A size already saved in the preferences stays.

- Symmetry: Finish selects the whole symmetry. The object picked inside it while editing (the last one drawn, for one) stayed selected, so the next drag moved that object alone, as if it had been double-clicked into.

- Toolbars: the buttons of the Standard and Zoom toolbars can be changed. With Tools > Options > Customization open (or with Alt held), drag a button along its bar to move it, onto the other bar, or off the bars to remove it; drag a command or a tool from the Commands page onto a bar to add it. Reset on the Command Bars page brings the first buttons back. The Zoom toolbar shows pictures, as the Zoom tool's property bar does; it showed words only (#563).

- Downloads: Linux on 64-bit ARM (arm64) gets its own `.deb`, AppImage and `.tar.gz`, built on an ARM machine (#561).

- Browser version: with the interface in Chinese or Japanese, the menus and dialogs show their text; they showed empty boxes, the browser having no system fonts. The page fetches Noto Sans SC, TC or JP from its font folder when it starts. Variable interface fonts are drawn at Regular weight, not at their default, which is Thin in some (#561).

- Shaping: Weld, Trim and Intersect work with one object selected: choose the command (Object > Shaping, the Shaping panel or the property bar), then click the object it acts on, as in other drawing programs. The buttons were greyed until two objects were selected, and which one was cut depended on the selection order (#562).

- Pen: the preview follows the pointer as smoothly as in 0.13.0; its white line, drawn as a path shape, cost about a third of each frame. The white line is now as thin as the dashes, so the preview reads mostly blue with short white gaps (#560).

- Symmetry: after Finish the mirror lines no longer show on the selected symmetry, so it reads as one object; they stayed whenever it was selected and editing looked still open.

## 0.14.0 (2026-10-23)

- Variable Outline: a width node dragged along the path slides along it, and its reach handles change its width; a drag on a node could only change its width (#479).

- Variable Outline: the width eases along a smooth curve between nodes however close they are; short spans on long paths ran in a few straight pieces. Shift or Alt for one side counts while dragging, not only when the drag starts (#492).

- Connectors: a copy of a connected object (Duplicate, right-click drag copy, Repeat, Copy and Paste) comes with a copy of its connector, of the same kind and style, joining the object at the other end to the copy (#476).

- Convert Outline to Object (Ctrl+Shift+Q) keeps a dashed outline's dashes: each dash becomes a closed shape and the gaps stay empty; the result was a solid band (#490).

- Repeat (Ctrl+R) after a copy made with a right click during a drag is one undo step, as is that copy itself; the first Undo only moved the new copy back onto the original (#471).

- Export: WebP takes the quality setting; it was ignored. The files stay lossless, and below 100 their colours are first rounded to fewer levels (near-lossless), so a lower quality makes a smaller file (#544).

- Edit Fill > Uniform: the colour's model list and a field for each value (C, M, Y, K or R, G, B and so on), as in the colour picker; it showed only the swatch and the hex code (#468).

- Colour palette: in an RGB drawing the default palette shows and applies its colours as RGB (converted, names kept); it applied CMYK colours. CMYK drawings keep the CMYK palette (#483).

- Welcome Screen: the colour palette shows pale and takes no clicks, and the dockers that work on a drawing say to open or create one, instead of showing and changing the selection of the drawing behind it (#482).

- Toolbox: docked on a side it fits the window's height, its buttons and icons getting smaller (down to 22 px) instead of the tools running off the bottom and needing the scroll arrows (#494).

- Property bar: the page size, nudge, duplicate distance, autofit margin and origin fields have one set of up/down arrows; a second pair beside each field did the same job. The field's own arrows and wheel step as the extra pair did (1 unit, 0.1 for nudge and duplicate) (#488).

- Shaping: Back Minus Front, Front Minus Back and Trim on a group cut each of its objects, each keeping its own fill and outline; the result had no fill or outline and showed only in Wireframe. Weld and Intersect with a group take its first filled object's attributes (#539).

- PowerClip (Clip to Frame) Edit Contents: a group inside can be ungrouped and its members stay in the frame, drawn in full and back inside on Finish; objects drawn or pasted while editing go in too. The members faded as if outside and Finish emptied the frame (#537).

- Print: Fit to page shrinks objects that run off the page (or the selection printed) to fit inside it, centred; they were cut at the page's edge. Print Preview's Close button sits at the right end of its bar (#534).

- Options > Customization > Appearance: the desktop colour button shows the colour around the page, and white picked makes the desktop white; white stood for the theme's colour, so picking it changed nothing until another colour had been picked (#530).

- Smart Fill fills enclosed areas off the page, on the desktop, and in drawings of any size; it worked only inside the page and gave up on drawings much larger than it. An area is open when the flood reaches past the outlines, not when it covers almost the whole page (#499).

- Print: Keep page size keeps the pages at their size for Several Pages per Sheet and Step and Repeat too; the pages shrank once the columns and rows no longer fit the sheet. The grid stays as set, and a warning says when pages run off the sheet. Columns and rows go up to 99, not 20 (#496).

- Print Merge: the data grid selects a block of cells by dragging across them (or over the record numbers or field names for whole records or fields), with Add (as many empty records after it), Delete (its records) and Clear (its cells, also the Delete key) (#501).

- Layout > Delete Page opens a dialog: the page to delete (the current one to start with) and, with Through to page, the last page of a range, deleted in one undo step; it deleted the current page only (#500).

- Print Merge: CSV files saved by Excel on Windows import; they are in code page 1252, not UTF-8, and failed to read. UTF-16 files (Excel's Unicode Text) and tab-separated files import too, and a UTF-8 byte order mark no longer sticks to the first field's name (#495).

- Paragraph text: the bottom handle of a selected frame drags it taller; the flow tab sat under that handle and took the press, so the next click linked the text to another object (after Text > Convert, for one). The tab now hangs below the handle (#503).

- Fonts: on Windows the font lists offer only the fonts Windows lists as installed; a font uninstalled while in use left its file in the font folder until the next restart and was still offered. A font for new text that is no longer installed gives way to Arial at start, instead of being named in the bars while text was drawn with a stand-in (#504).

- Text: typing in a text keeps its own hyphenation; it took the setting for new text, so editing a text turned its hyphenation on or off behind the menu's back. Text > Use Hyphenation while editing acts on the paragraphs at the caret, and its check mark shows them (#470).

- Text: Alt codes type their character on Windows (Alt+0193 gives Á, or the glyph a symbol font draws there, as the Glyphs docker's keystroke says); nothing was typed (#469).

- Text: Ctrl+Space while editing text finishes the text and switches to the Pick tool, as in the reference program; it typed a space (#478).

- Shape tool on text: in right-to-left text (Arabic, Hebrew) each character's node sits under its own letter; they were bunched at one end in left-to-right order (#474).

- Pen: the dashed blue preview to the pointer is drawn over a solid white line, so it stays one line over dark objects and any colour, as in the reference program (#489).

- Pen and Bezier: the constrain key (Ctrl) keeps the closing handle to the constrain angle when dragging on the first node to close a curve, as on every other node (#473).

- Shape tool: deleting the first node of a closed curve keeps the curve's shape, as deleting any other node does; the two segments around it kept their old handles and the curve changed (#472).

- Shape tool: Ctrl or Shift while dragging a control handle keeps it to the constrain angle (15° by default), and a smooth node's other handle keeps following; Ctrl turned the node into a cusp instead and Shift did nothing (#510).

- Interactive Fill: the colour node's bar and the colour picker popup have a dotted grip that drags them off the object they cover; they stay where they were put while open.

- About TraceDraw: a green ALPHA badge beside the version, with a tooltip saying what the alpha stage means.

- Symmetry: a double click on a symmetry goes back into editing it, as in the reference program; it opened the group's focus mode instead, so after Finish the only way back was the menu.

- Guidelines: Copy, Cut and Paste work on selected guidelines (no objects selected), pasting them in the same place, and the Transformations docker moves, rotates, scales and copies them; the Help says where the Ctrl+D duplicate offset is set (#376).

- Hyphenation: a word with a comma, full stop or quotation mark next to it can break at a line's end (before, any punctuation kept the whole word on the next line, in every language), words with an apostrophe break too (Italian *dell'informazione*), and Text > Use Hyphenation now turns hyphenation on for the selected text instead of only for text made afterwards. Checked with the Italian, English, Portuguese and German dictionaries (#255).

- Text on a path: changing the distance from the path, or pressing a mirror button with a distance set, no longer slides the text round the path; text centred on the bottom of a circle (Placement: Bottom, or both mirrors) stays centred as it moves in or out, as in the reference program (#193).

- Print Merge docker: Place Inside Frame makes a shape of your own (a circle, an oval, any outline, also inside a grouped card design) the frame of a picture field, so each card's photo goes inside it, placed by Align and scale; before, the photo went into a square frame the docker made, and a shape inside a group could not be a frame at all (#460).

- Browser version: every macro command (Macro Manager, Run Macro, Macro Editor, Load Macro Project) opens a message that macros run in the desktop version, with a button to its downloads; before, the commands did nothing visible or only wrote a line in the status bar.

- Symmetry editing: a symmetry made of one object picks the object alone, its selection box no longer spanning the copies; a click on an object where a mirror line crosses it picks the object instead of showing the line's turning handles. The symmetry bar's X and Y fields keep their label, box and arrows on one line.

- Browser version: opening or creating a drawing froze the page in 0.13.0 (the time measured for the lag fix of #441 is not available in the browser and stopped the program); it opens and draws again.

- Status bar: a double click on the fill opens the Edit Fill dialog box and on the outline the Outline Pen dialog box, as in the reference program, instead of the Properties panel (#465).

## 0.13.0 (2026-10-23)

- Print Merge docker: a Pictures folder for the photos the data names, so a folder of 50 photos fills 50 ID cards, each with its own; a name without an extension finds the .jpg or .png (#460).

- Tracing: Repeat Last Trace and Open Last Trace Settings in the Bitmaps menu, and a bitmap's right-click menu now lists them with Centerline Trace and Outline Trace and all their presets, not only Quick Trace (#452).

- Glyphs docker: the font list is the property bar's, so a font can be found by typing its name, and the line above the glyphs shows the keystroke that types the glyph (Alt+0169 for ©), as in the reference program. While editing text, Alt and a number types the character on every system, not only on Windows (#447).

- Transparency tool: a gradient transparency shows its nodes on the object, as Interactive Fill does: start, end, skew, nodes added on the line and the midpoint slider between two nodes to set where the blend is half way. A drag sets the gradient from where it began to where it ended, not only its angle (#448).

- Options > Text > Display: the default font and size can be changed while a text is selected, and they no longer turn into the selected text's font and size; with nothing selected the Text tool's bar shows them (#449).

- Envelope, perspective and distortions keep curves smooth: the letters of an enveloped text were drawn in visible facets (#455).

- T-shirt mockups work as in the reference program, and a new Help page walks through one (Effects > Making a T-shirt mockup). Three fixes made it possible:
  - An envelope, perspective or distortion on a group now bends each object from the box of the whole group and keeps its colours; before, the group vanished.
  - Uniform transparency keeps its merge mode (Multiply and the others), where it used to keep Normal only.
  - Objects inside a clip frame blend with what lies beneath the frame through their merge mode, so a print clipped to a shirt keeps the cloth's shading.

- Colour picker: Lab, Grayscale and YIQ can be chosen again in the Model list; the click on them closed the popup instead (#440). The Interactive Fill bar, the Text docker, shadows, gradient stops, tables, the palette editor and the page background now use the same picker as the Properties docker, with its Model list, and dragging in the bar's colour is one undo step that adds one colour to the document palette, not one for every colour passed over (#442).

- Options > General: Use hardware acceleration, with the graphics card to draw with, as in the reference program (#445). The window is now drawn with Direct3D 12, Vulkan or Metal. Turned off, or when the card cannot open the window (a virtual machine, for example), TraceDraw uses the software renderer and then OpenGL by itself and remembers what worked, so it opens where it used to fail (#439).

- Print Merge works as in the reference program: Finish opens the Print Merge docker instead of merging at once. There you insert Text, Image and QR code fields from the data source's columns. An image field takes a placeholder picture and an Align and scale setting (anchor point, then Actual size, Fit proportionally, Fill proportionally or Stretch to fill). Update Field changes the selected placeholder, and Create Merged Document or Print Merged Document makes the merged pages in a new drawing (#437).

- Tools > Options > AI > Service: the Model field has a list to pick from, with the models the service offers (Refresh list asks it for them) or the usual ones of its kind, as the AI bar has.

- Edit Fill (F11): the selection shows the fill as you change it, as in the reference program, so a gradient can be judged on the object. OK keeps it as one undo step, and Cancel or Esc puts the old fill back (#436).

- Less lag on heavy drawings (#345): the page is drawn on all the processor cores at once, and a page that takes long to draw is drawn on a thread of its own while the last picture stays on screen, so the window keeps answering. Each undo step keeps one copy of the drawing instead of two, objects are numbered with a faster table, snapping finds crossings without solving every pair of curves, and the bars, the cursor and the hover marks measure a large selection once instead of every frame. On a page of 5000 curves, moving an object went from 126 ms to 18 ms a frame, and hovering over all of them selected from 82 ms to 20 ms.

- Toolbox: a right click on a tool button or on its empty part opens the bars' menu (Customize, button size, the bars), as on the other bars; it used to open the flyout or nothing.

- Symmetry editing as in the reference program: a click or a marquee picks one of the objects inside (not the whole symmetry, not a copy), and the bar and the lines stay while you work inside it. The mirror lines can be dragged to move them, and a click on a line shows handles that turn them. The bar has icons, spin fields, Show full preview (copies as red outlines when off) and Show symmetry lines.
- Number fields everywhere (bars, dockers, dialogs, options) have spin arrows and step with the mouse wheel; Ctrl takes a tenth of a step and Shift ten steps.

- Toolbars: the Default Button Size starts at Medium on a new installation; a size already chosen is kept.

- Shape tool on a B-spline: control points can be chosen by a click, Shift+click or a marquee, as nodes are; Delete removes only the chosen points (it deleted the whole curve), and dragging a chosen point moves all the chosen ones.

- Segment Delete: curves inside a group (an imported ornament, for example) are now cut by a click and by a drag, as in the reference program; nothing happened before. A drag also takes level and upright lines, which it used to skip (#427).

## 0.12.0 (2026-10-23)

- Break Apart on a contour, blend or extrusion: Ctrl+K and the right-click menu now offer Break Contour (Blend, Extrude) Group Apart, as in the reference program, instead of a greyed-out Break Apart. The object stays and the steps come out as a group beside it, also from the dockers' Break apart buttons (#424).

- Colour Eyedropper: after a colour is sampled the pointer turns into the paint bucket, as in the reference program, and pictures give their colour again when they are grouped, inside a clip frame or kept as JPEG (#403).

- Hyphenation in any language: put a hyphenation dictionary (a `hyph_*.dic` pattern file) where the spelling dictionaries go and choose it under Options > Text > Paragraph Text > Hyphenation patterns (#255).

- Spell checking in any language: put Hunspell dictionaries (the `.dic` and `.aff` files many programs use) in the folder Options > Text > Spelling opens, or point it at a folder that has them. Words are checked with their prefixes, suffixes and compounds, suggestions use the dictionary's own replacement table, and each dictionary can be switched off (#255).

- Options > Text: Display, Paragraph Text, Fonts, Spelling and Autocorrect tabs, as in the reference program. Autocorrect can now capitalize the names of days, turn typed web addresses into hyperlinks, use the quotation marks of another language or ones you choose, keep straight quotes after numbers, and switch the replacement table off; Text > Writing Tools > Autocorrect opens that tab. Fonts sets how many recently used fonts the font lists show (up to 20), and Spelling keeps the spell checker options and can add its corrections to Autocorrect (#369).

- Segment Delete: after one stretch of a curve was deleted, clicking the stretch past the next crossing could delete the whole curve, because the first cut ended a hair off the outline it crossed. Crossings now lie exactly on both outlines (#354).

- Glyphs panel: a search field finds characters by themselves or by Unicode code point (`U+20AC`, `0x20ac`, `20AC`), and a line above the grid shows the code point, character and range of the glyph under the pointer (#375).

- Print and Print Preview: Keep page size stops pages from shrinking on a fixed sheet to make room for the marks, and a drag on the sheet in Print Preview moves the page information; Keep page size is remembered for the next print (#388).

- Print and Print Preview: a Back side option for n-up and step-and-repeat layouts runs the pages right to left on the sheet, so the back of double-sided ID cards prints right behind the front, without mirroring the text (#387).

- PDF and AI files: opening or importing one asks how to read its text, as in the reference program. Text keeps it editable; Curves draws each letter from the fonts the file embeds, so the text looks exactly as in the file even when those fonts are not installed. Options > File Formats keeps the choice (#386).

- Print Merge: imports Excel workbooks (`.xlsx`) as well as CSV, guesses the picture, QR and text fields and shows their types after the import. Fields are now filled inside groups and frames, so grouped ID cards merge; a photo placeholder inside a frame fills the frame, and a shape named `<Field>` takes the picture inside it. Page copies with frames used to share ids with the template, so a merge could change the template instead of the copy (#377, #378).

- Edit Fill (F11): a dialog as in the reference program, with the fill types across the top and, for gradients, a preview, the fill picker, type and repeat, a band of colours whose nodes can be picked, dragged, added and removed, the node's colour, transparency, position and midpoint, Reverse, rotation, centre and edge pad, and Overprint fill. OK changes only what was changed. F11 opened the Properties panel before (#361).

- Outline Pen (F12): a dialog with every outline setting laid out in groups as in the reference program: General (colour, width with its unit, style, mitre limit, dashes, corners, line caps, position), Arrowheads, Calligraphy with a picture of the nib, Behind fill, Scale with object, Variable Outline (width nodes, sides and total) and Overprint outline. OK changes only what was changed; with nothing selected it sets the outline of new objects. F12 opened the Properties panel before (#357).

- Bitmap effects on vector objects (3D Rotate and the others) are drawn at the drawing's rendering resolution from Layout > Page Setup instead of a fixed 300 dpi, up to 16 million pixels instead of 4, and are made again when the resolution changes; set it higher for sharper results on screen, in PDF and in exported images (#382).

- Paragraph text: frames have a flow tab under their bottom edge, as in the reference program. A click on it, then a click on another frame links them, even on another page; a click or drag on an empty spot makes a new frame there that the text flows into. Linking used to need both frames selected, so frames on different pages could not be linked (#371).

- Place Inside Frame, Order > In Front Of and Behind: while they wait for the click on the target object, the pointer is a thick arrow pointing at it, as in the reference program; it stayed the ordinary pointer (#379).

- Variable Outline tool: Shift and Ctrl now work as in the reference program. A drag started with Shift changes only the side the pointer is on (Alt still does too); Shift+click picks a run of width nodes and Ctrl+click adds one or takes it out; a drag or the property bar then changes all the picked nodes, and Delete removes them (#374).

- Guidelines: several selected guidelines move together when one of them is dragged, and a right click during the drag leaves copies. Edit > Duplicate (Ctrl+D) copies the selected guidelines, and the Guidelines docker gets Duplicate and Clear (every guideline of the type shown) buttons (#376).

- Text: the font list and the size list show the selected text in the family or size under the pointer while they are open, as the reference program does; the text goes back as it was when the pointer leaves or the list closes, and the preview leaves the undo history alone (#370).

- Pick tool: scaling with a corner handle follows the pointer smoothly. The size followed the faster of the two sides, so a line of text grew at the pace of its short side and jumped with small moves; it now goes by where the pointer falls along the box's diagonal. The handles also no longer jump a few pixels when the drag starts (#383).

- Pen tool: the pointer is a pen nib with a dot at its tip, and the node goes exactly under the dot; the close, continue, add, delete and edit signs show beside it (#362).

- Pen and Bezier: a finished curve keeps its anchors in the history, as in the reference program. After it is closed or deselected, Ctrl+Z opens it first and then takes the anchors back one by one; it used to take the whole curve away at once (#360).

- Shape tool: with two or more nodes selected, the align keys (L, R, T, B, E) and the Object > Align commands align the nodes, as in the reference program; handles move with their nodes and it is one undo step (#364).

- Pick tool: a marquee (or the Freehand Pick lasso) dragged with Shift turns over the objects it takes, as in the reference program: those already selected are deselected and the others added. It only added before (#401).

- Interactive Fill: colour nodes on the gradient's line, as in the reference program. A double click on the line adds a node in the colour the fill has there, a double click on a node removes it, and nodes drag along the line. The bar between two nodes moves where their colours meet half way. A click on a node opens a small bar under it with its colour (with the colour picker) and its transparency; the property bar shows the node's colour, transparency and position, and a palette colour goes to the picked node. Node transparency shows on screen and in SVG (#330, #285).

- Interactive Fill: gradient nodes are placed freely, as in the reference program. Dragging on an object puts the start and end nodes where the drag began and ended; the start, the end and a new skew handle move on their own (the start moves the fill, the end sets length and angle, the skew handle makes a radial fill elliptic or slants a linear one). The fill keeps its nodes when the object moves or scales, and SVG and PDF exports draw it the same way (#330, #285).

- Select All on a drawing with thousands of objects no longer slows the window down: the selection is gathered once per change instead of searched for object by object several times a frame (a frame with 3000 objects selected went from about 200 ms to about 7 ms, #367).

- Text > Edit Text (Ctrl+Shift+T) opens the Edit Text dialog with the whole text in a box; OK applies it as one step and the characters left as they were keep their formatting. It only started editing in place, with no window (#355).

- Font Manager: the samples were drawn in black on the dark themes, nearly unreadable; they now take the theme's text colour (#380). Showing the details pane made the window grow a little on every frame, off the screen; the pane now takes its room from the list (#381).

- Eyedroppers (the tools and the document palette's): they read nothing from a locked object or one on a locked layer; they now read colours and attributes from them too, while applying to a locked object stays blocked (#365).

- Pen and Bezier: closing a curve with a click on its first node keeps the last segment as the preview showed it (it turned straight, #359); dragging from the first node to close puts its handle where the pointer goes (it was the mirror, #358); and a curve of two nodes closes, for a leaf or a lens of two nodes (#356).

- Repeat (Ctrl+R): after an undo it repeated the action just undone; it now repeats the last action still in effect (#366). After a copy made with a right click during a drag it moved the selection; it now makes another copy as far again (#363).

- `.cdr` (version 16 and later): outlines keep their mitre limit; it was read as 5 degrees whatever the file said, so sharp corners were cut where they should not be, and it was not saved (#384).

- `.cdr` (version 16 and later): text with letters outside Windows-1252 (Cyrillic, Greek, most Central European letters) was saved as unmarked UTF-8 and came back as garbage, in TraceDraw and in other programs; such letters are now saved as marked two-byte units, and files saved the old way read back right (#368).

- Text: editing a second text put the first one back to its text before its last edit; each text's typing is now an undo step of its own and editing another text never changes it (#373).

- Browser build: the interface followed the browser's dark mode and switched to a dark style of its own on reload; the theme is now the one chosen in the options alone.

- Interface size: 100 % is now one interface point per pixel at the system's scaling on every system (it was 0.8 on macOS and in the browser and 12 / 13 on Windows), and the bars' Default Button Size starts at Small, so the bars stay compact while text and menus take the system's size. A chosen interface size or button size is kept.

- Toolbox: the Variable Outline tool has a button of its own at the bottom of the toolbox, after the Fill flyout; it was inside the Shape tool's flyout, where it was hard to find (#339).

- Page bar: the Autofit Page button opens a popup with a Margin field (kept between sessions) and the Autofit Page button, so the page can be fitted to the drawing with room left around it; it fitted the page with no margin at once (#340).

- Text: typing in the property bar's font search field (or any other field with the keyboard) while a text is being edited put the characters into the text too; they now go to the field alone (#342).

## 0.11.0 (2026-10-22)

- Panels: any panel can be dragged by its tab or its title bar to the left side of the window, to the right side or anywhere over the drawing as a floating window (resizable; the tab's and the title's right-click menu offer the three places too), so panels can sit on both sides at once; every panel still starts at the right. Where each panel is survives a restart.

- Bars: the right-click menu of any bar lists every toolbar with a check (as Window > Toolbars does) and has Customize: Default Button Size (small, medium, large), Toolbox Toolbar (Reset to Default, its own button size, Image Only, Caption Only or Caption to Right) and Workspace import and export; it listed only five bars and had no sizes or captions.

- B-spline: a curve drawn with the B-spline tool keeps its control points. The Shape tool shows them with the dashed control polygon instead of the curve's nodes: drag one to reshape the spline, double-click the polygon to add one, double-click a point to remove it. Convert To Curves (or any other edit of the outline) makes it an ordinary curve. The control points were dropped as soon as the curve was finished.

- Smooth tool: a stroke eases the wobbles out of the curve under the nib and leaves a handful of nodes behind it; it left a node every few millimetres and broke the curve into short pieces as the pointer advanced. Smear and Twirl measure the curve finer under the nib and leave smoother curves too.

- Clip frames: while objects are moved over a clip frame it lights up and a note under the pointer says what the release does; W held adds the objects to the frame, K held replaces its contents. Edit Contents fades the rest of the page while the frame and its contents are edited.

- Clip frames: with a clip frame selected a bar at the top left of the drawing window offers Edit, Select Contents, Extract Contents, the contents lock and Fit Contents (centre, fit or fill proportionally, stretch); while the contents are edited it shows Finish.

- 3-point curve: a curve drawn from an end node of the selected curve continues it, and one ending on its other end node closes it, as the other curve tools do; it always made a curve of its own.

- Fonts embedded in a `.cdr` whose OS/2 table sets only a reserved embedding bit (old Devanagari fonts set bit 0) were refused as "restricted" and their text fell back to another font; the embedding bits are now read as the specification has them, so those fonts serve their text (#313).

- Number fields (property bar, panels, dialogs): spin arrows at the right of every field, and the mouse wheel over a field steps its value (Shift ten times as much, Ctrl a tenth); values could only be typed.

- Symmetry: several selected objects become one symmetry group (each still to be picked inside it with Ctrl+click), and objects drawn, pasted or duplicated while the symmetry is edited join it; before, each object got a symmetry of its own and nothing could be added.

- Panels: the plus button under the tabs lists the panels in the groups of Window > Panels, with a box ticked at each open one; unticking closes it.

- Panels: the pointer over a panel tab shows a close cross at the top of the tab, and a click on it closes the panel.

## 0.10.2 (2026-10-22)

- Opening `.cdr`: the bitmaps of a big drawing (a `Bitmaps.dat` member inflating past 512 MB, two large pictures on a 1.5 m page) were cut off at the limit, so the pictures after the cut were missing with "objects could not be read"; the member now inflates to the size the archive declares, up to 3 GB (#323).

- Browser build: Ctrl+R is Edit > Repeat, as on the desktop, instead of reloading the page (the browser's reload button still does).

- Pen and Bezier tools: a click after a node whose handle was pulled gives the curved segment the dashed preview showed, leaving along that handle and arriving straight at the new node; the segment came out straight, the handle thrown away (#314).

- Vertical text: Text > Vertical Text and two property bar buttons set text in columns read top to bottom, right to left, as CJK typesetting does, with ideographs, kana and hangul upright and Latin letters turned; carets, selections, clicks and the arrow keys follow the columns. Offered with a Japanese or Chinese interface language, and for any text in those scripts (#257).

- Property bar: with nothing selected, a Fit page to drawing button sits after the page width and height, the Layout command at hand (#236).

- Segment Delete: a curve drawn over itself (a loop inside one segment, or a segment coming back over the one before it) loses only the stretch clicked between the crossings; the whole curve went. Pen, Bezier, Polyline and B-spline: Esc finishes the curve being drawn as it stands, keeping the nodes placed and the tool, instead of discarding it (#314).

- Remove Background (on this computer): the contiguous fill now follows a backdrop that drifts in colour (a texture, a gradient the edges did not show, a soft shadow under the subject), stopping at the subject's sharp edge, so such backdrops go whole instead of leaving patches; up to six background colours are learnt from the edges; and the command is in a bitmap's context menu beside the AI one (#231).

- Print: the Printer list holds the system's printers on Linux and macOS (CUPS), the default one first, and a print on one of them goes straight to it with its copies, instead of only through the PDF viewer; the printer is remembered for the next print (#300).

- Save As: Embed fonts for TraceDraw files too: the installed fonts the text uses go into the `.tdraw` file when their licence allows it (one file per face), so the drawing keeps its letters on a machine without them; the box starts on for `.tdraw` and `.cdr` files alike (#275).

- Bitmaps: a linked bitmap (an imported file, or one sent to the system image editor by Edit Bitmap) follows its file on the desktop: a save in the outside editor shows in the drawing within a second, as Update from Link does by hand (#284).

- Guidelines: a value typed in the Guidelines panel's x, y or angle field moves the selected guideline as soon as it is entered, without the Modify button; the fields follow the selected guideline; a double click on a guideline opens the panel on it; and the property bar shows the selected guideline's type and position, where a typed value moves it too (#306).

- Properties docker: with characters selected in the text being edited, Fill, Outline and Character show those characters' own values (the object's where they have none) and edit them alone; the docker showed the object's values and could not adjust the characters' (#304).

- Curve tools: Options > Tools > Curve tools > Constrain angle key chooses the key that keeps the next segment to the constrain angle, Ctrl (as before), Shift or Alt (#247).

- Layout > Fit Page to Selection and Fit Page to Drawing (also on the Page Size options page): the page takes the size of the selection or of everything on it, and the objects move onto it; one undo step (#236).

- Alignment and dynamic guides: preferences saved by a version from before the guides existed had both switches on with nothing behind them, so after the update the magenta alignment lines showed for everyone with an older configuration. They are switched off once at the update, as on a first start; switched on by hand afterwards they stay on.

- Colour pickers (Properties fill and outline swatches and every other swatch that opens the popup): a Model list switches the colour's model on demand (RGB, CMYK, HSB, HSL, Lab, Grayscale, YIQ); the colour converts and the object keeps a colour of that model (#286).

- AI options: the OpenCode preset goes to the Console Inference API (`https://opencode.ai/inference/openai/v1/chat/completions`), which has replaced the Zen and Go addresses; the Zen and Go models are chosen by name there, so the two presets are one (#173).

## 0.10.1 (2026-10-21)

- Drawing speed: objects outside the part of the page on screen are no longer drawn, and while the view pans, scrolls or zooms the last render is shown moved into place and the page is rendered anew once the view rests for a moment, so a drawing of thousands of objects pans at the screen's rate instead of redrawing every object on every frame (#253).

- Pointers: with a dark theme the tool's icon on the drawing tools' cross hair showed as a plain grey box; the icon is now painted black on its light patch under every theme (#244).

- Windows portable zip: ships the `tracedraw-settings` folder beside the program, so the unzipped copy keeps its preferences there and writes nothing to %APPDATA% (#292).

- CMYK drawings: the object defaults kept in the configuration file by an earlier version no longer keep new text RGB black; a black text default, RGB or CMYK, is the drawing's own black, while a colour of the user's own stays (#259).

- Shaping (Weld, Trim, Intersect, Simplify, Front minus back, Back minus front, Boundary): the result keeps the curve segments of the objects, with new nodes only where their outlines cross; it was made of hundreds of tiny straight lines, hard to edit and faceted when enlarged. Two welded circles now give a handful of curve segments. Crop, Knife and Eraser cuts keep the curves the same way (#252).

- Shape brushes (Smooth, Smear, Twirl, Smudge, Roughen): a stroke reshapes only the stretch of the curve within the nib's reach and leaves the rest exactly as it was; it rebuilt the whole curve through points a few millimetres apart, so one stroke of Smooth over a freehand curve turned it into hundreds of nodes with the outline twisted and crossing itself. The reshaped stretch is fitted with few curves, and Smooth thins jags smaller than its rate before fitting (#227).

- Linked paragraph text frames: a copy of one frame (Duplicate, a right click during a move, Paste) is a frame of its own, so typing in it no longer flows into the original's next frame; a chain copied whole is a chain of the copies (#162).

- Clip frames: an object that holds others (Place Inside Frame) can be painted and reshaped again: a fill or outline goes to its frame, the status bar and Properties show the frame's, and the Shape tool edits the frame's nodes (a rectangle's corners round, an ellipse takes a pie or arc, a curve node by node) with the contents staying inside; the frame inside a clip frame now has an id of its own (#273).

- Browser: a right click during a move, scale, rotation or skew leaves the original and gives the change to a copy, as on the desktop; the browser reports a button pressed while another is held as a move, so the click was lost (#278).

- Shape brushes (Smooth, Smear, Twirl, Attract and Repel, Smudge, Roughen): the nib, the pointer's move and the settings' millimetres are measured on the page; on a scaled object (text converted to curves, a resized drawing) the brush reshaped the outline within a nib scaled with the object, so Smooth with a 10 mm nib could flatten 50 mm of a curve, and Smear moved it by a fifth of the drag (#227).

- Segment Delete: after a stretch of one outline is deleted, the piece left ends on the outline it crossed; clicking that other outline now deletes only the stretch up to the crossing instead of the whole outline (#227).

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
