# Bitmap effects

The Effects menu's bitmap groups apply effects to the selected bitmaps.
As in the target design, effects are kept apart from the pixels: a
bitmap with effects remembers its original pixels and the list of
effects, so any of them can be edited, hidden, reordered or removed
later, and removing the last one gives the original back.

## How a bitmap keeps its effects

- A bitmap object's `png` always holds what it shows: the result of its
  visible effects. Its optional `fx` field keeps the original pixels,
  their place and size, and the effects in order (the first applies
  first). Rendering, exports and the `.cdr` writer use `png`, so they
  need to know nothing about effects.
- Each effect is its name and its settings by name: numbers, colours
  as `0xRRGGBB`, lists as the index of the chosen item, check boxes as 0
  or 1. Missing settings take the effect's defaults, and every setting
  is clamped to its range, so a file can never break an effect.
- The native `.tdraw` file stores the list; a bitmap without effects
  writes no `fx` field, and older files load unchanged.
- Changes that rework the pixels themselves (colour modes, Resample,
  Manually Inflate Bitmap, Straighten Image, Bitmap Color Mask, Update
  From Link, Effects > Flatten Effects) keep what the bitmap shows and
  drop the list.

## Effects menu

- Effects > Adjust, Transform and Correction, then the special effect
  groups (3D Effects, Art Strokes, Blur, Camera, Color Transform,
  Contour, Creative, Custom, Distort, Noise, Sharpen, Texture), in the
  target design's order. Items with settings end in "...".
- An item without settings (Auto Adjust, Desaturate, Invert Colors)
  adds the effect to every selected bitmap at once. The others open the
  effect's dialog; OK adds it to every selected bitmap. Either way the
  whole selection changes in one undo step ("Add Effect").
- Shortcuts with a bitmap selected: Ctrl+B Brightness/Contrast/Intensity,
  Ctrl+Shift+B Color Balance, Ctrl+Shift+U Hue/Saturation/Lightness.

## Effect dialogs

- Before and after previews of the first selected bitmap side by side:
  a copy at most 240 pixels across, worked out again only when a setting
  changes. Settings measured in pixels (radius, size, width, height,
  distance, period, amplitude, spacing and the like) are scaled with the
  copy, never below their minimum, so the preview looks like the result.
- Settings: sliders for numbers, a dial and a number for directions
  (degrees counter-clockwise from the right), lists, check boxes and
  colour buttons. Reset puts every setting back to its default.
- Editing an effect from the FX section previews from what the effects
  before it made, and the after view also runs the visible effects after
  it; OK changes that effect in place ("Edit Effect").
- Colours of the image (Target Color Balance's samples, Replace
  Colors' old colour) have an eyedropper beside their colour button: turn
  it on and click either preview to take the colour under the pointer.
- Contrast Enhancement shows the image's histogram with the input
  clipping markers below it (drag the black or white triangle; the
  clipped ends are shaded), the output range as a black to white bar with
  its two markers, eyedroppers that take the input black and white
  levels from the preview's brightness, and Auto adjust (the levels that
  cut 0.5 % at either end). The Channel list picks the channel the
  levels apply to and the histogram shows (Master: all, brightness). The
  numbers stay below for exact values.
- Tone Curve has a Channel list (Master, Red, Green, Blue) and a Style
  list (Curve, Straight, Freehand, Gamma), and the curve editor with the
  image's brightness histogram behind the grid. Curve and Straight: drag
  on empty space to add a node, drag a node to move it, double-click it
  or drag it off the grid to remove it; the eyedropper adds a node at the
  level of the pixel clicked in the preview (its brightness for Master,
  its channel otherwise). Freehand: drag to draw the curve; Smooth
  averages it over 9 levels. Gamma: drag the middle handle up or down.
  Invert flips the curve, Reset Active Channel resets the channel shown,
  Reset all of them, Auto Balance Tone stretches each of red, green and
  blue to the full range clipping 0.5 % at either end, Display all
  channels draws the other curves thin behind, and Load... and Save keep
  every channel's curve in a JSON file of the settings.

## Image Adjustments

Effects > Adjust > Image Adjustments opens a larger dialog instead of
the plain one:

- A 600 x 420 preview of a copy at most 720 pixels across. Tools above
  it: Rotate left and Rotate right (the view, a quarter turn), Pan (drag;
  the default tool), Zoom in and Zoom out (click a point to zoom by 1.5
  about it), Zoom to fit, 100% (one bitmap pixel per screen point); the
  wheel zooms about the pointer with any tool. Preview modes: Full
  preview, Before and after full preview (side by side, same view) and
  Before and after split preview (the original left of a dashed divider,
  the result right; drag the divider).
