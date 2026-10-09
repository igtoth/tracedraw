# Colour management

| Setting | Default |
|---|---|
| RGB working profile | sRGB IEC61966-2.1 |
| CMYK working profile | an open CMYK profile shipped with the app (no vendor profile) |
| Grayscale | Dot gain 20% |
| Rendering intent | Relative colorimetric |
| Black point compensation | on |
| Policy for embedded profiles | use embedded, warn on mismatch |
| Soft proof | off; profile = CMYK working profile |

## Conversions without a CMS (current implementation)

- RGB to CMYK: `k = 1 - max(r, g, b)`, `c = (1 - r - k) / (1 - k)`,
  same for `m`, `y`; black gives `k = 1`.
- CMYK to RGB: `r = (1 - c) * (1 - k)`.
- Gray: `y = 0.2126 r + 0.7152 g + 0.0722 b` in linear light.
- HSB, HSL: standard hexcone conversions.
- Lab: through sRGB linearisation and D65 XYZ, then CIE Lab; D50
  adaptation (Bradford) when a profile asks for it.

## With a CMS

`IccEngine` (core/color.rs) holds the working RGB profile, the CMYK
profile, the intent and the black point compensation flag, and builds the
transforms once: RGB and CMYK to the display (sRGB), RGB to CMYK and back.
`Color::to_rgb8`, `convert_to` and the gamut warning go through it when
it is installed; otherwise the naive formulas apply. Profiles are loaded
from `.icc`/`.icm` files in Tools > Colour Management (Load profile...),
remembered by path in the settings and installed again at start-up. The
CLI `tracedraw-cli icc <file>` prints a profile's class, spaces, version
and the sRGB values of its primaries, which is how a profile is checked.
The document stores the profile names used for its colours.

## PDF/X export

Export > PDF offers PDF 1.4, PDF/X-1a:2003, PDF/X-3:2003 and PDF/X-4
(`pdf::PdfOptions`):

- Every level writes an Info dictionary (Title, Creator, Producer,
  dates, Trapped False, GTS_PDFXVersion and, for X-1a, Conformance), a
  trailer ID, an XMP metadata stream with the pdfxid version, and an
  OutputIntent (`/S /GTS_PDFX`) whose identifier is the output condition
  typed in the dialog (default FOGRA39; "CGATS TR 001" when empty). The
  CMYK profile loaded in Colour Management is embedded as
  DestOutputProfile (`/N` from its colour space signature).
- X-1a: PDF 1.4, every colour converted to CMYK (ICC engine when a CMYK
  profile is loaded, else the naive formula), images written as
  DeviceCMYK, shadings in DeviceCMYK, no ExtGState alpha and no soft
  masks. Objects with opacity, transparency or lens effects, and
  bitmaps with transparent pixels, are flattened: everything drawn up to
  and including the object is rendered at the export resolution and the
  crop over the object's bounds replaces it as an opaque image.
- X-3: like X-1a but RGB colours stay RGB (the output intent gives them
  meaning).
- X-4: PDF 1.6, transparency kept.
- Bleed: MediaBox grows by the bleed on every side, TrimBox is the page,
  BleedBox the media box; content is shifted by the bleed.

`pdfinfo` reports "PDF subtype: PDF/X-1a:2003" for the X-1a output and
`qpdf --check` finds no errors; a sample X-1a file of the 2019 banner
rendered the same picture through the DeviceCMYK image.

## Checks

- Given CMYK (0, 100, 100, 0), then naive RGB is (255, 0, 0).
- Given a round trip RGB > CMYK > RGB of any pure colour, then the
  error is under 2/255 per channel.
