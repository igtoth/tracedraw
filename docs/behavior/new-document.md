# Create a New Document

File > New (Ctrl+N), the New button of the standard toolbar, the New
button after the last document tab and the Welcome Screen's New Document
open the Create a New Document dialog. With "Do not show this dialog
again" ticked (or Options > General > Show New Document dialog box
cleared) these commands create the drawing at once with the settings last
used in the dialog. Every new drawing opens in its own tab.

## Layout

| Section | Control | Default |
|---|---|---|
| General | Name | the next `Untitled-N` of the session |
| | Preset (destination) and a menu button with Save Preset / Delete Preset | TraceDraw default |
| | Number of pages | 1 (1..999) |
| | Primary color mode: CMYK or RGB | CMYK |
| Dimensions | Page size (paper list, or Custom) | A4 |
| | Width, with the drawing units list | 210 mm, millimeters |
| | Height, with the portrait and landscape buttons | 297 mm, portrait |
| | Resolution (list and field), dpi | 300 |
| Color settings (collapsed) | RGB, CMYK and grayscale profiles, rendering intent | sRGB IEC61966-2.1, the built-in CMYK, Dot Gain 20%, Relative colorimetric |

Under the sections: a help button, "Do not show this dialog again", OK
and Cancel. Enter presses OK.

## Presets

| Preset | Colour mode | Size | Units | Resolution |
|---|---|---|---|---|
| TraceDraw default | CMYK | A4 | mm | 300 dpi |
| Default CMYK | CMYK | A4 | mm | 300 dpi |
| Default RGB | RGB | A4 | mm | 300 dpi |
| Web | RGB | 1920 x 1080 px | px | 72 dpi |

Choosing a preset sets those five values. Changing any of them afterwards
shows Custom unless the values match the preset again. Save Preset asks
for a name and adds the current values to the list; Delete Preset removes
the selected saved preset (built-in ones cannot be deleted).

Choosing a page size keeps the current orientation. A typed width and
height that match a paper size in either orientation select its name;
anything else shows Custom.

## Result

OK opens a drawing with the given name, number of pages (each named
"Page N" with one layer), page size, resolution (document metadata, used
for rasterised effects), primary colour mode and profiles, and switches
the property bar's units to the chosen drawing units. The dialog's values
are kept in the settings file (`new_document`) for the next time.

## Checks

- `new_document::tests::build_makes_the_pages_size_resolution_and_colour_mode`
- `new_document::tests::editing_a_value_turns_the_preset_into_custom`
- `new_document::tests::page_sizes_keep_the_orientation_and_typed_sizes_find_their_name`
- `new_document::tests::ok_opens_a_new_tab_and_remembers_the_settings`
