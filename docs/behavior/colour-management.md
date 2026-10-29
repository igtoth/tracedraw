# Colour management

| Setting | Default |
|---|---|
| RGB working profile | sRGB IEC61966-2.1 |
| CMYK working profile | an open CMYK profile shipped with the app (no vendor profile) |
| Grayscale | Dot gain 20% |
| Rendering intent | Relative colorimetric |
| Black point compensation | on |
| Policy for embedded profiles | use embedded, no warnings |
| Soft proof | off; profile = CMYK working profile |

## The Color Management Settings dialog

Tools > Color Management (and the Launch list) opens a dialog with two
tabs; nothing changes until OK. "?" opens the help.

**Document**: Current Profiles (the drawing's RGB, CMYK and grayscale
profiles: its own CMYK profile when it carries one, else the names it
stores, else the defaults). Then Assign new (preserve colour values: the
numbers stay, so the colours may look different under the new profile)
or Convert (preserve colour appearance: the numbers change so the colours
look the same), and an RGB, a CMYK and a Grayscale
list: "(Default) name" under Default profiles, then Load color
profiles... and every RGB, CMYK or grayscale ICC profile found in the
system folders (`/usr/share/color/icc`, `~/.local/share/color/icc`, the
ColorSync folders, the Windows colour folder) under Other profiles, by
their descriptions. Primary color mode (CMYK, RGB) and Rendering intent.
OK assigns the chosen profiles to the drawing (one undo step; the CMYK
profile's file goes with the drawing and into `.cdr` files from version
16); with Convert the drawing's CMYK colours are first converted from its
old CMYK profile to the new one through the ICC engine, so they look the
same.

**Default**: Preset (North America, Europe, Japan general purpose, Web
and screen, and presets the user saves with the disk button and deletes
with the bin), the default RGB, CMYK and grayscale profiles, primary
colour mode and rendering intent of new drawings; Color conversion:
Color engine (the ICC engine or the built-in formulas), Preserve pure
black (CMYK colours of black ink only stay so when converted between
profiles), Map gray to CMYK black (neutral greys converted to CMYK use
black ink only; on at first), Black point compensation; Open policy for
RGB, CMYK and grayscale (use the embedded profile, assign the default
one, convert to the default one) with warnings on a different or missing
profile (shown in the status bar when a `.cdr` file opens); Import and
Paste policy (convert to the drawing's profile, or assign it: CMYK
colours of shapes imported from a `.cdr` file that carries a profile are
converted with Convert); Spot color definition (Lab, CMYK or RGB values).

## Conversions without a loaded profile

CMYK uses a built-in model of process inks on coated stock
(`core/press.rs`) of a coated web offset CMYK space shown with relative
colorimetric intent and black point compensation: process black is the
warm dark grey (34, 31, 32) rather than (0, 0, 0), cyan is (0, 173, 239),
magenta (236, 0, 139), yellow (255, 241, 0).

- CMYK to RGB: a Yule-Nielsen modified Neugebauer model. Each ink's
  coverage goes through its dot gain curve (11 points), the 16 overprints
  of solid inks are mixed with the Demichel weights in a power-law space
  (exponent per screen channel), the result is linear sRGB and is clipped
  to the screen gamut. The constants were fitted so the default palette
  lands within 2.5 Delta E (CIE76),
  any ink mixture up to 320% within about 1 on average and 2.2 for 99% of
  them.
- RGB to CMYK: pure black becomes
  K 100 only, neutral greys use black ink only (the grey with the same
  mean level), and other colours get a grey component replacement (black
  grows with the darkness of the colour) with C, M and Y solved by
  Levenberg-Marquardt so the colour shows as asked or as close as the
  press can print, within a 300% ink limit. Screen colours outside the
  press gamut (pure RGB red, green, blue) come back as the nearest
  printable colour, so the gamut alarm flags them.
- A colour already in the target model is returned unchanged.
- Gray: `y = 0.2126 r + 0.7152 g + 0.0722 b` in linear light.
- HSB, HSL: standard hexcone conversions.
- Lab: through sRGB linearisation and D65 XYZ, then CIE Lab; D50
  adaptation (Bradford) when a profile asks for it.

## With a CMS

`IccEngine` (core/color.rs) holds the working RGB profile, the CMYK
profile, the intent and the black point compensation flag, and builds the
transforms once: RGB and CMYK to the display (sRGB), RGB to CMYK and back.
`Color::to_rgb8`, `convert_to` and the gamut warning go through it when
it is installed; otherwise the built-in conversions apply. Profiles are loaded
from `.icc`/`.icm` files in Tools > Colour Management (Load profile...),
remembered by path in the settings and installed again at start-up. The
CLI `tracedraw-cli icc <file>` prints a profile's class, spaces, version
and the sRGB values of its primaries, which is how a profile is checked.
The document stores the profile names used for its colours.

## PDF/X export

Export > PDF offers PDF 1.4, PDF/X-1a:2003, PDF/X-3:2003 and PDF/X-4
(`pdf::PdfOptions`):

- Every level writes an Info dictionary (Title, Creator, Producer,
  dates, Trapped False, GTS_PDFXVersion and, for X-1a, Conformance), a
  trailer ID, an XMP metadata stream with the pdfxid version, and an
  OutputIntent (`/S /GTS_PDFX`) whose identifier is the output condition
  typed in the dialog (default FOGRA39; when empty, the embedded
  profile's description, "Custom" if it has none, or "CGATS TR 001"
  without a profile). The CMYK profile loaded in Colour Management, else
  the CMYK profile the drawing carries (`Document::cmyk_profile`, read
  from .cdr files), is embedded as
  DestOutputProfile (`/N` from its colour space signature).
