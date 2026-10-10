# CDR format notes

What the reader assumes, and what has been confirmed. Status legend:

- **confirmed (public spec)**: matches the public, declarative description
  of the format (a Kaitai Struct specification built from observed files).
  We use that description as documentation only; no code is taken from it.
- **confirmed (file)**: checked against a named file in our corpus.
- **assumed**: from public reverse-engineering notes or our own reading,
  not yet checked.
- **wrong**: kept for history.

Versions in this document are major numbers: 7 = CDR 7, 13 = X3, 14 = X4,
15 = X5, 16 = X6, 17 = X7, 18 = X8, 19 = 2017. The public spec writes them
as hundreds (1300 = X3); `vrsn` stores the same number.

## Containers

| Item | Status | Note |
|---|---|---|
| `RIFF <size> CDR<v>`; `v` is a version char: `1`..`9` = 1..9, `A` = 10 ... `H` = X7 (17), `J` = X8 (18), `K` = 2017 (19); `I` is not used | confirmed (public spec) | We used to map `J` to 19. |
| Lowercase form type `cdr` with `8` is a CDR 8 variant (spec: 801) | confirmed (public spec) | Treated as version 8. |
| `vrsn` chunk: u16 version in hundreds (1300 = X3, 801 = CDR 8 variant); `fver` has u16 full version, patch, minor, major | confirmed (public spec) | We read `vrsn` and prefer it over the form-type letter when sane. |
| Chunk: fourcc, u32 size, payload, pad byte when the size is odd | confirmed (public spec) | |
| X4 (14) and X5 (15): ZIP with `content/riffData.cdr` holding the RIFF stream | confirmed (public spec) | |
| `LIST cmpr` (7 to X3 with compression on): two (compressed, uncompressed) u32 pairs, then two `CPng` blocks (`CPng`, `01 00 04 00`, zlib). Block 1 inflates to the chunk stream, block 2 to a pool of u32 sizes; inside the inflated stream each chunk's size field is an index into the pool | confirmed (public spec) | We used to treat the body as plain sub-chunks and the inflated stream as byte-sized chunks. |
| Chunk sizes 4-byte aligned in some newer files | wrong | Not in the public spec; all chunks pad to 2 bytes. |

### X6 and later (16+)

| Item | Status | Note |
|---|---|---|
| ZIP whose RIFF entry point is `content/root.dat`; `content/dataFileList.dat` lists the external stream members (one name per line), found under `content/data/` | confirmed (public spec) | `container.rs` |
| Any chunk whose declared size is exactly 16 bytes is a redirect record: `stream u32, length u32, offset u32, reserved u32`; the payload is `length` bytes at `offset` in stream `stream` (index into `dataFileList.dat`) | confirmed (public spec) | `riff.rs`, gated on version 16+. |
| Stream `0xFFFFFFFF` means the payload (at most 8 bytes) is stored inline right after the length field | confirmed (public spec) | |
| A redirected `LIST` body starts with its form type in the external stream | confirmed (public spec) | |

## Units and coordinates

