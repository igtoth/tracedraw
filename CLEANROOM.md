# Clean-room record

This file records where TraceDraw's knowledge of each file format comes
from, so that anyone can check that the format support was written
independently. The details of each layout, and whether a file confirmed
it, live in `docs/cdr-format.md` and in the pages of `docs/behavior/`.

## Rules

- No program is ever decompiled or disassembled, and no decompiled code,
  disassembly or memory dump is used anywhere: not in the repository, not
  in issues, not in conversations with AI tools.
- No code is ported or copied from other implementations of a format,
  whatever their license. The documentation of open-source readers may be
  read to confirm a layout; the code that reads it here is written from
  that understanding and from the files themselves.
- Only files the contributor owns or is allowed to use are inspected.
  Personal files used for checking are not committed.
- Proprietary documentation is never copied into the repository.
- Every new piece of format knowledge gets a line in the log below:
  date, what was learned, and from which source.

## Sources

| Format | Sources |
|---|---|
| `.cdr` | Publicly available, unofficial format notes describing the RIFF chunks, the ZIP containers and the coordinate units; inspection of files owned by the author with `tracedraw-cli inspect` and a hex viewer; round trips of files TraceDraw writes, read back by TraceDraw and by another open-source reader. The libcdr documentation was consulted only to confirm layouts; none of its code (MPL 2.0) was ported. |
| SVG | The W3C SVG 1.1 and SVG 2 specifications. |
| PDF, AI (PDF-compatible) | ISO 32000 (PDF 1.7) and the PDF/X standards. |
| EPS, PostScript | The public PostScript language reference and the EPSF specification. |
| DXF | The publicly available DXF reference. |
| PSD | The publicly available PSD file format specification. |
| EMF, WMF | The published [MS-EMF] and [MS-WMF] open specifications. |
| PLT (HPGL) | Public HPGL command references. |
| DOCX, RTF, TXT | ECMA-376 (Office Open XML) and the RTF specification. |
| ICC profiles | The ICC.1 specification (v2 and v4). |

## Log

| Date | Finding | Source |
|---|---|---|
| 2026-10-08 | `.cdr` RIFF container, chunk walker, compressed lists (two zlib parts and a size table), coordinates in 1/254000 inch | public format notes |
| 2026-10-08 | Object, fill and outline records of the RIFF layout (rectangles, ellipses, curves, solid and gradient fills, outlines) | public format notes |
| 2026-10-09 | ZIP containers: `content/riffData.cdr` (versions 14 and 15), `content/root.dat` with `dataFileList.dat` and 16-byte redirect records (version 16 and later) | public format notes, cross-checked against the public format description |
| 2026-10-09 | Page origin at the page centre, objects stored front to back, bitmaps with a crop outline, the document palette in `color/docPalette.xml`, one byte per character in text from version 17 | a real banner file owned by the author (format version 21, not committed) |
| 2026-10-09 | Groups, text fitted to a path, paragraph spacing and indents, custom arrowhead outlines | public format notes; marked "assumed" in `docs/cdr-format.md` until a file confirms them |
| 2026-10-09 | Writing the version 12 RIFF layout (pages, layers, shapes, groups, text, bitmaps, fills, outlines, transparency, clipped bitmaps as crop paths) | the same notes; verified by round trip and by another open-source reader rendering the written files |
