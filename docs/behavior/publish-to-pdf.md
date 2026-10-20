# Publish to PDF

File > Publish to PDF... (also on the standard toolbar) opens the PDF
Settings dialog for the active drawing, with the tabs General, Color,
Document, Objects, Prepress, Security and the issues tab, then "?", OK
and Cancel. OK writes the file named on General (in the browser the file
is downloaded). The dialog is `ui/publish_pdf.rs`; the file comes from
`tracedraw_io::pdf` and its protection from `tracedraw_io::pdf_crypt`.

## General

- File name: the drawing's folder and name with `.pdf` (else the last
  folder used), and Browse... to pick another.
- Export range: Current document (every page, the default), Documents
  (every page of every open drawing, one drawing after the other; only
  with more than one open; symbols are renumbered so each drawing's
  instances keep theirs), Current Page, Pages with a list such as
  `1-3, 5` (pages that do not exist are left out; a list that names no
  page is refused), and Selection (only with a selection).
- Page size, for Selection: Defined by document (the page with the
  selected objects only) or Defined by selected objects (a page the size
  of the selection's bounds, the default).
- PDF preset: Document Distribution, Web, Prepress (PDF/X-4), PDF/X-1a,
  PDF/X-3, then the presets saved with "+" (it asks a name; a built-in
  name is not taken; the same name replaces the preset). "-" deletes the
  chosen saved preset. A preset sets the compatibility, output
  condition, flattening resolution, bookmarks and bleed.
- Compatibility: PDF 1.4, PDF/X-1a:2003, PDF/X-3:2003 or PDF/X-4.

## Color

For PDF/X: Output condition (FOGRA39 by default) and the note about the
output intent's profile (the CMYK profile of Colour Management). A plain
PDF keeps the drawing's colours as they are.

## Objects

Flatten transparency at (300 dpi), for PDF/X-1a and X-3 only; greyed
for the others, which keep transparency live.

### Text and fonts

Text goes into the file as text: it can be searched, selected and
copied in a viewer. The choices (kept in the preferences):

| Choice | Default | What it does |
|---|---|---|
| Export all text as curves | off | Every text is written as filled outlines; the file has no fonts. The other choices are greyed. |
| Embed fonts in document | on | The fonts go into the file. Off, a font is only named (a simple TrueType font in the WinAnsi encoding with its widths) for the viewer to supply, and characters outside WinAnsi, ligatures and alternate glyphs are drawn as curves. |
| Embed base 14 fonts | off | Text in the Helvetica, Times and Courier families names the standard font of its style (Helvetica-Bold, Times-Italic and so on) in WinAnsi with the widths of the font it was set in; on, those fonts are embedded like the others. Characters outside WinAnsi come from the embedded font. |
| Subset fonts, when less than (80 %) | on | Only the glyphs the file uses go in (plus any glyph they are built from). A font that uses this share of its glyphs or more, a font whose licence forbids subsetting, and every font with the choice off go in whole. |

PDF/X files always embed every font, the standard ones too, whatever
the choices. The writer:

- Writes an embedded font as a Type0 font with the Identity-H encoding
  (two bytes per glyph) over a CID font: `CIDFontType2` with the
  TrueType outlines in `FontFile2` (`CIDToGIDMap /Identity`), or
  `CIDFontType0` with the CFF outlines in `FontFile3` of subtype
  `CIDFontType0C`. CFF fonts are converted to CID-keyed CFF. A variable
  font is instanced at the weight and slant the text uses. A subset's
  name carries a six-letter tag (`ABCDEF+Name`). The `W` array gives
  each glyph's advance, and a `ToUnicode` CMap maps each glyph to the
  characters it stands for (a ligature to all of them), so copying and
  searching give back the text.
- Places glyphs as the text engine lays them out: one text matrix per
  run of glyphs of one font on one line (size, position, slant, turn,
  the object's transform), with `TJ` adjustments for kerning, tracking
  and justification, and a matrix of its own for each glyph that leaves
  the line (text on a path, shifted or turned characters, drop caps).
- Keeps paint: a uniform fill (RGB, CMYK, grey, spot inks) is the
  text's fill (render mode 0); a plain outline is its stroke (mode 1,
  or 2 with a fill), with its width, caps, joins and dashes. An outline
  set behind the fill is painted first and the fill over it, as on
  screen, for text and every other object (also in PostScript and
  EPS; SVG writes `paint-order="stroke"`) (#235). A gradient,
  pattern, texture or mesh fill, and a calligraphic, variable,
  painterly or painted outline, is painted on the text's outlines as
  for any curve; the text then goes over it invisibly (mode 3, or as
  the plain outline's stroke), so it can still be searched. Underlines,
  strike lines and hyphens added at line breaks are curves. Letters
  drawn bold by doubling the outline (a family without a bold face) are
  curves with invisible text over them.
- Respects licences: a font whose OS/2 embedding bits allow no
  embedding (restricted licence, or bitmaps only) is never embedded;
  its text is written as curves. Publish to PDF names those fonts on
  the status line, and the preflight's "Font may not be embedded" note
  (`preflight.md`) lists the text that uses them before publishing.

As curves, glyph outlines keep the font's own segments: lines stay
lines, cubic curves stay cubic, and each quadratic segment of a
TrueType outline becomes the one cubic that draws the same curve
(handles two thirds of the way from each end to the control point).
Nothing is flattened into short lines.

## Prepress

Bleed: the drawing's bleed (Document Properties) to start with, 0 to
50 mm, around every page (trim box the page, media box the page and the
bleed).

## Issues

What the preflight finds in the drawing (`preflight.md`), each with its
warning or note sign and its page. The tab is titled "No Issues" when
there are none, "Issues (n)" otherwise.

## Document

- Title (the drawing's name), Author, Subject and Keywords, filled in
  from the drawing's properties. An empty field leaves the entry out of
  the file. Text that is not plain ASCII is written as UTF-16 with a
  byte order mark. PDF/X files repeat the four in their XMP metadata
  (`dc:title`, `dc:creator`, `dc:description`, `pdf:Keywords`).
- A bookmark for each page (on by default): one outline item per page in
  page order, named after the page, that goes to the top left of the
  page.
- Show the bookmarks when the file opens (off by default): the catalog's
  `/PageMode /UseOutlines`.

## Security

PDF/X files cannot be protected (the standards forbid encryption): with
a PDF/X level picked the page says so and its fields are disabled.

- Require a password to open the file, and its password.
- Restrict printing, copying and changes with a password, and its
  password. Without it, whoever opens the file may do everything.
- Show passwords (they are masked otherwise).
- Permissions, enabled with the second password: Printing (Not allowed,
  Low resolution only, Allowed), Copy text and graphics, Change the
  content, Add comments, Fill in forms, Insert, delete and rotate pages,
  Text access for screen readers. All allowed by default.

OK refuses, naming the problem in red and showing the page to fix: the
Security page when a password that is asked for is empty or the two
passwords are the same, General when the page list names no page or
there is no file name. The passwords are never kept; the other choices are kept in
the preferences (`publish_pdf`) for next time.

### How a file is protected

The standard security handler of ISO 32000-2, revision 6:

- A file key of 32 random bytes encrypts every string and stream of the
  file (the encryption dictionary aside) with AES-256 in CBC mode, a
  random 16-byte IV in front of the data and PKCS #7 padding.
- Passwords are UTF-8, cut to 127 bytes. Each password's check value
  (`/U`, `/O`) is the algorithm 2.B hash of the password and an 8-byte
  random validation salt (the owner's also of `/U`), followed by that
  salt and an 8-byte key salt; the hash of the password and the key
  salt wraps the file key (`/UE`, `/OE`, AES-256 without padding and a
  zero IV). Without an open password the user password is empty.
- `/P` holds the permissions: bits 3 (print), 4 (modify), 5 (copy),
  6 (annotate), 9 (fill forms), 10 (accessibility), 11 (assemble) and
  12 (full-quality print) on when allowed, bits 7, 8 and 13 to 32 set,
  1 and 2 clear. `/Perms` repeats them with `T` (metadata encrypted) and
  `adb`, encrypted with the file key (AES-256, one block).
- The file is written as PDF 1.7 with extension level 8. There is no
  top-level `/Length` (the standard allows it only with V 2 or 3); the
  crypt filter `/StdCF` gives `/AESV3` and `/Length 32`.

## Command line

`tracedraw-cli convert in out.pdf` takes the same choices from the
environment: `TRACEDRAW_PDF_BOOKMARKS=1`, `TRACEDRAW_PDF_OPEN_PASSWORD`,
`TRACEDRAW_PDF_OWNER_PASSWORD` and `TRACEDRAW_PDF_DENY` (a comma list of
print, hq-print, copy, modify, annotate, forms, assemble, accessibility;
it needs the owner password). `TRACEDRAW_PDF_TEXT=curves` writes text
as curves and `TRACEDRAW_PDF_TEXT=named` leaves the fonts out of the
file.

## Checks

- Page lists, presets, the selection at its size or on the page, and every open drawing
  (`ui::publish_pdf::tests::page_lists_presets_and_what_goes_in`).

- `ui::publish_pdf::tests::the_dialog_starts_from_the_drawing_and_checks_the_passwords`
- `ui::publish_pdf::tests::publishing_writes_the_pages_with_bookmarks_and_protection`
- `pdf::pdfx_tests::bookmarks_name_every_page_and_can_open_the_panel`
- `pdf::pdfx_tests::protected_files_open_with_their_passwords_only`: another
  implementation accepts both passwords, refuses a wrong one, decrypts
  the information and the content, and the `/UE` and `/Perms` values
  unwrap and decrypt as the standard says.
- `pdf::pdfx_tests::document_properties_fill_the_information_and_the_xmp`
- `pdf_crypt::tests::permissions_make_the_p_value`
- `pdf_crypt::tests::objects_get_their_strings_and_streams_encrypted`
- `tests::pdf_protection_comes_from_the_environment_values` (command line)
- Text and fonts: `pdf::text_tests::text_goes_in_as_text_with_an_embedded_subset_font`
  (a Type0 font, Identity-H, `FontFile2`, ToUnicode, a tagged subset
  smaller than the font, the string extracted back by another PDF
  library), `text_as_curves_writes_no_fonts`,
  `a_font_that_may_not_be_embedded_goes_as_curves`,
  `outlines_fills_and_synthetic_bold_pick_the_render_mode`,
  `turned_text_and_cmyk_colours_keep_their_matrix_and_colour`,
  `named_fonts_and_standard_fonts_stay_out_of_the_file`,
  `subsetting_and_the_threshold_decide_how_much_goes_in`,
  `protected_files_keep_their_text`, `text_as_curves_keeps_the_font_curves`,
  `quadratic_segments_become_the_same_cubic` (also `eps::tests::`),
  `pdf::fonts::tests::*` (licence bits, standard fonts, WinAnsi, the
  ToUnicode CMap), `ui::publish_pdf::tests::publishing_keeps_text_as_text_unless_curves_are_asked_for`
  (the real text engine with an installed font, and `FontFile3` for a
  font with CFF outlines when one is installed), and
  `tests::pdf_text_comes_from_the_environment_value` (command line).
- Checked by hand: files written by the dialog and the command line open
  in two other PDF libraries and a PDF checker with either password,
  which report the permissions as chosen and no errors.
