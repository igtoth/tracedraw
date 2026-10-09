# Roadmap

Reference product: the most recent workflow of the target design we
can observe (2019 workspace).
File compatibility target: open `.cdr` from 9 through X7 reliably; newer
versions best effort.

## M0 Skeleton (done)
Workspace, document model, commands + undo, SVG export, native format,
`.cdr` reader skeleton, egui app with pick/rect/ellipse/polygon tools.

## M0.5 Workspace shell (done)
The full workspace layout: menus, standard toolbar, property bar,
toolbox with flyouts, rulers, palette, dockers, navigator, status bar.
Parity tracked in `docs/parity.md`.

## M1 Real files (done for all three containers)
- `tracedraw-cli inspect` dumps chunk trees
- Text objects read as text with font and size; bitmaps read and drawn
- Layer names, page names, multi-page
- X6+ container (`content/root.dat`, redirected chunks); a 2019 file
  opens correctly (text, cropped bitmaps, object order)
- Open: corpus per version in `corpus/`, text on path, splines, paragraph
  formatting from the style tables

## M2 Drawing (done)
- All 62 tools, node editing, transform handles, snapping, rulers
- tiny-skia renderer: fills of every kind, dashes, hairlines, effects
- Text engine with shaping, paragraph layout, text on path, tables
- Live effects, lenses, transparency, mesh fill, symbols, master layers
- Symmetry, ClipFrame editing, linked text frames, connector anchors,
  writing tools, clone effects, custom arrowheads

## M3 Output (done except `.cdr` write)
- PDF export (vector, CMYK, separations), EPS, AI (PDF-based), bitmaps
- Print to PDF, print preview, print merge
- PDF/X-1a, X-3 and X-4 output (done)
- Open: `.cdr` write (RIFF, targeting X3-compatible layout)

## M4 Platform (in progress)
- CLI batch convert (done); installers for Windows, macOS, Linux (done)
- JavaScript automation with macro recording (done)
- User interface in 12 languages (done)
- Open: JSON control channel, MCP server, WebAssembly build

## M5 Interchange
- Import PDF, AI, EPS, DXF and PSD (done); CMX; export HTML and DXF (done), EMF/WMF
- External ICC profiles (done: pure-Rust ICC engine, v2/v4), PDF/X intents (done)
- The remaining items in `docs/blueprint-gaps.md`
