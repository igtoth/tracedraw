# TraceDraw

**An open-source vector illustration editor in pure Rust.** Pages, layers,
curves, text, fountain, pattern and texture fills, interactive effects,
ClipFrame, a clean-room `.cdr` reader, SVG and PDF export, and a desktop
workspace built for people who already know how a professional vector
editor is laid out: menu bar, property bar, toolbox with flyouts, dockers,
colour palette, rulers and a page navigator.

No Electron, no webview, no runtime: a single native binary (egui/eframe)
for Windows, macOS and Linux, plus a command-line tool for batch
conversion.

Status: **alpha**. The workspace and the drawing core are usable; a lot of
the professional surface (text layout, colour management, print,
bitmap tracing) is still being built. `docs/parity.md` lists every
capability and its state, and is the measure of progress.

## Highlights

- **Document model**: multi-page documents, layers, guides, rectangles,
  ellipses (pie, arc), polygons and stars, spirals, Bezier curves, artistic
  and paragraph text, groups, ClipFrame containers, bitmaps.
- **Fills**: uniform (RGB, CMYK, grayscale), fountain fills with any
  number of stops (linear, radial, conical, square), two-colour and bitmap
  patterns, procedural textures.
- **Outlines**: width or hairline, caps, corners, dash patterns,
  arrowheads, calligraphic nib, behind-fill, scale-with-object.
- **Tools**: Pick (move, scale, rotate, skew, nudge), Freehand Pick, Shape
  (full node editing: cusp, smooth, symmetrical, add, delete, break, close,
  line and curve conversion), Crop, Knife, Eraser, Zoom, Pan, Freehand,
  2-Point Line, Bezier, Pen, B-Spline, Polyline, Brush Strokes, Smart
  shapes, Table, Dimension, Connector, Text, Drop Shadow, Contour, Blend,
  Distort, Extrude, Transparency, Eyedropper, Interactive Fill, Smear,
  Twirl, Smooth.
- **Shaping**: weld, trim, intersect, simplify, front minus back, back
  minus front, boundary; combine, break apart, convert outline to object.
- **Rendering**: CPU rasteriser (tiny-skia) with anti-aliasing, uniform
  transparency, drop shadows, wireframe view. Text is shaped with
  rustybuzz from the system's fonts and becomes real outlines.
- **Files**: native `.tdraw` (JSON), `.cdr` reader (RIFF and ZIP
  containers, compressed streams, versions 7 through 2019 best effort),
  bitmap import (PNG, JPEG, BMP, GIF, TIFF, WebP), SVG and PDF export.
- **Workspace**: welcome screen, document tabs, context-sensitive property
  bar, dockers (Properties, Objects, Hints, Transformations, Undo), colour
  palette, rulers, guides, snapping, page navigator, status bar, dialogs
  for pages, layers, document properties and options.

## Screenshots

See `docs/screenshots/` (captured with the headless driver in
`scripts/visual/`).

## Install

Installers are built by GitHub Actions on every push to `main` and
published on the Releases page for `v*` tags:

| Platform | File |
|---|---|
| Windows | `tracedraw-<version>-windows-x86_64-setup.exe` (Inno Setup) and a portable zip |
| macOS | `tracedraw-<version>-macos-arm64.dmg`, `-macos-x86_64.dmg` (unsigned: right-click, Open) |
| Linux | `.deb`, `.AppImage`, tarball |

Packaging sources live in `packaging/`.

## Build from source

```sh
cargo run --release -p tracedraw                 # empty A4 document
cargo run --release -p tracedraw -- file.cdr     # open a file
cargo run --release -p tracedraw-cli -- convert in.cdr out.pdf
cargo test --workspace
```

Rust 1.80 or newer. On Linux the app needs X11 or Wayland, libxkbcommon
and Mesa.

## Architecture

```text
crates/core       document model, commands, undo engine, geometry (kurbo),
                  node editing, shaping, effects, styles, colour   (no UI deps)
crates/cdr        .cdr reader: container, RIFF walker, object parser
crates/text       font discovery (fontdb), shaping (rustybuzz), outlines
crates/render     CPU rasteriser (tiny-skia): fills, outlines, effects
crates/io         native .tdraw, SVG and PDF writers
apps/tracedraw    desktop app (egui/eframe), a thin shell over the engine
apps/tracedraw-cli  inspect, info, convert
docs/             architecture, decisions, behaviour notes, format notes,
                  parity, acceptance criteria, roadmap
```

Every user-visible change is a `Command` applied through the engine, which
is what gives undo/redo, the CLI and automation the same code path. See
`docs/architecture.md` for the full picture and `docs/decisions.md` for
the choices behind it.

## Documentation

| Document | What it covers |
|---|---|
| `docs/architecture.md` | crates, data flow, units, rendering, text |
| `docs/decisions.md` | stack, rendering, text, PDF, colour, licensing |
| `docs/parity.md` | every capability of the target design and its status |
| `docs/behavior/` | exact behaviour, defaults and formulas per tool |
| `docs/acceptance.md` | measurable acceptance criteria and performance targets |
| `docs/i18n.md` | user interface languages |
| `docs/cdr-format.md` | what the `.cdr` reader assumes and what is confirmed |
| `docs/roadmap.md` | milestones |
| `AGENTS.md` | rules for contributors and AI agents |

## Shortcuts

| Action | Keys |
|---|---|
| Pick / Shape / Zoom / Pan | Space, F10, Z, H |
| Freehand / Bezier / Rectangle / Ellipse / Polygon / Text | F5, B, F6, F7, Y, F8 |
| Interactive Fill / Eyedropper | G, flyout |
| Zoom to page / fit / selected | Shift+F4 / F4 / Shift+F2 |
| Undo / Redo | Ctrl+Z / Ctrl+Shift+Z |
| Group / Ungroup / Convert to curves | Ctrl+G / Ctrl+U / Ctrl+Q |
| Combine / Break apart | Ctrl+L / Ctrl+K |
| Cut / Copy / Paste / Duplicate | Ctrl+X / Ctrl+C / Ctrl+V / Ctrl+D |
| Order: front / back / forward / back one | Shift+Home / Shift+End / Ctrl+PgUp / Ctrl+PgDn |
| Align left/right/top/bottom/centre | L, R, T, B, E, C, P with a selection |
| Nudge | Arrows (Shift = 10x) |
| Open / Save / Import / Export | Ctrl+O / Ctrl+S / Ctrl+I / Ctrl+E |
| Options | Ctrl+J |

## Clean-room notice

TraceDraw is implemented from public documentation, published
reverse-engineering notes and observation of real files only. It contains
no code or assets from any other vector editor and is not affiliated with
any vendor. Licensed colour libraries and third-party content libraries
are deliberately not included; see `docs/decisions.md`.

## License

MIT or Apache-2.0, at your option.
