# Exporting

[[menu.file > menu.file.export]] (<kbd>Ctrl+E</kbd>) writes the drawing, a
page or the selection in another format. The Export dialog box shows a
preview on the left and the settings on the right.

![The Export dialog box](../../img/file-formats-export.png)

## To export {#export}

1. Choose [[menu.file > menu.file.export]].
2. Choose the **format**. The common ones (JPG, PDF, AI, PNG) are first.
3. Set the format's options (below).
4. Tick **Selected only** to export only the selection, or **All pages** to
   export every page.
5. Click [[dialog.ok]] and choose where to save. In the browser, the file is
   downloaded.

The dialog box estimates the size of the file. With **Do not show filter
dialog** ticked, the next export goes straight to the file with the same
choices.

## Format options {#options}

| Format | Options |
|---|---|
| PNG, WebP, GIF | resolution (or the size in pixels), transparent background; WebP also quality |
| JPEG | resolution, quality (1 to 100) |
| BMP, TIFF, PSD | resolution |
| PDF | standard (PDF 1.4, PDF/X-1a, PDF/X-3 or PDF/X-4), bleed, output condition for PDF/X |
| PDF, AI | title and tags |
| SVG, EPS, AI, DXF, HTML, EMF, WMF, PLT, PS, PRN | none |

For bitmap formats you can type the width or height in pixels; the
resolution follows. PostScript and PRN files hold every exported page.

The starting choices are set on the [[options.file_formats]] page of the
global options. [[menu.file.export_for]] has presets for the web and for
office documents; see [Exporting for the web](../web-graphics/export-for-web.md).
For PDF files with bookmarks and security, use
[Publishing to PDF](publish-pdf.md).
