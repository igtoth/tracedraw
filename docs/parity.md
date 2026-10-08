# the editor parity

Measured against the default workspace. "wired" means the menu item
or tool exists in the UI; "works" means it does what the vendor does, with a test
or a checked file behind it. The goal is every row at "works".

## Workspace

| Element | Status |
|---|---|
| Menu bar: File, Edit, View, Layout, Object, Effects, Bitmaps, Text, Table, Tools, Window, Help | wired, items listed with the vendor's names and shortcuts |
| Standard toolbar | works: New, Open, Save, Cut, Copy, Paste, Undo, Redo, Import, Export, zoom combo |
| Property bar (context-sensitive) | works: page, object (X Y W H, scale, angle, mirror, outline, order), rectangle, polygon/star, text, zoom |
| Toolbox with flyouts and shortcuts | wired: all 16 groups; implemented tools below |
| Rulers (document units) | works |
| Colour palette (left fill, right outline, X none) | works |
| Dockers: Properties, Objects, Hints | works (basic) |
| Document navigator (page tabs, add page) | works |
| Status bar (cursor, object info, fill/outline swatches) | works |
| Grid | works (10 mm, no snapping) |
| Guidelines, snapping, full-screen preview, view modes | missing |

## Tools

| Tool | Status |
|---|---|
| Pick | works: select, shift-add, marquee, move, scale with handles (corners proportional), second click rotate mode, nudge |
| Freehand Pick | same as Pick (no freehand marquee yet) |
| Shape | works: drag nodes of curves; double-click converts to curves; no node add/delete, no handle editing |
| Smooth, Smear, Twirl | missing |
| Crop, Knife | missing |
| Eraser | works: click deletes an object (the vendor erases regions) |
| Zoom | works: click in, shift/right click out, drag box |
| Pan | works |
| Freehand | works: smoothed on release |
| 2-Point Line, Polyline | works |
| Bezier, Pen | partial: click places nodes, smoothed; no drag-for-handles |
| B-Spline, 3-Point Curve | missing (fall back to Bezier behaviour) |
| Brush Strokes | missing |
| Rectangle | works, corner radius on property bar, Ctrl square, double-click page frame |
| 3-Point Rectangle | draws like Rectangle |
| Ellipse | works, Ctrl circle; no pie/arc |
| 3-Point Ellipse | draws like Ellipse |
| Polygon, Star | works, points and sharpness on property bar |
| Spiral, Common Shapes | missing |
| Text | partial: artistic text, type in place, font/size on property bar; no paragraph text, no real font rendering (egui default font) |
| Table | missing |
| Parallel Dimension, Connector | missing |
| Drop Shadow, Contour, Blend, Distort, Envelope, Extrude | missing |
| Transparency | missing |
| Color Eyedropper | works (fill only) |
| Attributes Eyedropper | same as Color Eyedropper |
| Interactive Fill | partial: click applies default fill, drag makes a 2-stop linear fountain fill; no on-canvas handles |
| Mesh Fill, Area Fill | missing |

## Menus that work

File: New, Open, Save, Save As, Import (.cdr), Export (SVG), Exit.
Edit: Undo, Redo, Cut, Copy, Paste, Delete, Duplicate, Select All.
View: Zoom To Page/Fit/Selected, Rulers, Grid.
Layout: Insert Page, Delete Page, Page Size (A4/A3/Letter), Switch Orientation.
Object: Order (all six), Group, Ungroup, Convert To Curves, Properties, Objects.
Text: Convert To Curves.
Window: Dockers, Objects, Properties, Hints, Status Bar.
Help: About.

Everything else is listed and disabled.

## Rendering

| Feature | Status |
|---|---|
| Uniform fill | works |
| Fountain fill | partial: drawn as the average colour; export to SVG is correct |
| Outline width, hairline | works |
| Caps, joins, dashes | model only; renderer ignores them |
| Text | egui font, no bold/italic, no kerning control |
| Anti-aliasing | egui tessellator (feathered edges) |