- Auto adjust sets the black and white points to the levels that cut
  0.5 % of the opaque pixels at either end. Select white point and Select
  black point turn the preview into a picker: a click sets that point
  to the brightness of the pixel clicked (kept at least one level apart).
- Sliders in the target design's groups: Temperature, Tint,
  Saturation; Brightness, Contrast; Highlights, Shadows, Midtones. The
  histogram below shows the result's brightness (square-root scaled).
- Undo and Redo step through the corrections made in the dialog (a step
  is recorded when a slider is let go, a pick or a button); Reset to
  original clears every correction. Create snapshot keeps the current
  settings as a numbered thumbnail below the preview; a click on one
  brings its settings back, its cross deletes it.
- OK adds one Image Adjustments effect (or changes the one being
  edited) with the settings, which the FX section can edit later.

## Properties docker: FX

A bitmap's Properties show an FX section listing its effects, first
applied at the top. Each row has a check box to show or hide the effect,
its name, Edit (opens its dialog), Move up and Move down (the reference
editor drags rows instead) and Delete. Add effect opens the effect groups
as a menu. Every change is one undo step and renders the list again from
the original pixels.

## Auto inflate

Document Options > General > Auto inflate bitmaps for effects, also
Bitmaps > Inflate Bitmap > Auto Inflate Bitmap (a check mark), is on by
default. When on, the visible effects that spread past the edges grow the
original first by their reach (summed, at most the larger side), with a
transparent margin, and the bitmap's place grows by the same amount in
millimetres, so a blur is not cut off at the edges. Reach: Gaussian Blur
`2 x Radius`, Low Pass `3 x Radius`, Motion Blur `Distance`, Wind
`Strength / 2` pixels. The switch is stored in the document and applies
to effects rendered from then on.

## Effects

Settings are listed in the dialog's order with their defaults (lists show
the default in bold). Pixel values are 0 to 255 per channel, `L` is the
luminance `0.299 R + 0.587 G + 0.114 B`, and every effect keeps
transparency unless it says otherwise. Blurs, resampling and mixing work
on premultiplied alpha, so colours do not darken where they spread into
transparent areas.

