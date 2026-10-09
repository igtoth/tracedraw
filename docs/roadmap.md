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

## M1 Real files (done for RIFF and X4/X5)
- `tracedraw-cli inspect` dumps chunk trees
- Text objects read as text with font and size; bitmaps read and drawn
- Layer names, page names, multi-page
- Open: X6+ container (`content/root.dat`), corpus per version in `corpus/`

## M2 Drawing (done)
- All 62 tools, node editing, transform handles, snapping, rulers
- tiny-skia renderer: fills of every kind, dashes, hairlines, effects
- Text engine with shaping, paragraph layout, text on path, tables
- Live effects, lenses, transparency, mesh fill, symbols, master layers

## M3 Output (done except `.cdr` write)
- PDF export (vector, CMYK, separations), EPS, AI (PDF-based), bitmaps
- Print to PDF, print preview, print merge
- Open: `.cdr` write (RIFF, targeting X3-compatible layout), PDF/X

## M4 Platform (in progress)
- CLI batch convert (done); installers for Windows, macOS, Linux (done)
- JavaScript automation with macro recording (done)
- User interface in 12 languages (done)
- Open: JSON control channel, MCP server, WebAssembly build

## M5 Interchange
- Import AI, EPS, PDF, DXF, PSD, CMX; export HTML, DXF, EMF/WMF
- External ICC profiles
- The remaining items in `docs/blueprint-gaps.md`
