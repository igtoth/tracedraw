# Traço

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
- Native `.traco` format (JSON) and SVG export.
- `.cdr` reader: RIFF and ZIP (X4+) containers, compressed (`cmpr`)
  streams, version detection, and best-effort parsing of pages, layers,
  rectangles, ellipses, curves, solid and fountain fills and outlines.
  **Not yet validated against real files**, see `docs/cdr-format.md`.
- Desktop app (egui): toolbox (pick, rectangle, ellipse, polygon, pan),
  canvas with zoom/pan, selection with handles, move, marquee, nudge,
  object properties docker (fill, outline), objects list, pages bar.

## Build and run

```sh
cargo run --release -p traco                 # empty A4 document
cargo run --release -p traco -- file.cdr     # open a the editor file
cargo test --workspace
```

Requires a Rust toolchain (1.80+). On Linux the app needs the usual
windowing libraries (X11 or Wayland, libxkbcommon, Mesa).

## Layout

```text
crates/core   document model, geometry (kurbo), commands, engine  (no UI deps)
crates/cdr    the editor .cdr reader: container, RIFF walker, object parser
crates/io     SVG export, native .traco load/save
apps/traco    egui desktop app (thin shell over the engine)
docs/         architecture notes, CDR format notes, roadmap
```

Layering rule: `core` depends on nothing in the workspace; `cdr` and `io`
depend only on `core`; only `apps/` may depend on egui/eframe/rfd.

## Shortcuts

| Action | Keys |
|---|---|
| Pick / Rectangle / Ellipse / Polygon / Pan | Space, F6, F7, Y, H |
| Zoom to page | Shift+F4 |
| Zoom | Ctrl+wheel, pinch |
| Undo / Redo | Ctrl+Z / Ctrl+Shift+Z |
| Group / Ungroup | Ctrl+G / Ctrl+U |
| Select all / Delete | Ctrl+A / Del |
| Nudge | Arrows (Shift = 10x) |
| Open / Save / Export SVG | Ctrl+O / Ctrl+S / Ctrl+E |

## Clean-room notice

Traço is implemented from public documentation, reverse-engineering notes
and observation of real files only. It contains no the vendor code or assets.
the editor is a trademark of its vendor; Traço is not affiliated
with, sponsored by or endorsed by the vendor.

## License

MIT or Apache-2.0, at your option.