### Adjust

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Auto Adjust | none | Each channel stretched between the levels that cut 0.5 % of the opaque pixels at either end. |
| Image Adjustments | Black point 0 (0 to 254); White point 255 (1 to 255); Temperature 0 (-100 to 100); Tint 0 (-100 to 100); Saturation 0 (-100 to 100); Brightness 0 (-100 to 100); Contrast 0 (-100 to 100); Highlights 0 (-100 to 100); Shadows 0 (-100 to 100); Midtones 0 (-100 to 100) | In this order (settings / 100, values 0 to 1): black and white points map to 0 and 1; temperature `t` scales red by `1 - 0.25 t` and blue by `1 + 0.25 t`; tint `n` scales green by `1 + 0.2 n`, red and blue by `1 - 0.1 n`; saturation moves each channel away from the luminance by `1 + s`; brightness is the gamma `v^(2^-b)`; contrast scales about 0.5 by `1 + c`; then `0.35 (h smoothstep(0.5, 1, L) + s (1 - smoothstep(0, 0.5, L)) + m (1 - abs(2 L - 1)))` is added for highlights, shadows and midtones. |
| Contrast Enhancement | Channel: **Master**, Red, Green, Blue; Input low 0 (0 to 254); Input high 255 (1 to 255); Output low 0 (0 to 255); Output high 255 (0 to 255); Gamma 1 (0.1 to 10) | Levels: `t = clamp((v - in_low) / (in_high - in_low))^(1 / gamma)`, result `out_low + (out_high - out_low) t`, on every channel (Master) or the channel chosen. |
| Local Equalization | Width 20 (2 to 255); Height 20 (2 to 255); Lock width and height (on) | Histogram equalization of the brightness in tiles of Width x Height pixels (Width x Width when locked), each level clipped at 1/64 of the tile's pixels, blended bilinearly between tile centres; colours scale with their brightness so hues stay. |
| Target Color Balance | Channel: **Master**, Red, Green, Blue; Always adjust all channels (off); Shadow sample (#000000); Shadow target (#000000); Midtone sample (#808080); Midtone target (#808080); Highlight sample (#FFFFFF); Highlight target (#FFFFFF) | Per channel, straight segments through (0, 0), each sample to its target, and (255, 255); with Red, Green or Blue chosen only that channel changes, unless Always adjust all channels. |
| Tone Curve | curves (see below) | A master curve, then red, green and blue curves, each in its style: Curve (smooth monotone cubic through the nodes), Straight, Freehand (drawn, kept as straight segments every 4 levels) or Gamma (`255 (x / 255)^(1 / gamma)`). |
| Brightness/Contrast/Intensity | Brightness 0 (-100 to 100); Contrast 0 (-100 to 100); Intensity 0 (-100 to 100) | On values 0 to 1 with the settings / 100: `x = v + b / 2`, then `(x - 0.5)(1 + c) + 0.5`, then `x (1 + i / 2)`. |
| Color Balance | Shadows (on); Midtones (on); Highlights (on); Preserve luminance (on); Cyan / Red 0 (-100 to 100); Magenta / Green 0 (-100 to 100); Yellow / Blue 0 (-100 to 100) | Each axis adds `0.3 d w` to red, green or blue; `w` sums the checked ranges' weights (shadows `1 - smoothstep(0, 0.5, L)`, midtones `1 - abs(2 L - 1)`, highlights `smoothstep(0.5, 1, L)`). Preserve luminance shifts the three channels back to the old luminance. |
| Gamma | Gamma 1 (0.1 to 10) | `v^(1 / gamma)`. |
| Hue/Saturation/Lightness | Channel: **Master**, Red, Yellow, Green, Cyan, Blue, Magenta, Grayscale; Hue 0 (-180 to 180); Saturation 0 (-100 to 100); Lightness 0 (-100 to 100) | In HSL: hue plus `h`, saturation times `1 + s`, lightness toward white (`l > 0`) or black. A colour channel weights each pixel by its hue: full within 30° of red 0°, yellow 60°, green 120°, cyan 180°, blue 240° or magenta 300°, none past 60°, times `min(4 S, 1)`; Grayscale weights by `1 - 4 S`. |
| Selective Color | Color spectrum: **Reds**, Yellows, Greens, Cyans, Blues, Magentas, Grays; Method: **Relative**, Absolute; Cyan 0 (-100 to 100); Magenta 0 (-100 to 100); Yellow 0 (-100 to 100); Black 0 (-100 to 100) | The chosen colour range (weighted as in Hue/Saturation/Lightness; Grays by `1 - 4 S`) changes its CMYK percentages: Relative adds `d` times the ink's amount, Absolute adds `d`. |
| Replace Colors | Old color (#FF0000); New color (#0000FF); Ignore grayscale (on); Single destination color (off); Range 30 (1 to 100); Hue 0 (-180 to 180); Saturation 0 (-100 to 100); Lightness 0 (-100 to 100) | Pixels near the old colour (hue distance, plus saturation and lightness unless Ignore grayscale) move toward the new colour: shifted by the difference, or set to it with Single destination color. The weight falls linearly to zero at Range; the new colour is adjusted by Hue, Saturation and Lightness first. |
| Desaturate | none | Luminance `0.299 R + 0.587 G + 0.114 B` on all three channels. |
| Channel Mixer | Monochrome (off); Red from red 100 (-200 to 200); Red from green 0 (-200 to 200); Red from blue 0 (-200 to 200); Green from red 0 (-200 to 200); Green from green 100 (-200 to 200); Green from blue 0 (-200 to 200); Blue from red 0 (-200 to 200); Blue from green 0 (-200 to 200); Blue from blue 100 (-200 to 200) | Each output channel is the sum of the input channels times their percentages; Monochrome uses the red row for all three. |

### Transform

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Deinterlace | Remove: **Even lines**, Odd lines; Replace with: Duplication, **Interpolation** | Even or odd rows are replaced by the row above (Duplication) or the average of the rows above and below (Interpolation). |
| Invert Colors | none | `255 - v` per channel. |
| Posterize | Level 4 (2 to 32) | Each channel rounded to Level evenly spaced values. |

### Correction

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Dust and Scratch | Radius 2 (1 to 20); Threshold 32 (0 to 255) | Pixels that differ from the median of their Radius neighbourhood by more than Threshold take the median. |

### 3D Effects

| Effect | Settings (default; range) | What it does |
|---|---|---|
| 3D Rotate | Vertical 15 (-75 to 75); Horizontal 15 (-75 to 75); Best fit (on) | The image as a plane turned about x by Vertical and about y by Horizontal, seen in perspective with a focal length of twice the larger side; Best fit scales it to stay inside the bitmap. |
| Cylinder | Cylinder mode: **Horizontal**, Vertical; Percentage 50 (-100 to 100) | Columns (or rows) remapped as if wrapped on a cylinder: positive percentages stretch the middle (`t + k (asin t / (pi / 2) - t)`), negative ones squeeze it. |
| Emboss | Depth 2 (1 to 20); Level 100 (1 to 500); Direction 45°; Emboss color: Original color, **Gray**, Black, Other; Other color (#C0C0C0) | The brightness difference across Depth pixels along Direction, times Level, added to the base colour: the original colour, gray (128), black (40) or the other colour. |
| Page Curl | Corner: Top left, Top right, Bottom left, **Bottom right**; Direction: **Vertical**, Horizontal; Curl: **Opaque**, Transparent; Curl color (#F0F0F0); Background color (#FFFFFF); Width 50 (1 to 100); Height 50 (1 to 100) | A corner folds over a box of Width x Height percent: the uncovered part shows the background colour, the curl is shaded by its roundness, in the curl colour (opaque) or over the image (transparent). |
| Pinch/Punch | Amount 50 (-100 to 100); Center x 50 (0 to 100); Center y 50 (0 to 100) | Inside the largest circle about the centre, radius `r` (0 to 1) moves to `r^(1 - 0.6 k)`: positive amounts pinch, negative ones punch. |
| Sphere | Amount 50 (-100 to 100); Center x 50 (0 to 100); Center y 50 (0 to 100) | Inside the largest circle about the centre, `r + k (r^2 - r)` for positive amounts (the middle bulges), `r + abs(k) (sqrt(r) - r)` for negative ones. |

### Art Strokes

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Charcoal | Size 3 (1 to 10); Edge 5 (1 to 10) | Gray charcoal from the image blurred by Size, darker along its edges (Edge), with streaked grain. |
| Conté Crayon | Pressure 50 (1 to 100); Texture 50 (1 to 100); Crayon color: **Black**, Sepia, Sanguine, Gray; Paper color (#FFFFFF) | From the paper colour to the crayon colour (black, sepia, sanguine or gray) by darkness times Pressure, broken up by a Texture grain. |
| Crayon | Size 5 (1 to 20); Outline 30 (0 to 100) | Softened colours on a waxy paper grain, edges outlined by Outline. |
| Cubist | Size 10 (2 to 20); Brightness 50 (0 to 100); Paper color (#FFFFFF) | Squares of about Size pixels at jittered places, each in the colour under its centre shifted by Brightness, over the paper colour. |
| Impressionist | Style: **Strokes**, Dabs; Size 5 (1 to 20); Coloration 20 (0 to 100); Brightness 50 (0 to 100) | Elliptical strokes or round dabs of Size in the colour under them, jittered by Coloration and shifted by Brightness. |
| Palette Knife | Blade size 6 (1 to 20); Soft edge 50 (0 to 100); Angle 45° | Paint dragged Blade size along Angle (the darkest colour on the stroke, softened toward its average by Soft edge), held in short blocks. |
| Pastels | Pastel type: **Soft pastel**, Oil pastel; Stroke size 5 (1 to 20); Hue variation 20 (0 to 100) | The image blurred by Stroke size with streaks of paper showing through (less for oil pastel), hues jittered by Hue variation. |
| Pen & Ink | Style: **Crosshatch**, Stippling; Density 50 (1 to 100); Ink pools 50 (1 to 100) | Black ink on white: up to three hatching directions as the image darkens (Crosshatch) or random dots (Stippling), closer with Density; Ink pools fill the darkest parts. |
| Pointillist | Size 5 (1 to 30); Brightness 50 (0 to 100) | Round dots of Size in the colour under them, shifted by Brightness, on white. |
| Scraperboard | Paint: **Color**, Black and white; Density 50 (1 to 100); Size 5 (1 to 20) | A black board scraped where the image is light (Density, Size), showing the colour or white. |
| Sketch Pad | Pencil type: **Graphite**, Colored pencil; Style 50 (1 to 100); Pressure 50 (1 to 100); Outline 50 (0 to 100) | Pencil shading along a hatching angle set by Style, darker with Pressure, with Outline edges; graphite gray or coloured pencil. |
| Watercolor | Brush size 3 (1 to 10); Granulation 50 (1 to 100); Water 50 (1 to 100); Bleed 50 (1 to 100); Brightness 50 (0 to 100) | Colours washed by a median of Brush size and a blur (Bleed), lightened by Water with darker rims, Granulation noise and Brightness. |
| Felt Marker | Variation: **Default**, Order, Random; Size 5 (1 to 20); Color variation 20 (0 to 100) | Rectangular marker strokes of Size that multiply where they overlap; Variation sets their angle (fixed, level or random) and Color variation jitters their colours. |
| Paper Grain | Paint: **Color**, Black and white; Brush pressure 50 (1 to 100) | The image on paper with a wavy grain, stronger with Brush pressure, in colour or black and white. |

### Blur

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Directional Smooth | Percentage 50 (1 to 100) | Each pixel moves toward the average of the neighbours along the direction with the least change. |
| Gaussian Blur | Radius 1 (0.1 to 250) | A Gaussian with sigma Radius / 2 pixels (the radius is about two sigma). Reach: `2 x Radius`. |
| Smooth Jagged Edges | Width 1 (1 to 10); Height 1 (1 to 10) | Each pixel moves toward the average of its Width x Height neighbourhood, more where it differs from it. |
| Low Pass | Percentage 50 (1 to 100); Radius 3 (1 to 100) | A Gaussian with sigma Radius mixed in by Percentage. Reach: `3 x Radius`. |
| Motion Blur | Distance 10 (1 to 200); Direction 0° | The average over Distance pixels along Direction (at most 96 samples, spaced out for longer streaks). Reach: Distance. |
| Radial Blur | Amount 10 (1 to 100); Center x 50 (0 to 100); Center y 50 (0 to 100) | Averages along arcs about the centre, longer farther out (Amount). |
| Smooth | Percentage 50 (1 to 100) | A 3 x 3 average mixed in by Percentage. |
| Soften | Percentage 50 (1 to 100) | A Gaussian with sigma 0.7 mixed in by Percentage. |
| Zoom | Amount 20 (1 to 100); Center x 50 (0 to 100); Center y 50 (0 to 100) | Averages along the rays to the centre (Amount). |

### Camera

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Colorize | Hue 30 (0 to 360); Saturation 50 (0 to 100) | Every pixel takes the hue and saturation, keeping its luminance as the lightness. |
| Diffuse | Level 20 (1 to 100) | Pixels swapped with neighbours up to `1 + 4 level / 100` pixels away, then softened. |
| Photo Filter | Filter color (#EC8A00); Density 25 (1 to 100); Preserve luminosity (on) | `c (1 - d) + 2 c f d` per channel for filter colour `f` and density `d`; Preserve luminosity scales back to the old luminance. |
| Sepia Toning | Level 50 (0 to 100) | The gray `L` moves toward `(1.07 L + 20 t, 0.87 L + 8 t, 0.6 L)` by Level `t`. |
| Vintage Photo | Era: 1839 (daguerreotype), **1876 (albumen print)**, 1925 (sepia), 1945 (black and white), 1955 (cross-process), 1960 (faded print), 1965 (warm color); Intensity 70 (0 to 100) | Each era has its tint, contrast, fading, grain and vignette (1839 the strongest grain and vignette, the 1960s warm and faded), mixed in by Intensity. |

### Color Transform

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Bit Planes | Apply to all planes (on); Red 0 (0 to 7); Green 0 (0 to 7); Blue 0 (0 to 7) | Each channel shows one of its bits (0 the highest) as 0 or 255; Apply to all planes uses the red setting for all three. |
| Halftone | Max. dot radius 4 (2 to 20); Cyan angle 105°; Magenta angle 75°; Yellow angle 90°; Black angle 45° | Cyan, magenta, yellow and black screens of round dots in cells of twice Max. dot radius at their angles, printed over one another; a dot's area follows its ink. |
| Psychedelic | Level 128 (0 to 255) | Each channel shifted by Level (times 1, 2 and 3 for red, green and blue) and folded back. |
| Solarize | Level 128 (0 to 255) | Levels at or above `255 - Level` are inverted. |

### Contour

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Edge Detect | Background: **White**, Black, Other; Other color (#808080); Sensitivity 5 (1 to 10) | Sobel edges of the brightness times Sensitivity / 5: dark lines on white or the other colour, light lines on black. |
| Find Edges | Edge type: **Soft**, Solid; Level 50 (0 to 100) | Sobel edges in the pixels' own colours on white; Solid cuts them at 0.3. |
| Trace Contour | Level 128 (0 to 255); Edge type: **Lower**, Upper | Pixels on the border of the area below (Lower) or above (Upper) Level keep their colour; everything else turns white. |

### Creative

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Crystallize | Size 10 (3 to 100) | Voronoi cells about Size pixels apart, each in the colour at its seed. |
| Fabric | Style: **Needlepoint**, Rug hooking, Quilt, Strings, Ribbons, Tissue collage; Size 20 (1 to 100); Completion 100 (1 to 100); Brightness 50 (0 to 100); Rotation 0° | Cells of Size in a needlepoint, rug hooking, quilt, string, ribbon or tissue texture, turned by Rotation; Completion fills that share of them; Brightness. |
| Frame | Frame: **Plain**, Rounded, Torn edge, Soft edge, Brush stroke; Color (#FFFFFF); Opacity 100 (0 to 100); Blur/Feather 0 (0 to 100); Horizontal 100 (1 to 200); Vertical 100 (1 to 200); Rotation 0°; Flip horizontally (off); Flip vertically (off) | A border (8 % of the shorter side) in the frame colour, plain, rounded, torn, soft or brushed, scaled by Horizontal and Vertical, turned and flipped, with Opacity and Blur/Feather. |
| Glass Tiles | Width 10 (1 to 100); Height 10 (1 to 100) | Blocks of Width x Height pixels, each magnifying its middle (0.6). |
| Mosaic | Size 10 (2 to 100); Background color (#FFFFFF); Vignette (off) | Voronoi tiles in the colour at their seeds with grout in the background colour; Vignette fades toward the edges. |
| Scatter | Horizontal 5 (0 to 100); Vertical 5 (0 to 100) | Each pixel is taken from a random place up to Horizontal and Vertical pixels away. |
| Tinted Glass | Tint (#3C3C3C); Percentage 50 (0 to 100); Blur 2 (0 to 100) | A blur of Blur / 4 tinted toward the tint colour by Percentage. |
| Stained Glass | Size 20 (3 to 100); Light intensity 3 (0 to 10); Solder width 3 (1 to 10); Solder color (#000000); 3D lighting (off) | Voronoi panes of Size with solder of Solder width in the solder colour, brightened by Light intensity; 3D lighting shades each pane from its edge. |
| Vignette | Shape: **Ellipse**, Circle, Rectangle, Square; Color: **Black**, White, Other; Other color (#808080); Offset 100 (0 to 200); Fade 50 (0 to 100) | Outside the shape the image fades to black, white or the other colour: clear inside `0.8 x Offset`, fully covered after Fade more. |
| Vortex | Style: **Average**, Large, Fine, Layered; Size 20 (2 to 100); Inner direction 30°; Outer direction 330° | A turn about the centre from Inner direction to Outer direction, rippled by Size; Layered mixes the original back in. |

### Custom

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Bump Map | Bump map: The image itself, Waves, Rings, **Bricks**, Noise, Zigzag; Depth 20 (1 to 100); Smoothness 30 (0 to 100); Light direction 135°; Brightness 50 (0 to 100) | A height map (the image itself, waves, rings, bricks, noise or a zigzag), smoothed and lit from Light direction at Depth; Brightness. |

### Distort

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Blocks | Width 10 (1 to 100); Height 10 (1 to 100); Max offset % 20 (1 to 100); Undefined areas: **Original image**, Inverse image, Black, White, Other; Other color (#808080) | Blocks of Width x Height move up to Max offset % of their size; the gaps show the original image, its inverse, black, white or the other colour. |
| Displace | Displacement map: **Waves**, Rings, Bricks, Noise, Zigzag; Horizontal 10 (0 to 100); Vertical 10 (0 to 100); Undefined areas: Wrap around, **Repeat edges** | Pixels move by a built-in map (waves, rings, bricks, noise or zigzag) up to 30 % of Horizontal and Vertical in pixels; past the edges they wrap around or repeat the edges. |
| Mesh Warp | Gridlines 4 (2 to 10); Warp: **Random**, Bulge, Pinch, Wave; Strength 30 (0 to 100) | A Gridlines x Gridlines mesh is warped: random node moves, a bulge, a pinch or a wave, by Strength. |
| Offset | Horizontal 50 (-100 to 100); Vertical 50 (-100 to 100); Undefined areas: **Wrap around**, Repeat edges, Color; Other color (#FFFFFF) | The image moves by Horizontal and Vertical percent of its size; the gap wraps around, repeats the edges or takes the colour. |
| Pixelate | Mode: **Square**, Rectangular, Radial; Width 10 (1 to 100); Height 10 (1 to 100); Opacity 100 (0 to 100) | Squares, rectangles, or rings and sectors about the centre, each in the colour at its centre, mixed in by Opacity. |
| Ripple | Period 20 (1 to 100); Amplitude 10 (1 to 100); Direction 0°; Perpendicular wave (off); Distort ripple (off) | A sine wave across Direction with a wavelength of twice Period and an amplitude of Amplitude / 4; options add a perpendicular wave and a distorted ripple. |
| Swirl | Direction: **Clockwise**, Counterclockwise; Whole rotations 0 (0 to 10); Additional degrees 90 (0 to 359) | Inside the largest circle about the centre, a turn of `turns x 360° x (1 - r)^2`, clockwise or counterclockwise. |
| Tile | Horizontal 2 (1 to 100); Vertical 2 (1 to 100); Overlap % 0 (0 to 100) | Horizontal x Vertical copies of the image; Overlap enlarges each copy. |
| Wet Paint | Percentage 30 (0 to 100); Wetness 50 (-100 to 100) | Dark colours run down (light ones for negative Wetness) up to `40 x Wetness x Percentage` pixels. |
| Whirlpool | Spacing 20 (1 to 100); Smear length 10 (1 to 100); Twist 50 (1 to 100); Streak detail 50 (1 to 100) | Colours smeared along a fractal flow field (Spacing, Twist) for Smear length, with Streak detail grain. |
| Wind | Strength 30 (1 to 100); Opacity 100 (0 to 100); Direction 0° | Light streaks blown from Direction up to Strength / 2 pixels, mixed in by Opacity. Reach: Strength / 2. |

### Noise

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Add Noise | Noise type: **Gaussian**, Spike, Uniform; Level 50 (0 to 100); Density 50 (0 to 100); Color mode: **Intensity**, Random, Single color; Color (#000000) | Gaussian, spike or uniform noise of Level on Density % of the pixels: the same on every channel (Intensity), per channel (Random), or toward the noise colour (Single color). |
| Maximum | Percentage 100 (1 to 100); Radius 1 (1 to 20) | The per-channel maximum over Radius, mixed in by Percentage. |
| Median | Radius 1 (1 to 20) | The per-channel median over Radius. |
| Minimum | Percentage 100 (1 to 100); Radius 1 (1 to 20) | The per-channel minimum over Radius, mixed in by Percentage. |
| Remove Moiré | Amount 50 (1 to 100); Optimize for: Speed, **Quality** | A median and Gaussian (Quality) or a box blur (Speed), mixed in by Amount. |
| Remove Noise | Auto (on); Threshold 20 (0 to 255) | Pixels farther than Threshold from the median of their 3 x 3 neighbourhood take the median; Auto uses 24. |

### Sharpen

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Adaptive Unsharp | Percentage 50 (1 to 100) | An unsharp mask (sigma 1) that is stronger where a pixel differs from its neighbourhood. |
| Directional Sharpen | Percentage 50 (1 to 100) | Sharpens across the strongest edge direction only, so flat areas gain no grain. |
| High Pass | Percentage 50 (1 to 100); Radius 3 (1 to 20) | `128 + v - blur(v)` with a Gaussian of sigma Radius, mixed in by Percentage. |
| Sharpen | Edge level 50 (1 to 100); Threshold 0 (0 to 255); Preserve colors (off) | An unsharp mask of sigma 0.8 and amount `2 x Edge level / 100` where the difference reaches Threshold; Preserve colors applies the luminance change to all three channels. |
| Unsharp Mask | Percentage 100 (1 to 500); Radius 1 (0.1 to 100); Threshold 0 (0 to 255) | `v + (v - blur(v)) x Percentage / 100` with a Gaussian of sigma Radius, where the difference reaches Threshold. |

### Texture

| Effect | Settings (default; range) | What it does |
|---|---|---|
| Cobblestone | Size 20 (1 to 100); Coarseness 50 (1 to 100); Spacing 10 (1 to 100); Light direction 135° | Voronoi stones of Size with joints of Spacing and a Coarseness grain, lit from Light direction over the image. |
| Wrinkles | Age 50 (1 to 100); Skin color (#8C7C6C) | Wrinkles (Age) in the skin colour, lit, darker where the image is dark. |
| Etching | Detail 50 (1 to 100); Depth 50 (1 to 100); Brightness 50 (0 to 100); Light direction 135°; Metal color (#B4A48C) | The image's relief in the metal colour, lit from Light direction; Detail, Depth, Brightness. |
| Plastic | Highlight 50 (1 to 100); Depth 20 (1 to 100); Smoothness 50 (1 to 100); Light direction 135°; Light color (#FFFFFF) | The image blurred by Smoothness and lit as a glossy surface, with highlights in the light colour. |
| Relief Sculpture | Detail 50 (1 to 100); Depth 50 (1 to 100); Smoothness 50 (1 to 100); Light direction 135°; Surface color (#C8C0B0) | The image's relief in the surface colour, lit from Light direction; Detail, Depth, Smoothness. |
| Stone | Detail 50 (1 to 100); Density 50 (1 to 100); Light direction 135° | A fractal stone texture (Detail, Density) lit from Light direction over the image. |

### Tone Curve settings

The curves are stored as settings too: for each channel `rgb`, `r`, `g`
and `b`, `<channel>n` points, `<channel><k>x` and `<channel><k>y` for
each point (0 to 255), `<channel>style` (0 Curve, 1 Straight, 2
Freehand, 3 Gamma), `<channel>gamma` (0.1 to 10) and `<channel>linear`
(1 for straight segments, which older settings have instead of a style).
A channel with fewer than two points is the identity. Each pixel goes
through the master curve, then its channel's curve.

## Not covered

- The effects that the reference suite offers only in its photo editor
  (Bevel, Glass, The Boss, Zig zag, Dabble, Tune blur, Bokeh blur, Lens
  flare, Lighting effects, Spot filter, Band pass, User defined, Shear,
  Tune noise, 3D stereo noise, Brick wall, Bubbles, Canvas, Plaster wall,
  Screen door, Underpainting) are not in this list.
- The algorithms follow the target design's descriptions and
  settings; results are close in look, not pixel-identical. 3D Rotate
  uses sliders where the target design also has a model to drag.
- Tone Curve presets are JSON files of the settings rather than the
  target design's own curve files, and Auto Balance Tone always clips
  0.5 % (no settings for its limits).
- Third-party plug-in filters are not supported.

## Checks

- Given any effect, when it runs with its defaults, its maximums or its
  minimums, then the image keeps its size and every name, setting and
  list item has a translation (`fx::tests::every_effect_runs_keeps_size_and_has_names`).
- Given Brightness/Contrast/Intensity, Color Balance, Gamma,
  Hue/Saturation/Lightness, Selective Color, Channel Mixer, Image
  Adjustments, Tone Curve or Contrast Enhancement with default
  settings, then no pixel changes by more than one level
  (`neutral_settings_leave_the_image_alone`).
- Given an image, then Invert Colors twice gives it back, Desaturate
  leaves grays, Posterize at 2 leaves 0 and 255, more brightness never
  darkens, Gaussian Blur lowers a checkerboard's variance and Unsharp
  Mask raises it, Median removes a lone speck, colour effects keep alpha,
  and a tone curve from (0, 255) to (255, 0) inverts
  (`effects_do_what_their_names_say`).
- Given a bitmap with a Gaussian Blur and Auto inflate on, then the
  result is larger than the original and placed around it; off, it keeps
  the size; a hidden effect leaves the original
  (`stacks_keep_the_original_and_inflate_for_spreading_effects`,
  `auto_inflate_is_a_document_switch`).
- Given a selected bitmap, when an effect is added, hidden and deleted,
  then each is one undo step, hiding shows the original and deleting
  the last effect drops the list
  (`effects_are_added_edited_and_removed_in_single_steps`).
- Given a red square on transparency, when Gaussian Blur, Motion Blur,
  Low Pass, Smooth Jagged Edges, Radial Blur or Zoom spreads it, then every
  visible pixel stays red (`blurs_spread_colour_not_black_into_transparency`).
- Given noise, then the fast Gaussian is within 3 levels on average of
  the exact one for sigma 0.8, 3 and 9, and the running-histogram
  median, minimum and maximum equal sorting for radii 1, 3 and 6
  (`fx::util::tests`).
- Given the Tone Curve's styles, then a missing style follows Linear,
  setting a style keeps Linear in step, Gamma 2 lifts 64 to 128, a
  freehand curve keeps a point every 4 levels and smoothing keeps its
  ends, Auto Balance Tone stretches 50..200 to 0..255, and Reset Active
  Channel leaves the other channels' settings
  (`tone_curve_styles_gamma_freehand_and_balance`,
  `resetting_a_channel_leaves_the_others`).
- Given the Image Adjustments, then undo and redo walk the recorded
  steps and a new change drops the undone ones, snapshots are numbered
  from 1, Auto adjust finds 60 and 200 on columns of grays from 60 to
  200 with two outliers, the pickers keep the points apart, and every
  quarter turn maps preview points back to the same pixel
  (`ui::lab_dialog::tests`).
- Given gray 64, then Contrast Enhancement with input high 128 doubles
  only red when Red is chosen and every channel for Master, and
  Target Color Balance from 64 to 128 changes only green when Green is
  chosen and all three with Always adjust all channels
  (`channel_lists_limit_levels_and_target_balance`); a preset file keeps
  only its numbers (`presets_keep_numbers_only`).
- Given a preview at a quarter of the size, then pixel settings are a
  quarter, never below their minimum, and other settings stay
  (`previews_scale_pixel_sizes_only`).
- Given a document with effects and the switch off, when it is saved and
  loaded, then it is equal, and a file without the fields loads with
  defaults (`bitmap_effects_round_trip_and_plain_bitmaps_stay_short`).
