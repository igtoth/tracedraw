# Feature parity

The capabilities of the target design, grouped the way its own feature
list groups them, each with a status. "works" means it does what the
target design does, with a test or a checked screenshot behind it.
"partial" names what is missing. "baked" means the result is applied once
instead of staying live and editable. The goal is every row at "works".

Counts: works 78, partial 16, missing 6.

## 1. Selection and navigation

| Capability | Status |
|---|---|
| Pick: click, shift-click, marquee, Alt-marquee (touch), Alt-click dig, Tab cycling | works |
| Second click rotate/skew handles, centre-of-rotation drag | works |
| Freehand Pick (lasso) | works |
| Free Transform (rotate, reflect, scale, skew about a point) | works |
| Zoom tool, zoom levels, zoom to page/fit/selected, marquee zoom, Shift+F4 | works |
| Pan, mouse wheel, Ctrl and Shift wheel | works |
| Rulers with draggable origin, guidelines from rulers, Guidelines docker | works |
| Grid: document, pixel, baseline; snapping with threshold | works |
| Snap to objects, page, guidelines, dynamic and alignment guides; Alt+Q snap off | works |
| View modes: Wireframe, Normal, Enhanced, Pixels; full-screen preview (F9); preview selected only | works |
| Page sorter view | works |
| Navigator and page tabs | works |

## 2. Drawing

| Capability | Status |
|---|---|
| Rectangle, 3-point rectangle, corner radius, page frame on double-click | works |
| Ellipse, 3-point ellipse, pie and arc | works |
| Polygon, Star, Complex star, Spiral (symmetric, logarithmic), Graph paper, Common shapes, Impact | works |
| Freehand, 2-point line, Bezier, Pen, B-spline, Polyline, 3-point curve | works |
| Shape Recognition (shape recognition) and Sketch (stroke merging) | works |
| Brush Strokes: Preset, Brush, Sprayer, Calligraphic, Expression | works |
| Dimensions: parallel, horizontal/vertical, angular, segment, 3-point callout | works |
| Connectors: straight, right-angle, rounded right-angle; anchors | partial: anchor editing tool not interactive |
| Text (artistic and paragraph), Table | works |
| Crop, Knife, Segment delete, Eraser | works |
| QR code and page number insertion | works; barcode (Code 128, EAN) missing |

## 3. Shape and node editing

| Capability | Status |
|---|---|
| Shape tool: move nodes and handles, marquee select nodes, Elastic mode | works |
| Node types: cusp, smooth, symmetrical; to line / to curve; reverse subpaths | works |
| Add, delete, join, break, extract subpath, close curve, align nodes, reduce nodes | works |
| Weld, Trim, Intersect, Simplify, Front minus back, Back minus front, Boundary (menu and docker) | works |
| Combine, Break apart, Group, Ungroup, Ungroup all, Join curves | works |
| Convert to curves, Convert outline to object | works |
| Smear, Twirl, Attract, Repel, Smudge, Roughen, Smooth brushes | works |
| Lock, hide, order (front/back of page and layer, one step, in front of, behind, reverse) | works |
| Align and Distribute (all modes, align to page/edge/centre/grid/active object) | works |
| Transformations docker: position, rotate, scale, size, skew; apply to duplicate | works |
| Step and Repeat | works |
| Align with pixel grid | works |

## 4. Text

| Capability | Status |
|---|---|
| Artistic text and paragraph text frames, in-place editing | works |
| Font family, size, bold, italic, alignment, underline, strikethrough | works |
| Leading, paragraph spacing, indents, tabs, columns, bullets, drop cap | works |
| Tracking, baseline shift, OpenType features (ligatures, small caps, old-style figures, fractions, swash) | works |
| Hyphenation (Liang patterns en/pt, heuristic elsewhere) | works |
| Text on path (offset, distance, mirror), straighten text | works |
| Fit text to frame, wrap paragraph text around objects | works |
| Linked paragraph text frames | missing |
| Change case, insert formatting codes, show non-printing characters | works |
| Text statistics, Find and Replace, Glyphs docker, font filter and sample | works |
| Spell check (system Hunspell word lists) | works; grammar and thesaurus missing |
| Missing-font substitution with report | works |
| Convert text to table and table to text | works |
| Table: cell typing, Tab navigation, insert/delete rows and columns, merge/split, distribute | works |

## 5. Fills and outlines

| Capability | Status |
|---|---|
| Uniform fill in RGB, CMYK, Gray, HSB, HSL, Lab, YIQ, registration | works |
| Fountain fill: linear, radial, conical, square; multi-stop; angle, offset, edge pad; interactive drag | works |
| Pattern fill: two-colour (8 tiles), bitmap; texture fill (clouds, marble, noise, wood) | works; full-colour vector pattern and PostScript fills missing |
| Mesh fill with node editing | works |
| Area Fill (enclosed region) | works |
| Interactive Fill tool | works |
| Outline: width, colour, caps, joins, dashes, nib, behind fill, scale with object, arrowheads | works |
| Outline Pen and Outline Color dialogs (hidden flyout, Options toggle) | works |
| Eyedroppers: colour and attributes | works |

## 6. Interactive effects

