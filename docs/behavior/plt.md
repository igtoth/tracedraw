# HPGL plotter files (PLT)

Import (File > Open, File > Import) and export (File > Export) of
`.plt`, `.hpgl` and `.hgl` files.

## Units and orientation

1 plotter unit = 0.025 mm (40 per mm), origin at the bottom left, Y up,
which is the document's own orientation, so coordinates are scaled only.

## Reader

| Command | Behaviour |
|---|---|
| `IN`, `DF` | pen up, absolute mode, position (0, 0) |
| `SP n` | pen n; pens 1 to 8 default to black, red, green, yellow, blue, magenta, cyan, orange |
| `PW w [, n]` | pen width in mm for pen n (all pens without n); default 0.35 mm |
| `PC n, r, g, b` | pen colour |
| `PA`, `PR` | absolute or relative coordinates (also accept points) |
| `PU x,y,...`, `PD x,y,...` | pen up or down, then moves; every pen-down run becomes one open curve with the pen's outline and no fill |
| `CI r` | circle of radius r around the pen |
| `AA x,y,a`, `AR dx,dy,a` | arc of sweep a degrees around a centre, appended to the current stroke |
| `EA`, `ER` | rectangle outline (absolute or relative corner) |
| `RA`, `RR` | filled rectangle |
| `PM 0..2`, `EP`, `FP` | polygon mode; `EP` draws the edge, `FP` fills with the pen colour |
| `IP`, `SC` | scaling points and user units (uniform scaling; non-uniform is ignored with a warning) |
| `LB ...` ETX | labels are skipped (no text in the plotter font) |
| anything else | ignored |

Escape sequences (PJL) are skipped to the end of their line. Limits:
2,000,000 commands and 4,000,000 points. The page size is the drawing's
extent plus 10 mm (50 mm minimum).

## Writer

Every visible, printable layer of the page is flattened (0.02 mm
tolerance) into pen strokes: `IN;IP0,0,w,h;` then `PCn,r,g,b;PWw,n;`
per pen, one pen per distinct outline colour and width in order of first
use (more than 8 share pen 8), then `SPn;PUx,y;PDx,y,...;` per subpath.
Filled objects without an outline are drawn with their fill colour as a
0.25 mm pen; other fills are drawn as black outlines; bitmaps are
skipped. Text is written as the outlines of its glyphs.