| Item | Status | Note |
|---|---|---|
| Version 6+: coordinates are i32 in 1/254000 inch (0.0001 mm) | confirmed (public spec) | |
| Before version 6: coordinates are i16 in 1/1000 inch; header fields, counts, ids and offsets that are u32 later are u16 | confirmed (public spec) | New in the reader. |
| Angles: i32 in millionths of a degree (i16 in tenths of a degree before 6) | confirmed (public spec) | |
| Page origin is the page centre; shapes carry their own transform (`trfd`) | confirmed (file: 2019 banner) | A full-page picture sits at (-w/2, -h/2) after its translation. |
| Objects inside a layer are stored front to back (the first `obj ` in the file is the top-most) | confirmed (file: 2019 banner) | Text, then a patch, then the picture underneath. The reader reverses the order into its bottom-to-top layers. |
| `mcfg` holds the document page size even when the content bbox under `LIST page` is smaller; the `bbox` under a page is the content bounds, not the page | confirmed (file: 2019 banner) | Letter landscape page with a 247 x 164 mm picture; the master page has a near-empty bbox. |
| 2019 files (`CDRM`, `vrsn` 2100) parse with the X6+ rules: `root.dat`, redirect records, 0x05 bitmaps with `bmp ` in `Bitmaps.dat`, `txsm` layout version 1800 | confirmed (file: 2019 banner) | The file also carries `color/*.icc`, `styles/document.cdss` (JSON styles) and `META-INF/textinfo.xml` (plain text of every text object), which are not needed to read the drawing. |
| `color/docPalette.xml`: the document palette, `<palette name="Document Palette"><colors><page>` then one `<color cs="CMYK" name="Green" tints="1,0,1,0"/>` per colour, tints as fractions in the colour space's order | confirmed (file: 2019 banner) | Read into the document palette (CMYK, RGB, grey). `color/color.xml` names the RGB, CMYK and grayscale profiles, the colour model and the rendering intent. |
| `mcfg` page size: two coords (width, height) after 12 unknown bytes (X3+), 4 (9 to 12), 28 (version 6), 0 (7, 8, and 4 to 5); before 4: 2 unknown bytes then two corners | confirmed (public spec) | We lacked the version 6 offset and the pre-4 layout. |
| A page's own `loda` may carry argument 19130: width, height coords | confirmed (public spec) | Used per page when sane. |

## Structure

| Item | Status | Note |
|---|---|---|
| `LIST page` > `LIST layr` > `LIST obj ` > `loda` + `trfd`; groups (`LIST grp `) nest `obj ` lists and other `grp ` lists | assumed (layer > obj confirmed, file: 2019 banner) | Spec types the chunks but not the nesting. `grp ` lists become groups; other lists under a layer are descended, so an object list one level down is still found. |
| `flgs`: 4 bytes; byte 3 is the record type (0x08 object, 0x10 group, 0x90 page, 0x98 layer); for pages byte 2 != 0 marks the master page; for layers byte 0 is the layer type (0 normal, 0x08 desktop, 0x0a guides, 0x1a grid) | confirmed (public spec) | Guides and grid layers are imported hidden and unprintable. |
| Layer name: `loda` argument 1000 in the layer's own `loda`: NUL-terminated single-byte text before 12 (system code page), UTF-16LE from 12 | confirmed (public spec) | Single-byte text is decoded as Windows-1252 (the spec's own default; the real code page is the saving system's). |
| `lobj` has the same layout as `loda` | confirmed (public spec) | |
| `bbox`: four coords (two corners); `obbx`: eight coords (four corners) | confirmed (public spec) | Not used; we compute bounds. |
| `spnd`: u32 (u16 before 6); `usdn`: u32 static object id | confirmed (public spec) | Not used. |

## `loda` argument table

| Item | Status | Note |
|---|---|---|
| Header: length, argument count, offset of the offsets table, offset of the types table, object type (u32 each; u16 before version 6) | confirmed (public spec) | |
| Offsets table has argument count + 1 entries; the extra one ends the last argument | confirmed (public spec) | Used to bound each argument. |
| Types table is stored in reverse order: offsets[0] pairs with types[count - 1] | confirmed (public spec) | We used to pair them in the same order. |
| Argument types: 10 outline id, 20 fill id, 30 coordinates, 100 transform (pre-4), 200 style id, 1000 name, 2000 palette, 8000 opacity, 8005 container, 11000 polygon, 12010 gradient, 12030 rotate, 19130 page size, 40050 layer GUID | confirmed (public spec) | We read 10, 20, 30, 200, 1000, 8000, 11000, 19130. |
| Fill id and outline id are u32 references into the `fild`/`outl` tables (version 4+); before 4 the styles are inline | confirmed (public spec) | Inline pre-4 styles are not read. |
| Opacity argument: 10 unknown bytes (14 from X3), then u16 in thousandths | confirmed (public spec) | Applied to `Shape.opacity`. |
| The value is a transparency (0 = opaque), as the target design's uniform transparency slider; a value above 1.0 is percent in thousandths | assumed | The spec only says "value / 1000"; we pick transparency and compute `opacity = 1 - value`. |
| Style argument (200): style id (u32; u16 before 6) into the `stlt` table | confirmed (public spec) | Used as the default run style of text objects. |
| Object types: 0x01 rectangle, 0x02 ellipse, 0x03 line and curve, 0x04 artistic text, 0x05 bitmap, 0x06 paragraph text, 0x14 polygon, 0x25 path (X6+), 0x26 spline | confirmed (public spec) | Spline is not modelled. |

