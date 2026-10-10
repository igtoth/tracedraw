# Toolbox

- One button per flyout, top to bottom, with
  separator lines after the Zoom, Drawing, Text, Connector and
  Transparency groups. A button shows the tool last used from its flyout
  (kept in the settings); the active tool's button is tinted.
- Buttons with a flyout have a small arrow at the bottom-right. Clicking
  the arrow, right-clicking, dragging, or pressing and holding the button
  for 0.4 s opens the flyout; a click elsewhere picks the tool the button
  shows. While a flyout is open, hovering another flyout button opens that
  one instead; moving away from the toolbox or clicking elsewhere closes
  it.
- A flyout lists its tools vertically: a check mark beside the tool the
  button shows, the icon, the name and the shortcut at the right.
  Separators split Pick and Freehand Pick from Free Transform; Crop, Knife
  and Segment Delete from Eraser; Polygon, Star, Spiral and Common
  Shapes from Impact and Graph Paper; the dimension tools from the 3-point
  callout; the connectors from Anchor Editing; Interactive and Area Fill
  from Mesh Fill.
- Tooltips: "<Name> tool (<shortcut>)" in bold, then what the tool does.
  No tooltip shows while a flyout is open.
- Double-clicking a button: Pick selects all objects, Zoom fits all
  objects, Rectangle adds a page frame.
- The "+" at the end lists every flyout with a check box to show or hide
  it (the Outline flyout is hidden by default) and Reset toolbox.

## Checks

- `ui::toolbox::tests::flyout_groups_can_be_hidden_and_reset`
- `ui::toolbox::tests::separators_split_the_reference_subgroups`
- `ops::smoke_tests::toolbox_groups_remember_the_last_tool_used`
