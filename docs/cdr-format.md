# CDR format notes

What the reader assumes, and what has been confirmed on real files.
Status legend: **assumed** (from public reverse-engineering notes, not yet
checked), **confirmed** (checked against a named file), **wrong** (keep for
history).

## Containers

| Item | Status |
|---|---|
| `RIFF <size> CDR<v>`; `v` is a version char: `9` = 9, `A` = 10 ... `E` = X4 (14), `H` = X7 (17), `J` = 2017 (19) | assumed |
| X4+ files are ZIP with `content/riffData.cdr` holding the RIFF stream | assumed |
| `LIST cmpr`: first `cmpr` chunk = u32 uncompressed size + zlib stream of chunks | assumed |
| Chunk sizes are 2-byte aligned (4-byte in some newer versions) | assumed |

## Units and coordinates

| Item | Status |
|---|---|
| Coordinates are i32 in 1/254000 inch (0.0001 mm) | assumed |
| Page origin is the page centre; shapes carry their own transform (`trfd`) | assumed |
| `mcfg` holds page width/height; offset 0 (v7-8), 4 (v9-12), 12 (X3+) | assumed |

## Objects

| Item | Status |
|---|---|
| `LIST page` > `LIST layr` > `LIST obj ` > `loda` + `trfd` | assumed |
| `loda` header: len, numArgs, startArgs, startTypes, objType (u32 each) | assumed |
| arg types: `0x1e` geometry, `0x14` fill id, `0x0a` outline id | assumed |
| objType: `0x01` rect, `0x02` ellipse, `0x03` curve, `0x04` text, `0x05` bitmap, `0x14` polygon | assumed |
| Curve: count (u16, u32 from X6) + points (x,y) + one type byte per point; `0x00` move, `0x40` line, `0x80` control, `0xC0` curve end, bit `0x08` closes | assumed |
| `trfd`: args table, then 6 doubles (a c e b d f), translation in CDR units | assumed |
| `fild`: id, [flags X3+], type u16: 1 solid (colour), 2 fountain | assumed |
| `outl`: id, [flags X3+], lineType, caps, join, width, colour | assumed |
| Colour: model u16, palette u16, value u32; models 0x02 CMYK%, 0x03 CMYK255, 0x05 BGR, 0x09 gray | assumed |

## How to confirm an item

1. Draw one object in the target design, save as `.cdr` (and once with "compressed"
   off, if the version offers it). Note the version.
2. `cargo run -p tracedraw-cli -- inspect file.cdr` dumps the chunk tree.
3. Compare the parsed values with what the target design shows (position in mm,
   size, colour values) and update the table with the file name.
