# Supported file formats

## Saving {#save}

| Format | Notes |
|---|---|
| TraceDraw (`.tdraw`) | the native format; keeps everything, including effects, styles, comments and fonts |
| CDR (`.cdr`) | versions 7 to 27 from Save As; keeps pages and their sizes, layers, shapes, curves, groups and their names, text, bitmaps, fills, outlines and transparency, and the fonts with [[dialog.embed_fonts]]. From version 14, blends, contours, distortions, envelopes, perspectives, extrusions, bevels, block shadows and symmetries open again in TraceDraw as live effects; other programs see the objects the effects drew. Drop shadows are written as a picture of the shadow under the object |
| TraceDraw template (`.tdt`) | from [[menu.file.save_as_template]] |
| A package (a folder) | from [[menu.file.package]]: the drawing, its linked pictures, its fonts and a report; see [Packaging a drawing](package.md) |

## Opening and importing {#open}

| Format | What comes in |
|---|---|
| TraceDraw (`.tdraw`, `.tdt`) | everything |
| CDR (`.cdr`), from the oldest versions to the newest | pages, layers, shapes, curves, groups, text, bitmaps (CMYK pictures in the CMYK mode), fills including mesh fills, outlines, transparency, blends, embedded objects drawn as pictures (such as barcodes), embedded fonts, the color profile |
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
