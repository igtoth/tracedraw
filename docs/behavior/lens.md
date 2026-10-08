# Lens

A lens changes how objects beneath it appear, without changing them.

| Lens | Parameter | Default |
|---|---|---|
| Brighten | rate -100..100 | 50 |
| Colour add | colour, rate | white, 50 |
| Colour limit | colour, rate | black, 50 |
| Custom colour map | from, to, direction | black..white, direct |
| Fish eye | rate -1000..1000 | 100 |
| Heat map | palette rotation 0..100 | 0 |
| Invert | none | |
| Magnify | amount 1..100 | 2 |
| Tinted grayscale | colour | black |
| Transparency | rate 0..100, colour | 50, white |
| Wireframe | outline colour, fill colour | |

Shortcut: Alt+F3. Options: Frozen (bakes), Viewpoint, Remove face.

## Formulas (per pixel beneath the lens shape, colour `c` 0..1)

- Brighten: `c + rate/100 * (1 - c)` for positive, `c * (1 + rate/100)`
  for negative.
- Colour add: `min(1, c + rate/100 * k)`.
- Colour limit: `c * (1 - rate/100) + rate/100 * c * k`.
- Custom colour map: luminance `y` to `lerp(from, to, y)` (direct) or
  through hue rotation.
- Fish eye: radial remap `r' = r ^ (1 + rate/1000)` about the lens centre.
- Magnify: resample the scene at `1/amount` about the lens centre.
- Tinted grayscale: `y * tint`.
- Transparency: `c * (1 - rate/100) + k * rate/100`.
- Invert: `1 - c`.

Lenses render as a second pass: the scene beneath is rasterised, the
lens function is applied inside the shape's mask, then the result is
composited.

## Checks

- Given a white page and a 50% transparency lens coloured red, then
  pixels inside the lens are (255, 128, 128).
- Given an invert lens over black, then the pixels are white.
