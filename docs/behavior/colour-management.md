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

## Conversions without a loaded profile

CMYK uses a built-in model of process inks on coated stock
(`core/press.rs`), so CMYK colours look the way the target design shows
them with its default colour settings (a coated web offset CMYK space,
relative colorimetric, black point compensation): process black is the
warm dark grey (34, 31, 32) rather than (0, 0, 0), cyan is (0, 173, 239),
magenta (236, 0, 139), yellow (255, 241, 0).

- CMYK to RGB: a Yule-Nielsen modified Neugebauer model. Each ink's
  coverage goes through its dot gain curve (11 points), the 16 overprints
  of solid inks are mixed with the Demichel weights in a power-law space
  (exponent per screen channel), the result is linear sRGB and is clipped
  to the screen gamut. The constants were fitted to the colours the
  target design shows: its default palette within 2.5 Delta E (CIE76),
  any ink mixture up to 320% within about 1 on average and 2.2 for 99% of
  them.
- RGB to CMYK, as the target design's defaults do: pure black becomes
  K 100 only, neutral greys use black ink only (the grey with the same
  mean level), and other colours get a grey component replacement (black
  grows with the darkness of the colour) with C, M and Y solved by
  Levenberg-Marquardt so the colour shows as asked or as close as the
  press can print, within a 300% ink limit. Screen colours outside the
  press gamut (pure RGB red, green, blue) come back as the nearest
  printable colour, so the gamut alarm flags them.
- A colour already in the target model is returned unchanged.
- Gray: `y = 0.2126 r + 0.7152 g + 0.0722 b` in linear light.
- HSB, HSL: standard hexcone conversions.
- Lab: through sRGB linearisation and D65 XYZ, then CIE Lab; D50
  adaptation (Bradford) when a profile asks for it.

## With a CMS

`IccEngine` (core/color.rs) holds the working RGB profile, the CMYK
profile, the intent and the black point compensation flag, and builds the
transforms once: RGB and CMYK to the display (sRGB), RGB to CMYK and back.
`Color::to_rgb8`, `convert_to` and the gamut warning go through it when
it is installed; otherwise the built-in conversions apply. Profiles are loaded
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
  profile is loaded, else the press model; image pixels with the simple
  black generation `k = 1 - max(r, g, b)`), images written as
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

- `tracedraw_core` `press::tests::process_inks_show_as_in_the_reference_editor`:
  the target design's palette colours on screen within 5 levels.
- `press::tests::printable_colours_round_trip`: CMYK to RGB to CMYK to RGB
  within one level for ink totals up to 260%.
- `press::tests::black_and_greys_use_black_ink_only`,
  `press::tests::screen_colours_outside_the_press_gamut_come_out_close`,
  `press::tests::more_ink_never_shows_lighter`,
  `press::tests::bad_numbers_do_not_break_it`.
- `color::tests::screen_red_is_outside_the_press_gamut`,
  `color::tests::converting_to_the_same_model_keeps_the_colour`.
