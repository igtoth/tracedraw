# Gaps against the reference blueprint

Cross-check of the project against the public feature inventory of the
target design's 2019 release (toolbox help page, quick start guide,
reviewer's guide) and the community notes on the `.cdr` container.
Rows move to `parity.md` when done; this file keeps only what is open.

## Toolbox (17 groups, official list)

All 62 tools exist with their own icon, flyout position, property bar and
shortcut, and all of them are interactive (the Anchor Editing tool was the
last one; see `behavior/connectors-and-anchors.md`).

## Menu items

Every item of the twelve menus is wired, with these documented
exceptions, shown disabled so the layout stays complete:

| Menu | Item | Why |
|---|---|---|
| File | Acquire Image | no scanner (TWAIN/WIA/SANE) stack in pure Rust; import the scanned file |
| Object | Rollover | web-page interactivity with no HTML export target |
| Bitmaps | Plug-ins | no third-party filter host; the built-in effects cover the stock filters |
| Window | New Window, Cascade, Tile, Combine, Dock | single window with document tabs by design |
| Help | Highlight What's New | no per-release highlight data yet |

## Fills, colour and effects

- PostScript fills (legacy; vector pattern fills are done).

## File format

- `.cdr` generations: `WL` binary (v1, v2), RIFF (v3 to X3), ZIP with
  `content/riffData.cdr` (X4, X5), ZIP with `content/root.dat` and
  `content/data/*.dat` ordered by `content/dataFileList.dat` (X6 and
  later, every release since 2012). TraceDraw reads all three container
  layouts (RIFF, the X4/X5 ZIP and the X6+ ZIP with redirected chunks);
  the `WL` binary of v1 and v2 is not read. Writing `.cdr` is not planned
  before the reader is complete.
- Coordinates: 1/1000 inch (16-bit versions, `V < 600`), 1/254000 inch
  otherwise.
- Compressed lists: two zlib parts; the second is a size table that the
  first one's chunk sizes index into.
- Read: pages, layers, rectangles, ellipses, curves, paths, polygons,
  bitmaps, artistic and paragraph text (`font`, `stlt`, both `txsm`
  layouts from version 7 on; family, size, bold, italic, underline,
  strike-through, run fill, frame size, alignment), uniform, fountain
  (with the X6+ transformation), two-colour pattern (with the `bmpf`
  tile), colour bitmap and texture fills (the latter from the stored
  bitmap), outlines with dashes and arrowheads classified into our
  presets, object opacity.
- Done since: groups (`grp ` lists, nested), text fitted to a path
  (from the frame flag and the sibling curve), line spacing, character
  spacing and indents from `stlt`, custom arrowhead outlines kept as
  custom arrowheads.
- Still open in the reader: splines (0x26, layout not public), vector
  pattern (10) and PostScript (6) fills, text before version 7,
  single-byte code pages other than Windows-1252, tabs, bullets and drop
  caps from `stlt` (tables skipped by size), the `ftil` fill transform,
  lenses and other effects. Layouts marked "assumed" in
  `cdr-format.md` (`bmpf`, `arrw`, the X6+ style string content, the
  opacity direction, the alignment codes) have no file behind them yet.
- Chunks with public descriptions: `vrsn`, `DISP`, `LIST cmpr`, `stlt`,
  `font`, `txsm`, `mcfg`, `loda`, `trfd`, `fild`, `outl`, `bmp `, `sumi`.

## Formats to add

Import: CMX, EMF, WMF, DWG, CGM, PLT, DOCX, RTF, TXT.
Export: CMX, EMF, WMF, PSD.
Done: SVG, SVGZ, PDF, AI (PDF-compatible), EPS/PostScript, DXF and PSD
import; SVG, PDF, AI
(PDF-compatible), EPS, DXF, HTML, PDF/X-1a, X-3, X-4 and the bitmap
formats on export.

## Automation

The JavaScript object model covers documents, pages, layers, shapes,
fills, outlines and colours. Open: `Curve` > `SubPath` > `Node` access,
effects from scripts, import/export filters, and a script editor with
breakpoints.
