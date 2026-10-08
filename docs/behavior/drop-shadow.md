# Drop shadow

| Control | Default | Range |
|---|---|---|
| Offset | 5 mm, -5 mm | any |
| Opacity | 50% | 0..100 |
| Feathering | 15% | 0..100 |
| Feather direction | Average | Inside, Middle, Outside, Average |
| Colour | black | any |
| Merge mode | Multiply | |
| Perspective presets | flat | flat, perspective top-left and others with fade and stretch |

Feathering radius in mm `= feather/100 * max(w, h) * 0.2`, rendered as
three box blurs (approximating a Gaussian). The shadow is the object's
silhouette filled with the colour, blurred, drawn beneath the object.
