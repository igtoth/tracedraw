# Feature parity

The capabilities of the target design, grouped the way its own feature
list groups them, each with a status. "works" means it does what the
target design does, with a test or a checked screenshot behind it.
"partial" names what is missing. "baked" means the result is applied once
instead of staying live and editable. The goal is every row at "works".

Counts: works 141, partial 2 (`.cdr` content coverage; polygon nodes in the Shape tool), missing 3 (CMX import; window cascade and tile; some Options pages), not applicable 3, icons close but not pixel-identical.

## 1. Selection and navigation

| Capability | Status |
|---|---|
| Pick: click, shift-click, marquee, Alt-marquee (touch), Alt-click dig, Tab cycling | works |
| Second click rotate/skew handles, centre-of-rotation drag | works |
| Freehand Pick (lasso) | works |
| Free Transform (rotation, angle reflection, scale, skew modes, Apply to Duplicate) | works |
| Zoom tool, zoom levels, zoom to page/fit/selected, marquee zoom, Shift+F4 | works |
| Pan, mouse wheel, Ctrl and Shift wheel | works |
| Rulers drawn as the reference (numbers from the origin without sign, unit name, origin button, pointer markers), draggable origin, double-click for ruler settings, guidelines from rulers, Guidelines docker; all 13 units | works |
| Grid: document (lines or dots, spacing or frequency, thinned when dense, through the origin), pixel (Pixels view from 800%, colour and opacity), baseline (spacing, start from top, colour); snapping with threshold | works |
| Snap to objects, page, guidelines, dynamic and alignment guides; Alt+Q snap off, Alt+Z objects, Ctrl+Y grid | works |
| Snapping modes (node, intersection, midpoint, quadrant, tangent, perpendicular, edge, center, text baseline) with snap location marks and screen tips; moves carry the grabbed point (Options > Snapping) | works (see `behavior/snapping.md`) |
| View modes: Wireframe, Normal, Enhanced, Pixels; full-screen preview (F9); preview selected only | works |
| Page sorter view | works |
| Navigator and page tabs | works |

## 2. Drawing

| Capability | Status |
|---|---|
| Rectangle, 3-point rectangle, page frame on double-click; Ctrl square, Shift from centre | works (see `behavior/shape-tools.md`) |
| Rectangle corners: Round, Scalloped, Chamfered; a size per corner; Edit Corners Together; Relative Corner Scaling (property bar, Properties docker, Options > Toolbox > Rectangle tool) | works (see `behavior/shape-tools.md`) |
| Ellipse, 3-point ellipse, pie and arc | works |
| Polygon, Star, Complex star, Spiral (symmetric, logarithmic), Graph paper, Common shapes, Impact | works |
| Freehand, 2-point line, Bezier, Pen, B-spline, Polyline, 3-point curve | works |
| Shape Recognition (shape recognition) and Sketch (stroke merging) | works |
| Brush Strokes: Preset, Brush, Sprayer, Calligraphic, Expression | works |
| Dimensions: parallel, horizontal/vertical, angular, segment, 3-point callout | works |
| Connectors: straight, right-angle, rounded right-angle; Anchor Editing tool (add, move, delete anchors; connectors snap to them) | works |
| Text (artistic and paragraph), Table | works |
| Browser version (WebAssembly, WebGL 2): opens, edits and exports documents with uploads and downloads | works |
| Import of HPGL plotter files (PLT) and export of pages as pen strokes | works |
| Import of TXT, RTF and DOCX as paragraph text with run formatting | works |
| Export to PSD with one raster layer per document layer | works |
| Text editing in place: caret, selection by keyboard and mouse, word selection, character-level bold/italic/underline, clipboard | works |
| Crop, Knife, Segment delete, Eraser (band subtraction with thickness and nib shape) | works (see `behavior/crop-knife-eraser.md`) |
| QR code, barcode (Code 128, EAN-13, EAN-8) and page number insertion | works |

## 3. Shape and node editing

| Capability | Status |
|---|---|
| Shape tool: move nodes and handles, marquee select nodes, Elastic mode | works |
| Shape tool on rectangles (corner nodes, all or one corner), ellipses (pie inside, arc outside) and polygons (mirrored nodes) | partial: polygon nodes move in and out only, a drag's angle is not used (no twisted stars) (see `behavior/shape-tool.md`) |
| Node types: cusp, smooth, symmetrical; to line / to curve; reverse subpaths | works |
| Add, delete, join, break, extract subpath, close curve, align nodes, reduce nodes | works |
| Weld, Trim, Intersect, Simplify, Front minus back, Back minus front, Boundary (menu and docker) | works |
| Combine, Break apart, Group, Ungroup, Ungroup all | works |
| Join Curves docker: Extend, Chamfer, Fillet, Bezier Curve joints, gap tolerance, nearest ends first, closes subpaths | works (see `behavior/corners-and-joins.md`) |
| Corners docker: fillet, scallop, chamfer (A and B distances) the corners of curves or of chosen nodes, with preview; corners that do not fit are skipped | works (see `behavior/corners-and-joins.md`) |
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
| Linked paragraph text frames (link, unlink, live re-flow) | works |
| Change case, insert formatting codes, show non-printing characters, Encode, Make Text Web Compatible | works |
| Text statistics, Find and Replace, Glyphs docker, font filter and sample | works |
| Writing tools: spell check (system Hunspell word lists), grammar, thesaurus (built-in or MyThes file), Autocorrect | works |
| Missing-font substitution with report | works |
| Convert text to table and table to text | works |
| Table: cell typing, Tab navigation, insert/delete rows and columns, merge/split, distribute; fill and border on the property bar | works |

