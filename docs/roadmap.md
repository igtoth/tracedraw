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

## M1 Real files
- `tracedraw-cli inspect` to dump chunk trees
- Corpus of small `.cdr` files per version (one object each) in `corpus/`
- Confirm every "assumed" row in `docs/cdr-format.md`
- Text objects (artistic text) read as text with font and size
- Bitmaps read and drawn
- Layer names, page names, multi-page

## M2 Drawing
- Bezier/pen tool and shape (node) tool with node editing
- Transform handles: scale, rotate, skew (second-click mode)
- Snapping to objects, guides, grid; rulers
- Proper renderer (scanline, then wgpu): non-convex fills, gradients,
  dashes, hairlines, anti-aliasing
- Artistic and paragraph text with real shaping (parley or cosmic-text)
- Fountain fill editor (multi-stop), pattern and mesh fills later

## M3 Output
- PDF export (vector, CMYK), EPS, AI (PDF-based)
- Print with page setup
- `.cdr` write (RIFF, targeting X3-compatible layout)

## M4 Platform
- CLI batch convert; JSON control channel; MCP server
- WebAssembly build
- Installers for Windows, macOS, Linux