## Geometry (argument type 30)

| Item | Status | Note |
|---|---|---|
| Point list: `n` points of (x, y) coords followed by `n` type bytes; `n` is clamped to what fits | confirmed (public spec) | |
| Point type byte: bits 7..6 operation (00 move to, 01 line to, 10 cubic Bezier end point, 11 control point); bit 3 closes the subpath (also set on the move-to of a closed subpath); bits 5..4 node continuity (angle, smooth, symmetrical); bit 2 "can modify"; bit 1 character start | confirmed (public spec) | We had 10 and 11 swapped and closed on the move-to. |
| Rectangle before X5: width, height, radius (coords); from version 9 three more radii follow (four corners) | confirmed (public spec) | Our model has one radius; the largest is used. |
| Rectangle X5+: width, height as f64 in coordinate units, scale x, scale y (f64), scale-with flag u8, 7 unknown; then four radii as f64 each followed by a corner type byte and unknown bytes (16 bytes between radii) | confirmed (public spec) | Radii are multiplied by 254000 when the flag is set (spec); meaning of the flag is otherwise not stated. |
| Rectangle spans 0..width x scale and 0..height x scale from its origin | assumed | The spec derives half-extents from the same fields; we treat the fields as the full extent, like the ellipse. |
| Ellipse: width, height, start angle, end angle, pie flag (u32; u16 before 6); the ellipse spans 0..width, 0..height | confirmed (public spec) | Arc when the angles differ. |
| Line and curve (0x03): u32 point count then a point list, in all versions | confirmed (public spec) | We used a u16 count before X6. |
| Path (0x25): 4 unknown, two u16 counts (added), 16 unknown, point list | confirmed (public spec) | New. |
| Polygon (0x14): coordinates hold the base curve as a point list; argument 11000 holds: 4 unknown (before X3), angle count u32, next point u32, u32, 4 unknown (X3+), rx f64, ry f64, cx, cy coords | confirmed (public spec) | We used to read width/height. rx, ry taken as coordinate units (assumed). The base curve is not replicated; a regular polygon is drawn. |
| Bitmap (0x05): two corners (4 coords), 32 unknown, image id u32, unknown (8 before 4, 12 in version 8, else 20), u32 count, clipping point list | confirmed (file: 2019 banner) | The point list is the crop outline in the object's local space; when it is not the full rectangle the object is imported as a ClipFrame (frame = outline, contents = the bitmap). A patch cut from a larger picture confirmed it. |
| Artistic text (0x04): origin x, y; paragraph text (0x06): 4 unknown, width, height | confirmed (public spec) | Origin is the baseline start; the frame spans (0, 0) to (width, height) like our rectangles (assumed). |
| The `txsm` chunk of a text object is inside the object's `LIST obj ` | assumed | The spec does not state how a text object finds its content; the first `txsm` under the object list is used. |

## `trfd` transforms

| Item | Status | Note |
|---|---|---|
| Header: length, argument count, offset of the offsets table (u32 each; u16 before 6); no types table | confirmed (public spec) | We used to read a fourth header field. |
| Each record: 8 unknown bytes (X3+), type u16; type 0x08 is a matrix: 6 unknown bytes (version 6+) then six f64 `a c tx b d ty` | confirmed (public spec) | We had 4 + 2 (pre X3) and 8 + 2 + 2 (X3+) bytes before the matrix. |
| tx, ty are in coordinate units (1/254000 inch; 1/1000 inch before 6) | confirmed (public spec) | |
| Several matrices in one `trfd` | assumed | Only the first is used, with a warning. |

