# Transparency

| Control | Default | Range |
|---|---|---|
| Type | Uniform | None, Uniform, Gradient (linear, radial, conical, square), Pattern, Texture |
| Transparency | 50% | 0..100 |
| Merge mode | Normal | Normal, Add, Subtract, Difference, Multiply, Divide, If lighter, If darker, Texturize, Hue, Saturation, Colour, Invert, And, Or, Xor, Red, Green, Blue |
| Apply to | Fill and outline | fill, outline, both |
| Freeze | off | |

Gradient transparency uses the same geometry as gradient fills with
grayscale stops where white = opaque and black = transparent. The
object is rendered to a scratch layer, multiplied by the transparency
mask, then composited with the merge mode.
