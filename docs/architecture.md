# Architecture

TraceDraw is a Cargo workspace. The dependency direction is strict and
reviewed: nothing below `apps/` knows about egui, and `core` knows about
nothing else in the workspace.

```text
apps/tracedraw, apps/tracedraw-cli
        |
        v
crates/io  ---------> crates/render
        |                   |
        v                   v
crates/cdr, crates/text --> crates/core
```

## core

The document model and everything that changes it.

- `document.rs`: `Document` > `Page` > `Layer` > `Shape`. A shape has a
  `ShapeKind` (rect, ellipse, polygon, path, text, group, ClipFrame,
  bitmap), an affine `transform`, `fill`, `stroke`, `opacity`, `shadow`,
  `visible`, `locked`, `name`. Ids are allocated by `IdSource` and never
  reused, even across undo.
- `command.rs`: the `Command` enum. Every user-visible mutation is a
  variant with a `label()` for the Undo docker.
- `engine.rs`: applies commands, keeps snapshot history (undo/redo),
  batches, dirty tracking. Snapshots are whole documents; a document of a
  few thousand objects is tens of kilobytes, so this stays simple until
  measurements say otherwise (`docs/acceptance.md`).
- `geometry.rs`: kurbo re-exports and path builders (rect, rounded rect,
  ellipse, arc, polygon, star, polyline, Catmull-Rom smoothing, RDP
  simplification).
- `nodes.rs`: the node model used by the Shape tool (node types, handle
  moves, add, delete, break, close, line/curve conversion, reverse).
- `shaping.rs`: boolean operations through `i_overlay`, offsets.
- `effects.rs`: blend, contour, extrude, distort, brushes. These bake
  their result today; live effects are a planned change to the model
  (an `Effect` list on the shape, re-evaluated on render).
- `style.rs`: `Fill` (none, solid, fountain, pattern, texture), `Stroke`
  (width, caps, joins, dashes, arrowheads, nib), `Color`.
- `color.rs`: RGB, CMYK and grayscale colours, conversions, hex.

Units: millimetres, Y up, origin at the page's bottom-left corner. Angles
in degrees, counter-clockwise positive. Text sizes in points.

## cdr

Clean-room reader for `.cdr`. `container.rs` detects RIFF or ZIP and the
version, `riff.rs` walks chunks and inflates `cmpr` lists, `parse.rs`
builds a `Document` best effort. Every assumption is recorded in
`docs/cdr-format.md` with its confirmation status.

## text

`fontdb` discovers system fonts, `rustybuzz` shapes runs, `ttf-parser`
delivers glyph outlines. The crate installs itself into `core` through a
function pointer (`document::text_outline::set`) so `core` can compute
text bounds without depending on fonts. Paragraph text is wrapped to its
frame width; alignment is applied per line.

## render

A CPU rasteriser on `tiny-skia`. `render_page` draws a page into a pixmap
through a `ViewTransform` (zoom, origin). Fills map directly to shaders
(solid, linear, radial) or are computed per pixel (conical, square,
textures) or tiled (patterns). ClipFrame uses a mask, opacity a scratch
layer, drop shadows a separable box blur. `render_fill_image` rasterises
a fill alone for exporters.

The app caches the rendered page as a texture keyed by document revision,
view and canvas size (`apps/tracedraw/src/raster.rs`), so the UI only
re-renders when something changed.

## io

- Native `.tdraw`: serde JSON of the `Document`. Stable field names;
  additive changes only, with `#[serde(default)]` for new fields.
- SVG writer: one `<g>` per layer, fills as gradients or data-URI
  patterns, outlines, arrowheads, text as paths, bitmaps as `<image>`.
- PDF writer: hand-written PDF 1.4, one page per page, paths, RGB and
  CMYK fills, axial and radial shadings with stitching functions,
  ExtGState opacity, images, patterns and textures as images.

## apps/tracedraw

`App` holds the engine, the current page, the view, the active tool, the
selection, drag state and tool defaults. `interaction.rs` turns pointer
and keyboard input into commands per tool; `ops.rs` holds the operations
that menus and shortcuts call; `ui/` lays out the workspace: menus,
standard toolbar, property bar, toolbox, dockers, palette, status bar,
document tabs, dialogs. `canvas.rs` draws the cached raster and the
overlays (handles, nodes, rubber bands, guides, rulers).

## apps/tracedraw-cli

`inspect` (chunk tree of a `.cdr`), `info` (pages, layers, objects),
`convert` (to `.tdraw`, SVG, PDF). It installs the text engine so text
exports identically to the desktop app.