## `fild` fills (`fill` before version 7)

| Item | Status | Note |
|---|---|---|
| id u32; X3+: tag u32 (1300) and body length u32; type u16; body | confirmed (public spec) | We skipped only 4 bytes on X3+. |
| Fill types: 0 none, 1 uniform, 2 fountain, 6 PostScript, 7 two-colour pattern (older), 8 two-colour pattern (newer), 9 colour bitmap, 10 vector pattern, 11 texture | confirmed (public spec) | 6 and 10 become no fill with a warning. |
| Uniform before X3: 2 unknown bytes then a colour | confirmed (public spec) | We read the colour 2 bytes early. |
| Uniform X3+: tag u32, length u32, property list of (type u8, length u32, body) ending with type 0; type 1 = colour (12 bytes), 6 = Lab colour, 7 = palette GUID, 3/8/11/12 = special palette colour parts | confirmed (public spec) | Only the type 1 colour is used. |
| Fountain: 2 unknown (8 from X3), type u8, unknown 11 (before 6) / 19 (6 to 12) / 17 (X3+), edge offset (i32 in 6 to 12, else i16), angle, centre x, centre y (i32; i16 before 6), 2 unknown (6+), mode (u32; u16 before 6), mid point u8, 1 unknown, stop count (u32; u16 before 6; low 16 bits), 3 unknown (X3+), stops, 3 unknown (X3+), X6+: four f64 (centre x, centre y, width, height relative to the object) when present | confirmed (public spec) | We read the angle as f64 and skipped the stops. |
| X6+ transformation: the relative centre becomes `Fountain.offset` when the legacy centre offsets are zero; a width or height below 1 becomes `edge_pad = (1 - min(w, h)) / 2` when the legacy edge offset is zero; a fill larger than the object cannot be represented and is drawn at object size | assumed | The spec documents the fields, not their scale; "relative to the object" is read as a fraction of the object's size. |
| Fountain stop: colour, unknown 0 / 5 (X3, X4) / 26 (X5+), offset (u32; u16 before 6; low 16 bits in percent), 3 unknown (X3+) | confirmed (public spec) | Stop sizes 16 / 24 / 45 bytes. |
| Fountain type byte: 1 linear, 2 radial, 3 conical, 4 square | assumed | Not in the spec. |
| Centre offsets and edge offset are percentages | assumed | Scaling is not in the spec. |
| Two-colour pattern: 2 unknown (8 from X3), pattern id u32, width, height (i32; i16 before 6), tile offsets u16 x2 (before 9) or 4 unknown, rcp offset u16, flags u8 (bit 2 = relative size before 9), unknown 1 (6 from X3), colour 1, unknown 0 / 10 (X3 to X5) / 31 (X6+), colour 2 | confirmed (public spec) | The tile comes from `bmpf` (below); without it a placeholder tile is drawn. |
| `bmpf`: pattern id u32, then a device-independent bitmap: 40-byte info header (size 40, width, height, planes 1, 1 bit per pixel), two BGRx palette entries, rows padded to 4 bytes, bottom-up (top-down when the height is negative); the darker palette entry is the front colour | assumed | Not in the public spec. The header is searched for in the first 64 bytes and validated; anything else keeps the placeholder with a warning. |
| Image fills (9, 10, 11): leading records (X3+: a 0x640 word followed by 0x640 bytes, or a bare 0x514 word, repeated) or 2 unknown bytes, pattern id, width, height (u32; u16 before 6), tile offsets (before 9) or 4 unknown, rcp u16, flags u8, unknown 21 (17 from X3), pattern id again (6+) | confirmed (public spec) | Types 9 and 11 are read; 10 is not. |
| The image fill's pattern id names a `bmp ` image (same table as bitmap objects) | assumed | Colour bitmap fills become a tiled PNG at the stored tile width; textures are drawn from that bitmap too (not regenerated), or fall back to a built-in texture with a warning when the bitmap is missing. |

