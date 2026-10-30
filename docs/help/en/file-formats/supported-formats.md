# Supported file formats

## Saving {#save}

| Format | Notes |
|---|---|
| TraceDraw (`.tdraw`) | the native format; keeps everything, including effects, styles, comments and fonts |
| CDR (`.cdr`) | versions 7 to 27 from Save As; keeps pages, layers, shapes, curves, groups, text, bitmaps, fills, outlines and transparency; effects are written as plain objects |
| TraceDraw template (`.tdt`) | from [[menu.file.save_as_template]] |

## Opening and importing {#open}

| Format | What comes in |
|---|---|
| TraceDraw (`.tdraw`, `.tdt`) | everything |
| CDR (`.cdr`), from the oldest versions to the newest | pages, layers, shapes, curves, groups, text, bitmaps, fills, outlines, transparency, embedded fonts, the color profile |
| CMX | pages, layers, shapes, fills, outlines, images, clips, text |
| SVG, SVGZ | shapes, groups, clips, gradients, images, text |
| PDF, AI | paths, images, text, clips, shadings, patterns; password-protected files ask for the password |
| EPS, PS, PRN, older AI files | paths, images, text, clips, shadings; one page per printed page |
| DXF | lines, polylines, arcs, ellipses, splines, hatches, text, blocks |
| EMF, WMF | paths, pens, brushes, text, bitmaps, clips |
| PLT (HPGL) | plotter pen strokes |
| PSD, PSB | the layers as bitmaps, with opacity, visibility and masks |
| PNG, JPEG, BMP, GIF, WebP, TIFF | bitmaps |
| TXT, RTF, DOCX | paragraph text with its formatting |

## Exporting {#export}

| Format | Kind |
|---|---|
| SVG, HTML | vector, for the web |
| PDF, AI | vector, for print and sharing; PDF/X-1a, X-3 and X-4 |
| EPS, PostScript (`.ps`), PRN | vector, for print |
| DXF, PLT | vector, for CAD, plotters and cutters |
| EMF, WMF | vector, for office programs |
| PNG, JPEG, WebP, GIF, BMP, TIFF | bitmap |
| PSD | bitmap, one layer per layer of the drawing |

Scripts can add more formats to importing and exporting; see
[Scripts](../customize/scripts.md#filters).
