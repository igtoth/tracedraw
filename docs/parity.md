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
| Grid | works (10 mm), snap to grid |
| Guidelines | works: drag out of a ruler, drag to move, Delete removes, View > Guidelines toggles |
| Snapping (page, guides, objects, grid) | works for move and drawing tools; View > Snap To |
| Scrollbars | works |
| View modes | Wireframe and Normal; Simple Wireframe, Draft, Enhanced, Pixels missing |
| Full-screen preview | missing |

## Tools

| Tool | Status |
|---|---|
| Pick | works: select, shift-add, marquee, move, scale with handles (corners proportional), second click rotate mode, nudge |
| Freehand Pick | works: lasso selects objects whose centre is inside |
| Shape | works: select nodes (click, shift, marquee), drag nodes and handles, cusp/smooth/symmetrical, add (double-click segment), delete (double-click node or Delete), break, close, to line/curve, reverse, select all |
| Smooth, Smear, Twirl | works as brushes on curves (nib size on the property bar); objects are converted to curves first |
| Crop | works: drag a rectangle, vector objects are clipped (bitmaps not yet) |
| Knife | works: drag a line, objects are cut in two (straight cuts only) |
| Eraser | works: click deletes an object (the vendor erases regions) |
| Zoom | works: click in, shift/right click out, drag box |
| Pan | works |
| Freehand | works: smoothed on release |
| 2-Point Line, Polyline | works |
| Bezier, Pen | works: click = cusp node, click-drag = handle, click start node closes, Enter/double-click finishes |
| B-Spline, 3-Point Curve | partial: behave like Bezier |
| Brush Strokes | partial: calligraphic nib (width, angle); no presets, sprayer or brush strokes |
| Rectangle | works, corner radius on property bar, Ctrl square, double-click page frame |
| 3-Point Rectangle | draws like Rectangle |
| Ellipse | works, Ctrl circle, pie and arc with start/end angles |
| 3-Point Ellipse | draws like Ellipse |
| Polygon, Star | works, points and sharpness on property bar |
| Spiral | works: symmetrical and logarithmic, revolutions |
| Common Shapes | works: arrow, heart, diamond, banner, callout, cross, lightning, triangle (no glyph editing) |
| Text | works: artistic text (click) and paragraph text (drag a frame, word wrap), system fonts with bold/italic, size, alignment; no text on path, no kerning/spacing controls, single style per object |
| Table | partial: rows x columns of cells grouped; no cell text editing, merging or borders |
| Parallel Dimension | partial: three clicks produce a static dimension line with arrowheads and measured text |
| Connector | partial: straight connector between two objects, static |
| Drop Shadow | works: drag sets offset; opacity, feathering, colour on property bar |
| Contour | partial: inside/outside, steps, offset, colour; baked into a group, not live |
| Blend | partial: steps between two objects, colour and outline interpolation; baked |
| Distort | partial: push/pull, zipper, twister; baked |
| Envelope | missing (the Envelope tool offers the distortions) |
| Extrude | partial: parallel extrusion with shaded faces; baked |
| Transparency | works: uniform transparency per object (slider on property bar) |
| Color Eyedropper | works (fill only) |
| Attributes Eyedropper | same as Color Eyedropper |
| Interactive Fill | partial: click applies default fill, drag makes a 2-stop linear fountain fill; radial via Properties docker; no on-canvas handles, no multi-stop |
| Mesh Fill, Area Fill | missing |

## Menus that work

File: New, Open, Save, Save As, Import (.cdr and bitmaps), Import Bitmap, Export (SVG), Export PDF, Document Properties, Exit.
Edit: Undo, Redo, Cut, Copy, Paste, Delete, Duplicate, Select All.
View: Wireframe, Normal, Zoom To Page/Fit/Selected, Rulers, Grid, Guidelines, Snap To (grid, guides, objects, page).
Layout: Insert Page, Duplicate Page, Rename Page, Delete Page, Go To Page, Page Size (dialog, presets, all pages), Switch Orientation.
Object: ClipFrame (Place Inside Frame, Extract Contents), Transformations (Position, Rotate, Scale and Mirror, Size, Skew, Clear), Align and Distribute (all align modes with L/R/T/B/E/C/P keys, four distribute modes), Order (all six), Group, Ungroup, Combine, Break Apart, Lock, Unlock, Unlock All, Shaping (Weld, Trim, Intersect, Simplify, Front Minus Back, Back Minus Front, Boundary; on flattened curves), Convert To Curves, Convert Outline To Object, Properties, Objects.
Text: Convert To Curves.
Tools: Options (Ctrl+J).
Window: Dockers, Objects (layers: rename, reorder, delete, lock, hide), Properties, Hints, Transformations, Undo, Status Bar.
Welcome Screen tab with new/open and page presets.
Help: About.

Everything else is listed and disabled.

## Rendering

| Feature | Status |
|---|---|
| Uniform fill | works (tiny-skia rasterizer, even-odd) |
| Fountain fill | works: two-stop linear and radial; no multi-stop, no conical/square |
| Outline width, hairline, caps, joins, dashes, scale with object | works |
| Transparency (uniform) | works |
| Drop shadow with feathering | works (box blur) |
| ClipFrame | works: contents masked by the frame |
| Bitmaps | works: import PNG/JPEG/BMP/GIF/WebP/TIFF, rotate/scale |
| Text | real fonts via fontdb + rustybuzz, glyph outlines |
| Anti-aliasing | works |
| Export | SVG (gradients, opacity, bitmaps), PDF (vector, CMYK/RGB fills, axial/radial shadings, ExtGState opacity, images) |