## `outl` outlines

| Item | Status | Note |
|---|---|---|
| id u32; X3+: records (id u32, length u32, body) until id 1, whose body is the outline | confirmed (public spec) | We skipped 4 fixed bytes. |
| line type u16, caps u16, join u16, 2 unknown (6 to 12), width coord, stretch u16 (percent), 2 unknown (6+), nib angle, unknown 0 / 52 (6 to 12) / 46 (X3+), colour, unknown 10 (before 6) / 16, dash count u16, then a fixed area of 20 (before 6) / 22 bytes whose start holds the dash values (u16 each), start marker u32, end marker u32 | confirmed (public spec) | Dashes are stored in multiples of the width. We used to read the markers right after the dashes. |
| `arrw`: arrowhead id u32, point count u32, point list as in geometry | assumed | Not in the public spec. The outline is classified into our presets: three corners = arrow (open arrow when not closed), four = square, bar (aspect under 0.35) or diamond (not axis-aligned), closed all-Bezier = circle; anything else is kept as a custom arrowhead from the outline itself (assumed: tip towards +x); an undefined marker id = none, with a warning. |
| line type bit 0 = no outline, bit 4 = behind fill, bit 5 = scale with object; caps 0 butt, 1 round, 2 square; join 0 miter, 1 round, 2 bevel | assumed | Not in the spec. |

## Colours

| Item | Status | Note |
|---|---|---|
| Version 5+: model u16, palette u16, 4 unknown bytes, 4 value bytes (12 bytes) | confirmed (public spec) | We read the value 4 bytes early. |
| Version 4: model u16, C M Y K as u16 each, 2 unknown (12 bytes); before 4: model u8, 4 value bytes | confirmed (public spec) | |
| Models: 1 Pantone, 2 CMYK percent, 3 CMYK 0..255, 4 CMY, 5 BGR, 6 HSB, 7 HLS, 8 black and white, 9 grayscale, 11 YIQ, 12 Lab (signed), 13 index, 14 Pantone hex, 15 Hexachrome, 17 CMYK 0..255, 18 Lab (offset 128), 20 registration, 21 BGR with tint byte (tint already applied), 22 user ink, 25 spot, 26 multi-channel, 99 mixed | confirmed (public spec) | We map 2, 3, 4, 5, 9, 17, 20, 21; others become black with a warning. Model 17 was wrongly read as percent. |

## `bmp ` bitmaps

| Item | Status | Note |
|---|---|---|
| image id (u32; u16 before 6), unknown 14 (before 6) / 46 (6) / 50, colour model u32, 4, width u32, height u32, 4, bits per pixel u32, 4, bitmap size u32, 32; palette when bpp < 24 and model not 5 or 6: 2 unknown, count u16, BGR triplets; then the pixel bytes | confirmed (file) | Colour model 1 is 24-bit BGR (a 2019 file, 700 x 464, model 1, 24 bpp, size 974400 = 700 x 3 rounded to 4 x 464). Another public reader draws model 5 as greyscale, so the writer uses 1. |
| Pixel rows padded to 4 bytes, stored bottom-up; 32 and 24 bpp are BGR(x), 8 and 1 bpp index the palette | assumed | Device-independent bitmap convention. |

## Text and styles

