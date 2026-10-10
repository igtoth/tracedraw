# Status bar

From the left:

1. The settings button. Its menu chooses what the first field shows:
   Tool Hints (default), Object Details, Cursor Coordinates or Document
   Color Settings. The choice is kept in the settings (`status_info`).
2. The first field:
   - Tool Hints: the active tool's hint (`status_hint.<tool id>` in the
     language files; every tool has one). The Pick tool's hint changes
     when something is selected. A message from the last command comes
     first when there is one.
   - Object Details: width, height and centre of the selection in the
     drawing units.
   - Cursor Coordinates: the pointer's page position in the drawing units.
   - Document Color Settings: the RGB, CMYK and grayscale profiles.
3. The object information: "Rectangle on Layer 1", "Curve on Layer 1",
   "2 Objects Selected on Layer 1" (or "on Multiple Layers"); with the
   Shape tool and a curve selected, "Curve: N Nodes"; while typing,
   "Editing text". It starts after the first field, never closer than
   156 px from it.
4. The fill: the Interactive Fill icon, a swatch and a description (None,
   the colour values, or the fill type). At 69.5 % of the bar.
5. The outline: the outline pen icon, a swatch and the colour values with
   the width in the drawing units (three decimals; two in pixels), or
   Hairline, or None. At 82.5 % of the bar.
6. The proof colours button at the right end (a small monitor); a click
   toggles soft proofing and its tooltip says whether it is on.

With nothing selected the fill and outline parts show the defaults for
new objects: no fill, a 0.2 mm black (C0 M0 Y0 K100) outline. A swatch
with no colour is a white box crossed by a red line. Double-clicking the
fill part opens the fill settings (as F11); the outline part, the
outline settings (as F12).

With no drawing open the bar shows the placeholders Tool Hints, Object
Information, Fill Color and Outline Color.

## Checks

- `ui::status::tests::every_tool_has_its_own_hint`
- `ui::status::tests::object_information_names_the_object_and_its_layer`
- `ui::status::tests::outline_width_is_shown_in_the_drawing_units`