## 5. Fills and outlines

| Capability | Status |
|---|---|
| Uniform fill in RGB, CMYK, Gray, HSB, HSL, Lab, YIQ, registration | works |
| Fountain fill: linear, radial, conical, square; multi-stop; angle, offset, edge pad; interactive drag | works |
| Pattern fill: two-colour (8 tiles), bitmap, full-colour vector (tile from the selection); texture fill (clouds, marble, noise, wood) | works; PostScript fills not planned (legacy) |
| Mesh fill with node editing | works |
| Area Fill (enclosed region) | works |
| Interactive Fill tool: property bar with fill type and fountain fields, axis and centre handles on the canvas | works |
| Outline: width, colour, caps, joins, dashes, nib, behind fill, scale with object, arrowheads (presets and custom from the selection) | works |
| Outline Pen and Outline Color (hidden flyout, Options toggle); calligraphic nib rendered and exported | works |
| Eyedroppers: colour (bitmap sampling, fill or outline) and attributes (properties, transformations, effects groups) | works |

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
| Block shadow (depth, direction, colour, gap on the property bar) | works |
| Transparency: uniform, fountain, pattern, texture; merge modes; fill/outline/all | works |
| Lens (11 types, frozen, rate, palette rotation) | works |
| ClipFrame (place inside, extract, empty frame, text frame, edit in place, lock contents) | works |
| Symbols (Object > Symbol menu, create, insert, revert, Symbols docker, Ctrl+F3) | works |
| Copy effect, Clone effect (shadow and transparency follow the source until edited), Clear effect, Clear transformations, Symmetry (live, 1 to 12 mirror lines) | works |
| Rollover | not applicable: web-page interactivity with no HTML export target |

## 7. Colour

| Capability | Status |
|---|---|
| Colour models with conversions, colour docker, mixer, gamut warning | works |
| Palettes: the target design's default CMYK palette (99 named colours, the values of the first 64 checked on screen), document palette (filled as colours are applied, eyedropper, add from selection or document, delete, reset; read from `.cdr`), scroll arrows and wheel, show all colours, click and hold for shades, Ctrl+click to mix, open/save, palette editor, palette manager, from document/selection; Options > Customization > Color Palette | works |
| Colour styles and harmonies | works |
| Object styles | works |
| Overprint fill, outline and bitmap; Simulate Overprints preview | works |
| Proof colours (soft proofing with the built-in CMYK model) | works |
| CMYK shown as in the target design without a loaded profile (built-in press model of coated stock: default palette within 2.5 Delta E); RGB to CMYK with black-only greys and pure black | works |
| ICC colour management (v2/v4 profiles, matrix/TRC and LUT, four intents, black point compensation, gamut check); PDF/X output intent embedding | works |
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
| Edit bitmap in the system editor, linked bitmaps (update from link, break link) | works |

## 9. Layout and document

| Capability | Status |
|---|---|
| Multiple pages, insert/duplicate/rename/delete/move, go to page, page sorter | works |
| Page size presets and orientation, custom sizes, bleed, printable area | works |
| Page background: solid or bitmap | works |
| Master layers (all, odd, even pages), layer visibility, lock, printable | works |
| Objects docker (layers and objects tree, drag to reorder) | works |
| Document properties and metadata, rendering resolution, baseline grid | works |
| Guidelines docker, presets (seven presets, user-defined margins, columns, grid), angled guides, per-guideline colour, style and lock, rotate handles, drag off to delete, right-click Lock/Unlock, Select All > Guidelines | works |
| Insert page number (active layer, all, odd, even) | works |
| Templates: save as template, new from template | works |

## 10. Import and export