- X-1a: PDF 1.4, every colour converted to CMYK (ICC engine when a CMYK
  profile is loaded, else the press model; image pixels with the simple
  black generation `k = 1 - max(r, g, b)`), images written as
  DeviceCMYK, shadings in DeviceCMYK, no ExtGState alpha and no soft
  masks. Objects with opacity, transparency or lens effects, and
  bitmaps with transparent pixels, are flattened: everything drawn up to
  and including the object is rendered at the export resolution and the
  crop over the object's bounds replaces it as an opaque image.
- X-3: like X-1a but RGB colours stay RGB (the output intent gives them
  meaning).
- X-4: PDF 1.6, transparency kept.
- Bleed: MediaBox grows by the bleed on every side, TrimBox is the page,
  BleedBox the media box; content is shifted by the bleed.

`pdfinfo` reports "PDF subtype: PDF/X-1a:2003" for the X-1a output and
`qpdf --check` finds no errors; a sample X-1a file of a real banner
rendered the same picture through the DeviceCMYK image.

## A drawing's own CMYK profile

A drawing can carry the CMYK profile its CMYK values were chosen in: a
`.cdr` file's profile (under `color/profiles/cmyk/`) comes with the
drawing and is saved in its `.tdraw` file. While that drawing is the
active one, its CMYK colours show through its profile (rendering intent
and black point compensation from the settings); other drawings use the
profile of the settings. Switching tabs, opening or reverting a drawing
installs the right one. `tracedraw-cli` renders and exports through the
drawing's profile too (relative colorimetric, no black point
compensation).

## Convert Colors

Tools > Convert Colors (#972) turns every colour of the selected objects,
or of the whole drawing, into CMYK or into RGB, in one undo step
("Convert Colors", `Command::ConvertColors`). The dialog: Convert to
CMYK (default) or RGB; Colors of the selected objects (the default when
something is selected) or the whole drawing (the pages' backgrounds and
the symbols too); Convert spot colors too (off).

- What converts: solid fills, gradient stops, mesh node colours,
  pattern and texture colours, outlines, text, table cells, the colours
  of effects (contour, blend, extrude, bevel, block shadow and the rest)
  and drop shadows, in groups and clip frames at any depth
  (`tracedraw_core::color_convert`: every colour of an object's saved
  form is found by its model tag, so no part is missed).
- How: `Color::convert_to`, so the conversion goes through the colour
  engine with the profiles above (the drawing's own CMYK profile when it
  carries one, else the settings' RGB and CMYK profiles, the rendering
  intent and black point compensation; Map gray to black applies), and
  through the built-in press model when no profile is loaded.
- Registration colour never changes. Spot colours stay unless the box is
  ticked; then each becomes its process stand-in at its tint in the model
  asked for. A colour already in the model stays. Bitmaps keep their
  pixels.
- The status line says how many colours changed; with none to change (or
  Selected objects with nothing selected) nothing is done and no undo
  step is made.

Checks: `color_convert::tests` (stops, outline, background, spots kept
and converted when asked, registration kept, a selection only, effects
and shadows), `ui::convert_colors::tests::colours_convert_for_the_selection_or_the_drawing`
(the dialog by Enter, one undo step each, nothing to change makes no
step).

## Proof colours

View > Proof Colors, the status bar's proof button and Options > Display
show the drawing as it would print: every colour of the rendered page
goes to CMYK and back (`color::proof_rgb`), through the drawing's or the
settings' CMYK profile when there is one, else the press model, so
screen colours CMYK cannot print look duller. The page is proofed on a
17 x 17 x 17 grid of screen colours, made once per colour setting, with
colours between interpolated (trilinear); transparent and half-covered
pixels keep their coverage. Paper white stays white. The desktop, the
selection and the guides are not proofed.

## Thin outlines on screen

An outline thinner than one pixel at the current zoom (or the
resolution of a bitmap export) is drawn one pixel wide at full strength
instead of a faded fraction of a pixel; vector exports keep the true
width.

## Checks

- `color::tests::proofed_colours_are_what_cmyk_can_print` (core),
  `raster::tests::proofed_pixels_follow_the_grid_of_proofed_colours`.

- `app::tests::a_drawing_shows_through_the_cmyk_profile_it_carries`,
  `container::tests::the_cmyk_profile_comes_with_the_document` (cdr crate),
  `document::tests::a_carried_colour_profile_is_saved_with_the_drawing`
  (core) and `tests::outlines_thinner_than_a_pixel_stay_one_pixel_wide`
  (render crate).

- `tracedraw_core` `press::tests::process_inks_show_as_on_a_coated_web_press`:
  the default palette colours on screen within 5 levels.
- `press::tests::printable_colours_round_trip`: CMYK to RGB to CMYK to RGB
  within one level for ink totals up to 260%.
- `press::tests::black_and_greys_use_black_ink_only`,
  `press::tests::screen_colours_outside_the_press_gamut_come_out_close`,
  `press::tests::more_ink_never_shows_lighter`,
  `press::tests::bad_numbers_do_not_break_it`.
- `color::tests::screen_red_is_outside_the_press_gamut`,
  `color::tests::converting_to_the_same_model_keeps_the_colour`.
- OK in the dialog stores the defaults and gives the drawing the chosen
  grayscale profile, primary mode and intent; both tabs draw and Enter is
  OK (`ui::color_management::tests::ok_sets_the_defaults_and_the_drawings_profiles_and_mode`).
- Presets fill the three default profiles
  (`ui::color_management::tests::presets_fill_the_three_default_profiles`).
- Converting between CMYK profiles keeps pure black when asked, leaves
  other models alone, and recolours shapes inside groups
  (`color_policy::tests`).
