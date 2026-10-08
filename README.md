# TraceDraw

**An open-source vector illustration editor written in pure Rust, with a
classic workflow and a clean-room `.cdr` reader.**

Status: **pre-alpha**. The skeleton compiles and runs; most of the product
does not exist yet. See `docs/roadmap.md`.

## What works today

- Document model: pages, layers, rectangles, ellipses, polygons/stars,
  free paths, groups; fills (solid, two-stop linear/radial) and outlines
  (width, caps, joins, dashes, hairline). Units are millimetres, Y up,
  origin at the page's bottom-left, like the editor.
- Command engine with undo/redo. Every mutation is a `Command`, so the UI,
  a CLI and an agent can drive the same engine.
- Native `.tdraw` format (JSON) and SVG export.
- `.cdr` reader: RIFF and ZIP (X4+) containers, compressed (`cmpr`)
  streams, version detection, and best-effort parsing of pages, layers,
  rectangles, ellipses, curves, solid and fountain fills and outlines.
  **Not yet validated against real files**, see `docs/cdr-format.md`.
- Desktop app (egui) laid out like the editor: menu bar, standard toolbar,
  context-sensitive property bar, toolbox with flyouts and the vendor shortcuts,
  rulers, colour palette, dockers (Properties, Objects, Hints), page
  navigator and status bar. Working tools: Pick (move, scale, rotate),
  Shape (node drag), Zoom, Pan, Freehand, Bezier/Polyline, Rectangle,
  Ellipse, Polygon/Star, Text, Eyedropper, Interactive Fill, Eraser.
  Status per tool and menu in `docs/parity.md`.

## Build and run

```sh
cargo run --release -p tracedraw                 # empty A4 document
cargo run --release -p tracedraw -- file.cdr     # open a the editor file
cargo test --workspace
```

Requires a Rust toolchain (1.80+). On Linux the app needs the usual
windowing libraries (X11 or Wayland, libxkbcommon, Mesa).

## Layout

```text
crates/core   document model, geometry (kurbo), commands, engine  (no UI deps)
crates/cdr    the editor .cdr reader: container, RIFF walker, object parser
crates/io     SVG export, native .tdraw load/save
apps/tracedraw    egui desktop app (thin shell over the engine)
docs/         architecture notes, CDR format notes, roadmap
```

Layering rule: `core` depends on nothing in the workspace; `cdr` and `io`
depend only on `core`; only `apps/` may depend on egui/eframe/rfd.

## Shortcuts

| Action | Keys |
|---|---|
| Pick / Shape / Zoom / Pan | Space, F10, Z, H |
| Freehand / Bezier / Rectangle / Ellipse / Polygon / Text | F5, B, F6, F7, Y, F8 |
| Interactive Fill / Eyedropper (next click applies) | G, flyout |
| Zoom to page / fit / selected | Shift+F4 / F4 / Shift+F2 |
| Zoom | Ctrl+wheel, pinch |
| Undo / Redo | Ctrl+Z / Ctrl+Shift+Z |
| Group / Ungroup / Convert to curves | Ctrl+G / Ctrl+U / Ctrl+Q |
| Cut / Copy / Paste / Duplicate | Ctrl+X / Ctrl+C / Ctrl+V / Ctrl+D |
| Order: front / back / forward / back one | Shift+Home / Shift+End / Ctrl+PgUp / Ctrl+PgDn |
| Select all / Delete | Ctrl+A / Del |
| Nudge | Arrows (Shift = 10x) |
| Open / Save / Export SVG | Ctrl+O / Ctrl+S / Ctrl+E |

## Clean-room notice

TraceDraw is implemented from public documentation, reverse-engineering notes
and observation of real files only. It contains no the vendor code or assets.
the editor is a trademark of its vendor; TraceDraw is not affiliated
with, sponsored by or endorsed by the vendor.

## License

MIT or Apache-2.0, at your option.

## Installers

Every push to `main` builds installers as workflow artifacts (Actions tab,
pick the run, scroll to Artifacts); tagging `vX.Y.Z` publishes them on the
Releases page:

- Windows: `tracedraw-<version>-windows-x86_64-setup.exe` (Inno Setup;
  Start menu entry, `.tdraw` association, optional PATH) and a portable zip
- macOS: `tracedraw-<version>-macos-arm64.dmg` and `-macos-x86_64.dmg`
  (unsigned for now: right-click the app and choose Open the first time)
- Linux: `.deb`, `.AppImage` and a tarball

Packaging sources live in `packaging/`.
