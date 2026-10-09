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
The document stores the profile names used for its colours; export
embeds the output intent for PDF/X (open).

## Checks

- Given CMYK (0, 100, 100, 0), then naive RGB is (255, 0, 0).
- Given a round trip RGB > CMYK > RGB of any pure colour, then the
  error is under 2/255 per channel.