| Capability | Status |
|---|---|
| Drop shadow (offset, opacity, feather, colour), live | works |
| Contour (steps, offset, direction, colour blend), live | works |
| Blend (steps, rotation, accelerations, path), live | works |
| Distort (push/pull, zipper, twister), live | works |
| Envelope (presets, keep lines, node drag), live | works |
| Perspective (Add Perspective, node drag) | works |
| Extrude (depth, vanishing point, bevel, lighting), live | works |
| Bevel (soft edge, emboss) | works |
| Block shadow | works |
| Transparency: uniform, fountain, pattern, texture; merge modes; fill/outline/all | works |
| Lens (11 types, frozen, rate, palette rotation) | works |
| ClipFrame (place inside, extract, empty frame, text frame) | works; edit in place and lock contents missing |
| Symbols (create, instances, Symbols docker) | works |
| Copy and clone effects, Clear effect, Clear transformations | works |
| Rollover | missing |

## 7. Colour

| Capability | Status |
|---|---|
| Colour models with conversions, colour docker, mixer, gamut warning | works |
| Palettes: default, document palette, open/save, palette editor, palette manager, from document/selection | works |
| Colour styles and harmonies | works |
| Object styles | works |
| Overprint fill and outline | works |
| Proof colours (soft proofing with the built-in CMYK model) | works |
| ICC colour management | partial: built-in sRGB/generic CMYK matrices; external ICC profiles not parsed |
| Separations | works (PDF separations export) |

## 8. Bitmaps and images

| Capability | Status |
|---|---|
| Import PNG, JPEG, BMP, GIF, WebP, TIFF; crop, resample, straighten | works |
| Convert to bitmap (resolution, colour mode, transparent background) | works |
| Bitmap effects: 3D, adjust, art strokes, blur, camera, colour transform, contour, correction, creative, custom, distort, noise, sharpen, texture, transform | works |
| Colour modes: 1-bit, grayscale, RGB, CMYK | works |
| Bitmap colour mask | works |
| Inflate bitmap (auto and manual) | works |
| Bitmap tracing: quick, centreline, outline; presets; smoothing, detail, colour count | works |
| Edit bitmap in an external editor, linked bitmaps | missing |

## 9. Layout and document

| Capability | Status |
|---|---|
| Multiple pages, insert/duplicate/rename/delete/move, go to page, page sorter | works |
| Page size presets and orientation, custom sizes, bleed, printable area | works |
| Page background: solid or bitmap | works |
| Master layers (all, odd, even pages), layer visibility, lock, printable | works |
| Objects docker (layers and objects tree, drag to reorder) | works |
| Document properties and metadata, rendering resolution | works |
| Guidelines docker, presets, angled guides | works |
| Insert page number (active layer, all, odd, even) | works |
| Templates: save as template, new from template | works |

## 10. Import and export

| Capability | Status |
|---|---|
| Open `.cdr` RIFF (v3 to X3) and X4/X5 ZIP | works for the supported generations |
| Open `.cdr` X6 and later (`content/root.dat`) | missing (next reader milestone) |
| Native `.tdraw` save/load | works |
| Import SVG and SVGZ (groups, clips, gradients, images); open SVG as a document | works |
| Export SVG, PDF, AI (PDF-compatible), EPS, PNG, JPEG, WebP, GIF, BMP, TIFF | works |
| Export for Web and Office presets | works |
| PDF: fills rasterised when needed, arrowheads, separations | works; PDF/X profiles missing |
| Import AI, EPS, PDF, DXF, PSD, CMX | missing |

## 11. Productivity and customisation

| Capability | Status |
|---|---|
| Undo/redo with history docker, repeat (Ctrl+R) | works |
| Copy, paste, paste in view, duplicate, clone, copy properties from | works |
| Find and Replace (text and object attributes) | works |
| Scripts docker: JavaScript object model, run, record macro, load/save | works |
| Workspaces: Default, Lite, Classic, Illustration, Page Layout; toolbar toggles | works |
| Options: general, workspace, document, snapping, text, tools, shortcuts (customisable) | works |
| Save settings as default | works |
| Hints docker with per-tool help; Welcome Screen (recent, templates, news, learn) | works |
| User interface in 12 languages with system fallback fonts | works |
| Keyboard shortcuts of the target design | works |

## 12. Print and prepress

| Capability | Status |
|---|---|
| Print (PDF to the system viewer), copies, range, bleed marks | works |
| Print preview | works |
| Print merge from CSV (create/load, edit, perform) | works |
| Separations and overprint simulation | partial: separations export; overprint preview missing |

## Workspace look

| Element | Status |
|---|---|
| Menu bar, standard toolbar, property bar, toolbox with flyouts, rulers, document tabs, bottom palette, docker tab strip, navigator, status bar | works |
| Icons | painted vector icons, one per tool, close in style; not pixel-identical |
| Dockers (30): Properties, Objects, Hints, Transformations, Undo, Align and Distribute, Shaping, Step and Repeat, Text, Glyphs, Colour, Colour Styles, Object Styles, Find and Replace, Scripts, Palettes, Lens, Blend, Contour, Envelope, Extrude, Bevel, Brush Strokes, Bitmap Mask, Object Data, Links, Symbols, Pages, Guidelines, Fonts | works |
| Welcome Screen with tabs | works |
| Right-click context menus (object, node, page, table) | works |
| Dialogs: New Document, Options, Export, Print, Print Merge, Colour Management, Font Manager, Document Properties, Convert to Bitmap, Resample, Trace, QR Code, Change Case, Text Statistics, Tabs, Columns, Bullets, Drop Cap, Table create/split, Page Number Settings | works |
| Window management (new window, cascade, tile) | not applicable: single document window with tabs |
