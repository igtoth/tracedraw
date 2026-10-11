# PDF and AI import

File > Open and File > Import accept `.pdf` and `.ai` (PDF-compatible
AI files, which carry a PDF after a PostScript preamble). Open
makes one page per PDF page; Import places the first page's objects on
the active layer.

## What is read

- Page geometry: MediaBox and CropBox (the crop box becomes the page),
  /Rotate 90, 180 and 270. Points become millimetres, the crop box's
  bottom-left corner is the origin.
- Paths: all construction operators, fill rules (even-odd is recorded as
  object data), strokes with width, caps, joins and dashes scaled by the
  CTM, constant alpha from ExtGStates (`ca`, `CA`).
- Colour: DeviceGray/RGB/CMYK, ICCBased (by component count), Indexed,
  Separation and DeviceN (as tints), CalRGB/Lab (as RGB).
- Clipping: `W`/`W*` paths become clip frames around the objects they
  clip; the intersection of nested clips is approximated by the inner
  one (or the shared rectangle). Objects fully inside a clip are not
  wrapped; text that is at least 60 % inside is left unclipped because
  substitute fonts change its width.
- Form XObjects (recursively, 12 levels), annotation appearance streams
  (stamps, form fields; links and popups skipped).
- Images: Flate/LZW/raw samples at 1, 2, 4, 8 and 16 bits with Gray, RGB,
  CMYK and Indexed spaces, Decode arrays, image masks (filled with
  black), SMask soft masks (resampled nearest), DCTDecode through the
  JPEG decoder. JPX images become a grey box. Inline images (`BI`) are
  read with their abbreviated keys expanded.
- Shadings: axial (type 2) and radial (type 3) as gradient fills, with
  type 2, 3 (stitching) and 0 (sampled) functions sampled into stops;
  `sh` paints the current clip. Shading patterns used as fills become
  gradient fills; tiling patterns become vector pattern fills built from
  the tile's content.
- Text: `BT`/`ET` blocks with Tf, Td, TD, Tm, T*, TL, Tc, Tw, Tz, Ts, Tr,
  Tj, TJ, ' and ". Consecutive strings on the same baseline merge into
  one artistic text object; a gap over 0.18 em (or a TJ adjustment under
  -180) inserts a space. Font family comes from BaseFont without the
  subset prefix (`Arial-BoldMT` becomes Arial, bold), bold and italic
  from the name, the descriptor flags and StemV. Characters decode
  through ToUnicode CMaps (bfchar, bfrange), then the font's Encoding
  (WinAnsi, MacRoman, Standard, Differences with glyph names), then
  Latin-1. Advances use Widths, W/DW for CID fonts, and Helvetica metrics
  for the standard 14. Render modes 3 and 7 (invisible) are skipped.
  Type 3 glyph procedures are not run.

## Not read

Soft masks on groups (`SMask` in ExtGState), blend modes, transfer
functions, optional content visibility, JPX and CCITT images (grey box
and skipped respectively), PostScript calculator functions (linear
approximation), embedded font programs (text uses the installed font of
the same name, so widths can differ). Encrypted files open when the user
password is empty.

## Checks

- A page with `MediaBox [0 0 200 100]` gives a 70.56 x 35.28 mm page.
- `1 0 0 RG 0 0 1 rg 4 w ... B` gives a blue fill with a red 1.41 mm
  outline.
- `(Hello) Tj ( World) Tj` on one line gives one text "Hello World" at
  12 pt bold; `[(Se) -250 (cond)] TJ` gives "Se cond".
- A form XObject with a Matrix is placed by it and takes the page's
  `ca`; a rectangle painted under `W n` becomes a clip frame.
- A type 2 axial shading pattern becomes a two-stop linear gradient.
- A raw 2 x 1 RGB image gives a 2 x 1 bitmap at the CTM's rectangle,
  also when the file starts with a PostScript preamble (`.ai`).
- Garbage and truncated input return an error, never a panic.