| Capability | Status |
|---|---|
| Open `.cdr`: RIFF (v3 to X3), X4/X5 ZIP, X6+ ZIP with redirected chunks | works (layouts confirmed against the public format description; real-file corpus still being collected) |
| `.cdr` content: shapes, curves, paths, polygons, groups, bitmaps (cropped ones as ClipFrames), text (artistic, paragraph, on a path), line spacing and indents, fills, outlines, preset and custom arrowheads, opacity | partial: splines, vector pattern and PostScript fills, tabs/bullets/drop caps from style tables, lenses and effects |
| Native `.tdraw` save/load | works |
| MCP server (`tracedraw --mcp`): open, new, save, export, run_script, document_info, undo, redo over JSON-RPC | works (see `behavior/mcp.md`; beyond the target design) |
| Save As `.cdr` (version 12 layout: pages, layers, rectangles, ellipses, curves, groups, text, bitmaps, solid and fountain fills, outlines, transparency) | works (round trip; see `cdr-format.md`, Writing) |
| Import SVG and SVGZ (groups, clips, gradients, images); open SVG as a document | works |
| Export SVG, PDF, AI (PDF-compatible), EPS, DXF, EMF, WMF, HTML (inline SVG, page tabs), PNG, JPEG, WebP, GIF, BMP, TIFF; Send To (Desktop, Documents, mail) as PDF; bitmaps, ClipFrames and bitmap transparency in SVG and PDF | works |
| Export for Web and Office presets | works |
| PDF: fills rasterised when needed, arrowheads, separations, PDF/X-1a, X-3 and X-4 with output intent, flattening and bleed | works |
| Import PDF and AI (PDF-compatible): paths, images, text, clips, shadings, patterns, forms, annotations | works (see `behavior/pdf-import.md`) |
| Import DXF (lines, polylines, arcs, ellipses, splines, hatches, text, blocks) | works (see `behavior/dxf.md`) |
| Import EPS and PostScript (interpreter: paths, images, text, clips, shadings, patterns, prologs) | works (see `behavior/eps-import.md`) |
| Import PSD/PSB (layers as bitmaps with opacity, visibility and masks) | works (see `behavior/psd-import.md`) |
| Import EMF and WMF (GDI paths, pens, brushes, text, DIB bitmaps, clips); export EMF and WMF | works (see `behavior/emf-wmf.md`) |
| Import CMX | missing |
| Acquire image (scanner) | not applicable: no scanner stack in pure Rust; import the scanned file instead |

## 11. Productivity and customisation

| Capability | Status |
|---|---|
| Undo/redo with history docker, repeat (Ctrl+R) | works |
| Copy, paste, paste in view, Paste Special (system clipboard as text, bitmap or objects), duplicate, clone, copy properties from | works |
| Find and Replace (text and object attributes) | works |
| Scripts docker: JavaScript object model, run, record macro, load/save | works |
| Workspaces: Default, Lite, Classic, Illustration, Page Layout; toolbar toggles | works |
| Options as the reference's dialogs: Tools > Options > TraceDraw (General with start-up and undo levels, Display, Edit with constrain angle and precision, Nodes and Handles, ClipFrame auto-centre, Snapping radius, Save with backups and auto-backup, Text), Customization (Appearance, Commands, Command Bars), Tools (Pick, Zoom/Pan, shape tools, Eraser), Global, Workspaces; Layout > Document Options (General with Fill open curves, Page Size, Layout, Background, Bleed, Rulers, Grid, Guidelines, Save as Default); Cancel restores | works |
| Options pages without an equivalent here: Tablet Mode, Bitmap tracing, Pen Settings, Warnings, VBA, Plug-ins, Printing, File Formats | missing |
| Save settings as default | works |
| Hints docker with per-tool help; Welcome Screen (recent, templates, news, learn) | works |
| Border and Grommet (large-format finishing) | works |
| Bitmap plug-ins (third-party filters) | not applicable: no plug-in host; the built-in bitmap effects cover the stock filters |
| User interface in 12 languages with system fallback fonts | works |
| Keyboard shortcuts of the target design (every menu shortcut bound; Esc closes dialogs) | works (see `behavior/shortcuts.md`) |

## 12. Print and prepress

| Capability | Status |
|---|---|
| Print (PDF to the system viewer), copies, range, bleed marks | works |
| Print preview | works |
| Print merge from CSV (create/load, edit, perform) | works |
| Separations and overprint simulation | works |

## Workspace look

| Element | Status |
|---|---|
| Menu bar, standard toolbar (icons, Open/Undo/Redo lists, zoom box, view toggles, Snap Off, Snap To, Launch), property bar (stacked fields, page bar, object bar with object origin, shape parts, outline part, zoom bar), toolbox with flyouts, rulers, document tabs, bottom palette, docker tab strip, navigator, status bar | works |
| Icons | painted vector icons, one per tool, close in style; not pixel-identical |
| Dockers (30): Properties, Objects, Hints, Transformations, Undo, Align and Distribute, Shaping, Step and Repeat, Text, Glyphs, Colour, Colour Styles, Object Styles, Find and Replace, Scripts, Palettes, Lens, Blend, Contour, Envelope, Extrude, Bevel, Brush Strokes, Bitmap Mask, Object Data, Links, Symbols, Pages, Guidelines, Fonts | works |
| Welcome Screen with tabs | works |
| Right-click context menus (object, node, page, table) | works |
| Dialogs: New Document, Options, Export, Print, Print Merge, Colour Management, Font Manager, Document Properties, Convert to Bitmap, Resample, Trace, QR Code, Barcode, Change Case, Text Statistics, Tabs, Columns, Bullets, Drop Cap, Table create/split, Page Number Settings, Paste Special, Symmetry, Thesaurus, Grammar, Autocorrect, Encode, Border and Grommet | works |
| Several open drawings in tabs (unsaved marker, close button, New after the last tab), Window list, Close, Close All, exit asks about each unsaved drawing | works (see `behavior/documents.md`) |
| Window > Cascade, Tile, undocking a drawing into its own window | missing |
