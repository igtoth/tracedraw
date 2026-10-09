# Gaps against the reference blueprint

Cross-check of the project against the public feature inventory of the
target design's 2019 release (toolbox help page, quick start guide,
reviewer's guide) and the community notes on the `.cdr` container.
Rows move to `parity.md` when done; this file keeps only what is open.

## Toolbox (17 groups, official list)

All 62 tools exist with their own icon, flyout position, property bar and
shortcut. The one that is not interactive yet:

| Group | Tool | What is missing |
|---|---|---|
| Connector | Anchor editing | dragging anchor points on an object; connectors attach to bounding-box sides |

## Menu items still disabled

| Menu | Item | Note |
|---|---|---|
| Edit | Paste Special | paste as text / as bitmap / with link |
| View | Simulate Overprints, Rasterize Complex Effects | overprint preview compositing; effects are already rasterised |
| Object | Create Arrowhead | custom arrowheads need a `Custom(path)` arrowhead variant |
| Object | ClipFrame Edit in place / Lock contents | edit mode inside the frame; lock flag |
| Object | Object Hinting | pixel hinting for web export |
| Bitmaps | Edit Bitmap, Break Link, Update from Link | external editor round trip and linked bitmaps |
| Text | Link / Unlink paragraph frames | flowing text across frames |
| Text | Align to Baseline Grid | baseline grid exists for snapping only |
| Text | Grammatik, Thesaurus, Autocorrect | writing tools beyond spell check |
| Text | Make Text Web Compatible, Encode | legacy items |
| Tools | Border and Grommet | large-format print helper |
| Window | New Window, Cascade, Tile, Combine, Dock | single window with document tabs by design |

## Fills, colour and effects

- PostScript fills (legacy; vector pattern fills are done).
- PDF/X output intents embedding the loaded ICC profile.
- Rollover objects.

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
- Still open in the reader: splines (0x26), vector pattern (10) and
  PostScript (6) fills, text on a path (the path data is skipped), text
  before version 7, single-byte code pages other than Windows-1252,
  paragraph formatting (indents, tabs, bullets, drop caps, intervals from
  `stlt`), custom arrowhead outlines (classified, not kept), the `ftil`
  fill transform, lenses and other effects. Layouts marked "assumed" in
  `cdr-format.md` (`bmpf`, `arrw`, the X6+ style string content, the
  opacity direction, the alignment codes) have no file behind them yet.
- Chunks with public descriptions: `vrsn`, `DISP`, `LIST cmpr`, `stlt`,
  `font`, `txsm`, `mcfg`, `loda`, `trfd`, `fild`, `outl`, `bmp `, `sumi`.

## Formats to add

Import: AI, EPS, PDF, CMX, EMF, WMF, DXF, DWG, PSD, CGM, PLT, DOCX, RTF,
TXT. Export: CMX, EMF, WMF, DXF, PSD, HTML, PDF/X-1a, X-3, X-4.
Done: SVG and SVGZ import; SVG, PDF, AI (PDF-compatible), EPS and the
bitmap formats on export.

## Automation

The JavaScript object model covers documents, pages, layers, shapes,
fills, outlines and colours. Open: `Curve` > `SubPath` > `Node` access,
effects from scripts, import/export filters, and a script editor with
breakpoints.
