# the editor parity

The 92 capabilities of the editor, grouped as the vendor groups them, each with a
status. "works" means it does what the vendor does, with a test or a checked file
behind it. "partial" names what is missing. "baked" means the result is
applied once instead of staying live and editable. The goal is every row
at "works".

Counts: works 31, partial 19, missing 42.

## 1. Selection and navigation

| Capability | Status |
|---|---|
| Pick | works: select, shift-add, marquee, move, scale (corners proportional), second click rotate/skew handles, nudge, Tab cycling |
| Freehand Pick | works: lasso |
| Shape | works: node select/drag, handles, cusp/smooth/symmetrical, add, delete, break, close, to line/curve, reverse |
| Free Transform | missing |
| Zoom, Pan | works: click/shift-click/box, wheel, pinch, scrollbars, zoom levels |
| Eyedropper (colour) | works for fills |
| Eyedropper (attributes) | partial: same as colour eyedropper; outline, shadow and transparency not copied |
| Rulers, guides, measurement | works: rulers in document units, pointer markers; guides from rulers |
| Dimension (linear, angular, callout) | partial: linear only, static |
| Connector (smart, interactive) | partial: straight, static |
| Area Fill (paint bucket) | missing |

## 2. Drawing

| Capability | Status |
|---|---|
| Rectangle | partial: one corner radius for all corners (the vendor: per corner, round/scalloped/chamfered) |
| Ellipse | works: ellipse, pie, arc |
| Polygon, Star | works |
| Spiral | works: symmetrical, logarithmic |
| Graph Paper | missing |
| Bezier, Pen | works: click = cusp, drag = handle, close on start |
| B-Spline | partial: behaves like Bezier |
| Freehand | works |
| 2-Point Line | works |
| Shape Recognition | missing |
| Brush Strokes: brush, sprayer, calligraphic, pressure | partial: calligraphic only |
| Table | partial: grid of cells; no text in cells, merging, borders |

## 3. Shape and node editing

| Capability | Status |
|---|---|
| Node types cusp/smooth/symmetrical | works |
| Add, delete, reduce nodes; align nodes; extend curve; auto-close | partial: add/delete/close work; reduce, align, extend missing |
| Line/curve conversion; outline to object | works |
| Break, combine, join, break apart, group | partial: join curves missing |
| Free distortion: Smear, Twirl, Attract, Repel, Smooth, Roughen | partial: Smear, Twirl, Smooth as brushes; Attract, Repel, Roughen missing |
| Knife, Eraser | partial: knife straight cuts; eraser deletes whole objects |
| Boolean: weld, trim, intersect, simplify, front minus back, back minus front, boundary | works (on flattened curves) |
| Lock, hide, order (front, back, in front of, behind) | partial: lock and order front/back/one step; hide object and in-front-of/behind missing |

## 4. Text

| Capability | Status |
|---|---|
| Artistic and paragraph text (frame, columns) | partial: both exist; no columns |
| Text on path, fit to shape | missing |
| Indent, tabs, forced breaks, bullets, drop cap | missing |
| Leading, paragraph spacing, kerning, tracking, baseline shift | missing |
| Hyphenation, justification | partial: justify aligns left |
| OpenType features | missing |
| Character window, glyph browser | missing |
| Text styles | missing |
| Spell check, find and replace | missing |
| Font manager, playground, missing-font substitution | partial: fallback to sans-serif |
| TrueType, OpenType, Type 1 | partial: TrueType and OpenType via fontdb |

## 5. Fills and outlines

| Capability | Status |
|---|---|
| Uniform fill | works |
| Fountain: linear, radial, conical, square | partial: two-stop linear and radial |
| Pattern (bitmap, two-colour, full-colour), texture, PostScript | missing |
| Mesh fill | missing |
| Area fill | missing |
| Outline: width, colour, dashes, caps, corners, arrowheads, pen styles | partial: arrowheads and pen styles missing |

## 6. Interactive effects

| Capability | Status |
|---|---|
| Blend (with path) | partial: baked, no path |
| Contour | partial: baked |
| Distort | partial: baked, push/pull, zipper, twister |
| Envelope | missing |
| Perspective | missing |
| Extrude (lighting, rotation, surface colour) | partial: parallel, shaded faces, baked |
| Drop Shadow | works |
| Transparency: uniform, fountain, pattern, texture | partial: uniform only |
| Lens (11 types) | missing |
| ClipFrame, Fit to path | partial: ClipFrame works; fit to path missing |
| Symbols, clones | missing |
| Bevel, Glow, Relief | missing |

## 7. Colour

| Capability | Status |
|---|---|
| CMYK, RGB, HSB, HSL, LAB, grayscale, YIQ, registration | partial: CMYK, RGB, gray stored; others missing |
| Spot colours and libraries | missing |
| Custom palettes, palette manager | missing |
| Colour styles, harmonies | missing |
| Colour mixing, gamut alarm | missing |
| ICC colour management | missing |
| Proofing, separations | missing |
| Overprint | missing |

## 8. Bitmaps and images

| Capability | Status |
|---|---|
| Bitmap tracing | missing |
| Bitmap effects (blur, art strokes, camera, colour transform, creative, distort, noise, sharpen, texture, contour, relief) | missing |
| Bitmap colour mask, recolour | missing |
| Image adjustments | missing |
| Crop image, import with resolution | partial: import at 96 dpi; crop vector only |
| the photo editor | out of scope |

## 9. Layout and document

| Capability | Status |
|---|---|
| Multiple pages, page sorter, page setup, presets | partial: pages, dialog, presets; sorter missing |
| Bleed, crop marks, registration marks | missing |
| Rulers, guides, grid, dynamic guides, snapping (guides, objects, nodes, grid) | partial: dynamic guides and node snapping missing |
| Layers and master layers | partial: layers; master missing |
| Objects docker | works (basic) |
| Document navigator, document properties | works |
| Object, document and graphic styles | missing |
| Templates and template manager | missing |
| Hyperlinks and bookmarks | missing |
| Object data | missing |

## 10. Import and export

| Capability | Status |
|---|---|
| Native CDR, CDT | partial: .cdr read untested on real files; no write; native format is .tdraw |
| Import AI, EPS, PDF, SVG, DXF, PSD, CMX, WMF, JPEG, PNG, TIFF, BMP, GIF, TGA, PCX, DOC, DOCX, XLS | partial: bitmaps only |
| Export CDR, CDT, AI, EPS, PDF (PDF/X), SVG, PSD, DXF, PNG, JPEG, TIFF, BMP, GIF, WMF, HTML5 | partial: SVG, PDF |
| Export for Web | missing |

## 11. Productivity and customisation

| Capability | Status |
|---|---|
| Macros, VBA | missing |
| Customise toolbars, dockers, shortcuts, workspace | missing |
| CONNECT (clipart, fonts, photos) | out of scope |
| CAPTURE | out of scope |
| Context help, tutorials | partial: Hints docker |
| OLE | out of scope |

## 12. Print and prepress

| Capability | Status |
|---|---|
| Print proofs, separations, composite | missing |
| Scaled print, PostScript | missing |
| Bleed and crop marks in print | missing |
| Imposition, RIP options | missing |
| PDF/X-1a, X-3, X-4 | missing |

## Workspace look (the editor 2019)

| Element | Status |
|---|---|
| Menu bar, standard toolbar, property bar, toolbox with flyouts, rulers, document tabs, bottom palette, docker tab strip, navigator, status bar | works |
| Icons | painted vector icons, close in style; not pixel-identical |
| Dockers: Properties, Objects, Hints, Transformations, Undo | works |
| Welcome Screen | partial: new/open/presets |