| Item | Status | Note |
|---|---|---|
| `font`: id u16, encoding u16, style flags u32, 10 unknown, name to the end (UTF-16LE from 12, single-byte before) | confirmed (public spec) | Gives the family name per font id. |
| Text encodings: UTF-16LE from version 12; single-byte code page before (the `font` encoding byte names it: 0 Latin, 0xcc Cyrillic, 0xee Central European ...) | confirmed (public spec) | We decode every single-byte string as Windows-1252; other code pages are not mapped yet. |
| Style flags u32: low 18 bits name the face (one bit each: thin, thin italic, ..., bold 0x1000, bold italic 0x2000, ..., heavy italic 0x20000), bits 18..20 underline, 21..23 overline, 24..26 strike-through, 27..28 script | confirmed (public spec) | Bold = any weight bit from semi-bold up; italic = any italic bit. |
| `stlt` (7+): the body of `LIST stlt` is one record, not sub-chunks: record count u32, then (when the count is not 0) the mapping tables, then the records | confirmed (public spec) | `riff.rs` leaves the list unparsed from version 7. Before 7 the list holds untyped sub-chunks we skip. |
| `stlt` mapping tables, each `count u32` then entries: fills (id u32, 4, value u32, 48 more from X3), outlines (12 bytes), fonts (id u32, 12 / 20 (10+) unknown, font id u16, encoding u16, 8, size coord, 8, style flags u32, 8 (10+)), alignments (12 bytes), intervals (52), set5 (152), tabs (784), bullets (variable), indents (16 + 3 coords), hyphens (32, 36 from X3), drop caps (28), set11 (12; only from the CDR 8 variant 801) | confirmed (public spec) | Fills, outlines, fonts, alignments, intervals and indents are kept. Our major version cannot tell 800 from 801; version 8 reads set11 as absent. |
| `stlt` interval entry: id u32, 8 unknown, character spacing u32, 8 unknown, line spacing u32, 24 unknown; both in millionths (1 000 000 = 100 %) | confirmed (public spec) for the layout; the meaning of the values is assumed | Line spacing becomes the paragraph leading in percent; character spacing becomes tracking in percent of the em when within -2000..2000 %. |
| `stlt` indent entry: id u32, 12 unknown, right, first line, left (coords) | confirmed (public spec) | Converted to millimetres into the paragraph indents. |
| `stlt` record: group count u32, style id u32, parent id u32, 8 unknown, name length u32 (characters), name, fill ref u32, outline ref u32; when the group count > 1: font ref, align ref, interval ref, set5 ref (and set11 ref when that table exists); when > 2: tab, bullet, indent, hyphen, drop cap refs | confirmed (public spec) | Refs index the mapping tables; the entry's value is the real `fild`/`outl`/`font` id. Parents are followed for unset properties. |
| Alignment values: 0 none, 1 left, 2 right, 3 centre, 4 full justify, 5 force justify | assumed | The order of the target design's public scripting enumeration; not in the spec. |
| `txsm` 7 to X5: frame flag u32, 32 unknown, 1 (X5), (before 8: on-path u32 and 32 bytes when set), frame count, frames, paragraph count, paragraphs | confirmed (public spec) | Versions before 7 are not described and are skipped with a warning. |
| `txsm` 7 to X5 frame: id u32, 48 unknown, (8+: on-path u32; when set 4, 8 (X3+), 28, 8 (X5); when clear 8 (X5)), then 36 (7) / 32 (8) / 34 (8 variant 801 to X3) / 36 (X4) / 40 (X5) bytes when the frame flag is clear, 4 bytes (X5) when set | confirmed (public spec) | Version 8 is read as 32 bytes (800); 801 files may misparse. |
| `txsm` 7 to X5 paragraph: style id u32, 1 unknown, 1 more (X3+ with the frame flag), style count u32, styles, char count u32, char descriptions (4 bytes; 8 from 12), byte count u32 (from 12; before: one byte per char), text, has-path u8, 24 bytes per char when set | confirmed (public spec) | |
| `txsm` 7 to X5 style: char count u16, flags u8, fl3 u8 (8+); flags: 0x01 font id u16 + encoding u16, 0x02 style flags u32, 0x04 size coord, 0x08 / 0x10 / 0x20 4 bytes each, 0x40 fill id u32 (+48 from X3), 0x80 outline id u32; fl3: 0x02 URL (len u32, len x 2 bytes), 0x08 locale (4 bytes; from X3 len u32 + len x 2), 0x20 a peeked flag byte that, when set, starts a 4 (52 from X5) byte block | confirmed (public spec) | Styles are run lengths over the paragraph text. Sizes are in coordinate units (0.0001 mm) and converted to points. |
| `txsm` X6+: frame flag u32, 32 unknown, layout version u16 (1500 ... 1800), 3 unknown, frame count, frames (id u32, 48 unknown, on-path u32 (+40 when set), 8 unknown, and when the frame flag is clear: 16 unknown, length u32, that many bytes (x 2 before X7)), paragraph count, paragraphs | confirmed (public spec) | |
| Text fitted to a path: the frame's on-path flag is set and the path is a sibling object of the text in the same `grp ` list | assumed | When a group holds flagged text and exactly one curve, the text is placed on that curve (offset 0, no distance). The on-path bytes themselves are skipped. |
| `txsm` X6+ paragraph: style id u32, 1 unknown, flag u8 (frame flag set; 64 bytes follow when it is 1), paragraph style string (layout < 1700 and no frame flag), default style string, record count u32, records (3 x u16; URL when the second is 0x3fff and the third has 0x11; locale when the third has 0x04; a style string when the second is not 0 or the third has 0x04), char count, char descriptions (flags u16, override index u8 >> 1, 1, 4), byte count, text, has-path u8 (+24 bytes per char) | confirmed (public spec) | The override index selects a record whose string overrides the default. Text encoding: UTF-16LE up to X8; from 2017 (17) one byte per character (UTF-8 when valid, else Windows-1252), confirmed with a 2019 file where a 6-character run is 6 bytes. |
| Style string: length u32 in UTF-16 units (bytes from X7), then the text | confirmed (public spec) | |
| Style string content: a JSON object with `character` (font id, size, fill id, outline id) and `paragraph` (justify) sections | assumed | Not in the spec. We parse JSON leniently and also accept `key:value;` lists, matching key names loosely (size, weight/bold, italic, fill, outline, justify/align, font/family). Sizes above 1000 are taken as coordinate units, smaller ones as points. |
| `ftil`: six f64 fill transform; `uidr`: colour id, user id, 36 unknown, colour; `DISP`: a preview bitmap header; `urls`: text | confirmed (public spec) | Not implemented. |

