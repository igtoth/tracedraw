# Copying colors and attributes with the eyedroppers

The eyedropper tools take a color, or a whole set of attributes, from one
object and give it to others. Both work in two steps, shown on the property
bar: **select** (take a sample) and **apply** (paint with it). After a
sample, the tool switches to apply; click the select button to sample again.
<kbd>Esc</kbd> drops the sample and goes back to the [[tool.pick]] tool.

The pointer is a pipette while the tool samples and a paint bucket while it
applies. Pictures give their color even inside a group or a frame.

## Color Eyedropper {#color-eyedropper}

1. Choose the [[tool.color_eyedropper]] tool.
2. Click an object to take its color: the fill color of a uniform fill, the
   first color of a gradient, or the outline color. On a bitmap, it takes the
   color of the pixels under the pointer, averaged over the sample size set on
   the property bar (1 by 1, 2 by 2 or 5 by 5 pixels). <kbd>Shift</kbd>+click
   takes the outline color first.
3. Click objects to fill them with that color; <kbd>Shift</kbd>+click to color
   their outline (an object without an outline gets a thin one). Clicking
   inside an empty closed shape fills it.

The property bar shows the sampled color.

While you move the pointer to take a color, a small box beside it shows the
color under it: a swatch, the color's name when it is a palette color or a
spot ink, its values in the drawing's color mode (CMYK or RGB) and its hex
value. The eyedropper button of a color picker shows the same box.

## Attributes Eyedropper {#attributes-eyedropper}

The [[tool.attributes_eyedropper]] tool copies several properties at once
from one object to others.

1. Choose the [[tool.attributes_eyedropper]] tool.
2. On the property bar, choose what to copy:
   - **Properties**: outline, fill and text (the font, size and style; the
     text itself is kept). All on at first.
   - **Transformations**: size, rotation and position. Off at first.
   - **Effects**: transparency, drop shadow and live effects. Off at first.
3. Click the object to copy from.
4. Click each object to copy to. <kbd>Shift</kbd>+click samples another
   object instead.

Each click is one undo step.
