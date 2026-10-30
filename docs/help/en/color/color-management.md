# Color management

Screens show colors with red, green and blue light; printers mix cyan,
magenta, yellow and black inks. Color management uses **color profiles** to
describe each device, so a color keeps its look from the screen to the
printed page. The settings are in
[[menu.tools > menu.tools.color_management]], the
[[color_mgmt.title]] dialog box. Nothing changes until you click
[[dialog.ok]].

![The Color Management Settings dialog box](../../img/color-color-management.png)

## The drawing's profiles {#document}

The [[color_mgmt.tab_document]] tab sets the profiles of the active drawing.

- [[color_mgmt.current_profiles]] shows the RGB, CMYK and grayscale profiles
  the drawing uses now.
- Choose how a new profile is applied:
  - [[color_mgmt.assign]]: the color values stay and their look changes to
    the new profile;
  - [[color_mgmt.convert]]: the CMYK colors are converted so they look the
    same in the new profile.
- Choose the [[color_mgmt.rgb_profile]], [[color_mgmt.cmyk_profile]] and
  [[color_mgmt.grayscale]] profiles. Each list has the default profiles, then
  every profile found on your computer. [[color_mgmt.load_profiles]] adds
  profile files (`.icc`, `.icm`).
- [[color_mgmt.primary_mode]]: CMYK or RGB.
- [[color_mgmt.intent]]: how colors that cannot be shown are brought in:
  [[color_mgmt.intent_relative]] (at first), [[color_mgmt.intent_perceptual]],
  [[color_mgmt.intent_saturation]] or [[color_mgmt.intent_absolute]].

The CMYK profile goes with the drawing, also into `.cdr` files from version
16.

## Defaults for new drawings {#default}

The [[color_mgmt.tab_default]] tab sets what new drawings use, and how
colors are converted.

- [[color_mgmt.preset]]: ready-made settings for North America, Europe,
  Japan, or web and screen work. [[color_mgmt.save_preset]] saves your own
  settings under a [[color_mgmt.preset_name]]; [[color_mgmt.delete_preset]]
  removes one.
- The default [[color_mgmt.rgb_profile]], [[color_mgmt.cmyk_profile]] and
  [[color_mgmt.gray_profile]], [[color_mgmt.primary_mode]] and
  [[color_mgmt.intent]].
- [[color_mgmt.conversion]]:
  - [[color_mgmt.engine]]: [[color_mgmt.engine_icc]], which uses the
    profiles, or [[color_mgmt.engine_builtin]];
  - [[color_mgmt.preserve_black]]: colors of black ink only stay black ink
    only when converted;
  - [[color_mgmt.map_gray]] (on at first): grays converted to CMYK use black
    ink only;
  - [[dialog.black_point]] (on at first): keeps the darkest shades apart.
- [[color_mgmt.open_policy]]: what happens to the profile of a file you open,
  for RGB, CMYK and grayscale: [[color_mgmt.open_use_embedded]],
  [[color_mgmt.open_assign_default]] or [[color_mgmt.open_convert_default]].
  [[color_mgmt.warn_mismatch_box]] and [[color_mgmt.warn_missing_box]] show a
  message in the status bar when a file has another profile or none.
- [[color_mgmt.import_policy]]: [[color_mgmt.import_convert]] or
  [[color_mgmt.import_assign]], for what you import or paste.
- [[color_mgmt.spot_definition]]: whether spot colors are defined by
  [[color_mgmt.spot_lab]], [[color_mgmt.spot_cmyk]] or
  [[color_mgmt.spot_rgb]].

## Previewing print colors {#proof}

[[menu.view > menu.view.proof_colors]], or the proof button at the right end
of the status bar, shows the drawing as it would print: colors that CMYK inks
cannot print look duller. The [[docker.color]] panel also warns about such
colors.

Without loaded profiles, TraceDraw converts colors with its own model of
process inks on coated paper.