## Writing

`crates/cdr/src/write.rs` writes the version 12 layout (`CDRC`, 32-bit
fields): `vrsn` 1200; `doc ` with `mcfg` (4 unknown bytes, width,
height, padded to 48 bytes), `fntt`/`font`, `filt`/`fild` (types 0, 1 and
the version 6 to 12 fountain body), `otlt`/`outl` (version 9 to 12 layout)
and `bmpt`/`bmp ` (24-bit, model 1); then one `page` per page with `flgs`
0x90, a `gobj` list and one `layr` per layer (`flgs` 0x98, the layer's
`loda` with its name, then the objects front to back). Each object is an
`obj ` list with its `loda` (type, coordinates, fill id, outline id,
name, transparency), a `trfd` with one type 8 matrix (translation in
coordinate units, page centre as origin) and, for text, a `txsm` in the
7 to X5 layout with one style per run. Rectangles, ellipses and text
keep their own types; every other object is a curve; groups are `grp `
lists; a ClipFrame holding one bitmap is written as that bitmap with the
frame as its crop path (the way the target design stores cropped
bitmaps), other ClipFrames become a group of the frame outline and the
contents; effects are evaluated first. A `stlt` with one complete style
(id 0) accompanies the font table and text objects reference it. The file reads back in this crate (round
trip tests in `write.rs`) and another public reader of the layout
rendered the vector content, fills, outlines and plain bitmaps of the
test files; text and transformed bitmaps are known not to show there.

## How to confirm an item on a file

1. Draw one object in the target design, save as `.cdr` (and once with
   "compressed" off, if the version offers it). Note the version.
2. `cargo run -p tracedraw-cli -- inspect file.cdr` dumps the chunk tree.
3. Compare the parsed values with what the target design shows (position
   in mm, size, colour values) and move the row to "confirmed (file)" with
   the file name.
