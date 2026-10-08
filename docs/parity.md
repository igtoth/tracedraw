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
| Freehand Pick | same as Pick (no freehand marquee yet) |
| Shape | works: select nodes (click, shift, marquee), drag nodes and handles, cusp/smooth/symmetrical, add (double-click segment), delete (double-click node or Delete), break, close, to line/curve, reverse, select all |
| Smooth, Smear, Twirl | missing |
| Crop, Knife | missing |
| Eraser | works: click deletes an object (the vendor erases regions) |
| Zoom | works: click in, shift/right click out, drag box |
| Pan | works |
| Freehand | works: smoothed on release |
| 2-Point Line, Polyline | works |
| Bezier, Pen | works: click = cusp node, click-drag = handle, click start node closes, Enter/double-click finishes |
| B-Spline, 3-Point Curve | missing (fall back to Bezier behaviour) |
| Brush Strokes | missing |
| Rectangle | works, corner radius on property bar, Ctrl square, double-click page frame |
| 3-Point Rectangle | draws like Rectangle |
| Ellipse | works, Ctrl circle, pie and arc with start/end angles |
| 3-Point Ellipse | draws like Ellipse |
| Polygon, Star | works, points and sharpness on property bar |
| Spiral, Common Shapes | missing |
| Text | works: artistic text (click) and paragraph text (drag a frame, word wrap), system fonts with bold/italic, size, alignment; no text on path, no kerning/spacing controls, single style per object |
| Table | missing |
| Parallel Dimension, Connector | missing |
| Drop Shadow | works: drag sets offset; opacity, feathering, colour on property bar |
| Contour, Blend, Distort, Envelope, Extrude | missing |
| Transparency | works: uniform transparency per object (slider on property bar) |
| Color Eyedropper | works (fill only) |
| Attributes Eyedropper | same as Color Eyedropper |
| Interactive Fill | partial: click applies default fill, drag makes a 2-stop linear fountain fill; radial via Properties docker; no on-canvas handles, no multi-stop |
| Mesh Fill, Area Fill | missing |

## Menus that work

File: New, Open, Save, Save As, Import (.cdr and bitmaps), Import Bitmap, Export (SVG), Export PDF, Exit.
Edit: Undo, Redo, Cut, Copy, Paste, Delete, Duplicate, Select All.
View: Wireframe, Normal, Zoom To Page/Fit/Selected, Rulers, Grid, Guidelines, Snap To (grid, guides, objects, page).
Layout: Insert Page, Delete Page, Page Size (A4/A3/Letter), Switch Orientation.
Object: Transformations (Position, Rotate, Scale and Mirror, Size, Skew, Clear), Align and Distribute (all align modes with L/R/T/B/E/C/P keys, four distribute modes), Order (all six), Group, Ungroup, Combine, Break Apart, Lock, Unlock, Unlock All, Shaping (Weld, Trim, Intersect, Simplify, Front Minus Back, Back Minus Front, Boundary; on flattened curves), Convert To Curves, Properties, Objects.
Text: Convert To Curves.
Window: Dockers, Objects, Properties, Hints, Transformations, Status Bar.
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
| Bitmaps | works: import PNG/JPEG/BMP/GIF/WebP/TIFF, rotate/scale |
| Text | real fonts via fontdb + rustybuzz, glyph outlines |
| Anti-aliasing | works |
| Export | SVG (gradients, opacity, bitmaps), PDF (vector, CMYK/RGB fills, axial/radial shadings, ExtGState opacity, images) |
