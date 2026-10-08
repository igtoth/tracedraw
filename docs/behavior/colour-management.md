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

A `ColorEngine` trait with `to_rgb(color, profile)`,
`to_cmyk(color, profile, intent)`, `proof(rgb, output_profile)`. The
lcms2 implementation builds a transform per (source, destination,
intent) and caches it. The document stores the profile names used for
its colours; export embeds the output intent for PDF/X.

## Checks

- Given CMYK (0, 100, 100, 0), then naive RGB is (255, 0, 0).
- Given a round trip RGB > CMYK > RGB of any pure colour, then the
  error is under 2/255 per channel.
