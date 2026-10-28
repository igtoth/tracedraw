# Export

File > Export (Ctrl+E) and File > Export For > Web or Office open the
Export dialog. File > Export starts with the choices of Options > Global
> File Formats, which can also keep those of the last export; the Web and
Office presets start with their own (see `options.md`).

## Layout

- On the left, a 260 x 300 point preview of what will be written: the
  page, or the selection with Selected only, rendered to fit, on a
  checkerboard when the format keeps transparency and Transparent
  background is on, else on white.
- On the right: Format, then the format's own settings, then Selected
  only and All pages, then the estimated file size.

| Format | Settings |
|---|---|
| PNG, WebP, GIF | resolution (1 to 2400 dpi, presets 72, 96, 150, 300), transparent background; WebP also quality (the files are lossless; below 100 the colours are first rounded to fewer levels, WebP's near-lossless way: 1 low bit from 90, 2 from 75, 3 from 50, 4 below, so a lower quality makes a smaller file; #544, `export::tests::webp_quality_shrinks_the_file`) |
| JPEG | resolution, quality (1 to 100) |
| BMP, TIFF, PSD | resolution |
| PDF | standard (PDF 1.4, PDF/X-1a, PDF/X-3, PDF/X-4), bleed, output condition for PDF/X; the author, subject and keywords come from the drawing's properties (File > Publish to PDF has bookmarks and protection, see `publish-to-pdf.md`) |
| SVG, EPS, AI, DXF, HTML, EMF, WMF, PLT | none |
| PostScript (`.ps`), PRN (`.prn`) | none; every exported page in one file (All pages, else the current page), each page setting its size with `setpagedevice` and ending with `showpage` (Document Structuring Conventions 3.0); a `.prn` print file is the same PostScript, to send to a PostScript printer |

- Bitmap formats show Size: a width and a height and their unit (px,
  mm, cm or in, kept with the other choices; #617). In pixels the size
  is the page (or selection) size over 25.4 times the resolution,
  rounded up, and typing a width or a height sets the resolution that
  gives it, so a 32 x 32 pixel icon is typed rather than worked out. In
  a length it is the printed size: typing a width or a height sets the
  scale of the artwork (the resolution stays), from 1 % to 10000 % and
  never past what 2400 dpi can draw; a button with the scale ("200 %")
  shows when it is not 100 % and puts it back. Either way the other side
  follows the page's proportions.
- Under the size: what comes out, "1240 x 1754 pixels, 150 dpi, sRGB"
  ("sRGB + alpha" with a transparent background).
- PNG, JPEG and BMP files record the resolution of the dialog (PNG
  `pHYs`, the JFIF density, the BMP header's pixels per metre), so a
  scaled export opens at the printed size chosen; the scale multiplies
  the pixels, not that resolution (#617,
  `export::tests::bitmap_exports_record_their_resolution_and_scale`,
  `export::tests::the_export_size_is_typed_in_pixels_or_lengths`).
- File size: vector formats (SVG, EPS, PostScript, PDF, AI) are written in memory and
  measured; bitmap formats are estimated by encoding the preview the same
  way and scaling by the pixel count of the export (times the pages with
  All pages). It shows as "about 820 KB" or "about 3.4 MB".
- Text in PDF and AI files is text with its fonts embedded as subsets,
  as Publish to PDF writes it by default (see `publish-to-pdf.md`, Text
  and fonts). Vector formats write curves with their own segments:
  quadratic segments (TrueType letters, for example) become the cubic
  that draws the same curve in PDF, AI, EPS, PostScript and EMF, and
  stay quadratic in SVG; only DXF, WMF and PLT, which have no curves of
  that kind, approximate them with short lines.
- The preview and estimate are worked out again only when the drawing,
  the page, the selection or a setting changes.
- OK asks where to save; the file name is the drawing's with the format's
  extension.

## Presets

Above the format: Preset, a list of every saved preset, each with the
format it was saved for in brackets ("Web PNG (PNG)"), "(none)" first,
whole from the moment the dialog opens whatever format it opens at; "+"
to save the dialog's choices under a name (typed in a field that opens
below, OK or Enter; the same name replaces that preset of the format), a
pencil to rename the chosen preset (a name another preset of the format
has is refused) and "−" to delete it.

Choosing a preset of another format switches the dialog to that format.
Changing the format leaves the chosen preset unchosen ("(none)") while
the list stays whole (#612, the reporter's second request: the list
used to shrink to the chosen format's presets). Saving, renaming and
deleting act on the presets of the chosen format only, so a preset of
another format with the same name is kept. With a script filter chosen
none is chosen and "+" is greyed. A preset of a format this version does
not know shows by its name alone; its name must be free in every format
(`prefs::tests::export_presets_save_apply_rename_and_delete`,
`pick_drag_tests::the_export_dialog_lists_every_preset_and_a_format_change_unchooses`).

Choosing a preset
puts back its format, resolution, quality, transparent background,
selection only, all pages, the size's unit and scale and the PDF
choices (standard, bleed, output condition). Presets are kept in the preferences
(`formats.export_presets`; #547,
`prefs::tests::export_presets_save_apply_rename_and_delete`).

## WebP

Lossless (off at first) and Quality (1 to 100). Off, the file is lossy
WebP at that quality (libwebp, in the desktop builds), far smaller: a
picture of 450 KB lossless comes to about 50 KB at quality 10. On, it is
lossless, and below 100 the colours are first rounded to fewer levels
(near-lossless, #544). The browser build has no lossy encoder and always
writes the lossless way (#592, `export::tests::webp_quality_shrinks_the_file`).

## Starting choices

File > Export starts with the choices of the last export of the session
(Selected only off again, the file's title and tags empty), else those
of Options > Global > File Formats, which keep the last
export's choices for the next session while Remember the settings of the
last export is on (on; preferences from before 0.19.4 are brought up to
date once, #691; `options.md`, #617).

Each format also keeps how it was exported last: choosing a format in the
dialog brings back its resolution, quality, transparent background,
WebP lossless, all pages, size unit and scale and PDF choices (Selected
only and the preset stay as they are), from this session's exports and,
with Remember the settings of the last export on, from earlier ones
(`formats.export_by_format`). A format not exported yet keeps the
dialog's choices, and Export for Web and Office keep their own (#617,
`prefs::tests::each_export_format_comes_back_as_it_was_last_exported`).

## Checks

- `export::tests::the_export_preview_shows_the_page_and_estimates_the_size`
- `pick_drag_tests::the_export_dialog_starts_with_the_last_exports_choices`
- `export::tests::sizes_read_in_kilobytes_and_megabytes`
- `ui::dialogs::tests::a_pixel_size_sets_the_resolution_that_gives_it`
- `pick_drag_tests::the_export_dialog_lists_every_preset_and_a_format_change_unchooses`
- `export::tests::bitmap_exports_record_their_resolution_and_scale`
- `export::tests::the_export_size_is_typed_in_pixels_or_lengths`
- `prefs::tests::each_export_format_comes_back_as_it_was_last_exported`

## Format list and file properties

- The Format list shows each format as "EXT - Description (*.ext)": JPG,
  PDF, AI and PNG first, a separator, then every format in the order of
  its extension, then installed script filters.
- Under the format's settings: Selected only, All pages, and "Do not
  show filter dialog": when on, File > Export (Ctrl+E) goes straight to
  the file with the last choices (Options > Global > File Formats keeps
  them); Export for Web and for Office still show their dialog.
- PDF and AI files: Title (the drawing's title when empty) and Tags,
  written as the file's title and keywords.
- Checks: `ui::dialogs::tests::the_export_list_puts_the_common_formats_first`.

