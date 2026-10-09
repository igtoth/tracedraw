# Pick tool

| Action | Behaviour |
|---|---|
| Click | select the topmost object under the pointer (its fill counts only when filled; otherwise its outline within 3 px; text, groups, tables, bitmaps, ClipFrames and symbols count anywhere inside their bounds) |
| Shift+click | add or remove from the selection |
| Alt+click | select the object beneath the current one (dig) |
| Click on selected | toggle the rotate/skew handles |
| Drag on empty | marquee; objects fully inside are selected; Alt makes touching count |
| Drag object | move; Ctrl constrains to the axis; right-click while dragging leaves a copy; Space while dragging drops a copy |
| Corner handle | scale proportionally; Shift scales from the centre; Alt scales freely |
| Side handle | stretch one axis |
| Rotate handle | rotate about the centre marker; Ctrl constrains to 15 degrees |
| Skew handle | skew along the edge |
| Arrows | nudge by the nudge distance (2.54 mm default); Shift x10 (super nudge), Ctrl x0.1 (micro nudge) |
| Tab / Shift+Tab | cycle selection in stacking order |
| Double-click | Shape tool on a curve; on text, the Text tool editing that text with the caret at the end |
| Esc | deselect |

Selecting inside a group: Ctrl+click.

## Property bar

With nothing selected the bar shows the page fields (size, orientation,
units, nudge distance). With a selection it shows X, Y (lower left corner),
W, H, scale, angle, mirror buttons, the outline width, To Front, To Back
and Convert to Curves. Text selected on its own adds the text fields of the
Text tool after them. Bitmaps selected on their own replace Convert to
Curves with the bitmap commands: Edit Bitmap, Crop Bitmap (switches to the
Crop tool), Trace Bitmap (Quick Trace and the centreline and outline
presets), Straighten Image and Resample.

## Dialogs

Enter presses OK in every modal dialog (Esc cancels). In the two
multi-line fields (QR code text and document notes) Enter inserts a line
break instead.
