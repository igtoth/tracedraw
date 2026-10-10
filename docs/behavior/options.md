# Options dialogs

Tools > Options is a submenu: TraceDraw
(Ctrl+J), Customization, Tools, Global and Workspaces each open their own
dialog. Layout > Document Options opens the document's dialog (also a
double-click on a ruler, the page border or its shadow).

## Layout

- 887 x 663 px, centred: a white 30 px title bar with the title at the
  left and a close button at the right (red under the pointer), a light
  grey body, a 1 px grey border.
- A white page list on the left (225 px; rows 23 px apart; the current
  page `#CCE8FF` with a `#99D1FF` frame) and the white page frame on the
  right (622 px), both 566 px high with a 1 px `#D9D9D9` border.
- Section headings are a label followed by a light line to the right
  edge. Indented groups have no guide line.
- Bottom row: the help button (27 x 27) at the left (and Save as Default
  in Document Options), OK and Cancel (100 x 27) at the right.
- Changes show at once. OK keeps them and writes the preferences; Cancel,
  the close button and Esc put back everything the dialog changed,
  including document settings (their commands are undone).

## TraceDraw Options

| Page | Settings (defaults) |
|---|---|
| General | On start-up: Welcome Screen, Start a new document, Open last edited document; Show New Document dialog box (on); Undo levels, Regular (150) |
| Display | Proof colors; Show tooltips (on); Hide bounding box for curve tools (off); Full-screen preview: Show page border (on); Default action for mouse wheel: Zoom or Scroll |
| Edit | Constrain angle (15 degrees: Ctrl while rotating); Drawing precision (3 decimal places in the property bar and the status bar) |
| Nodes and Handles | Node size Small, Medium, Large (7, 9, 11 px); node shape per type (cusp square, smooth circle, symmetrical diamond); Show curve direction (an arrow after the first node, secondary colour); main colour (selected nodes and handles, blue) and secondary colour (red); Show unselected nodes with fill (on, Ctrl+Shift+G) |
| ClipFrame | Auto-center new content: when it lies completely outside the frame (default), always, never; Show lines in empty ClipFrame frames (on, on screen) |
| Snapping | Snap to objects, page, guidelines, grid, baseline grid, pixels; snapping radius (10 px) |
| Save | Back up original file before saving (on): `backup_of_<name>` next to the file or in a chosen folder; Auto-backup every 20 minutes (on): each open drawing with unsaved changes as `AutoBackup_of_<name>.tdraw` in `TraceDraw` under the temporary folder or a chosen folder |
| Text | Keyboard text increment (1 pt: Ctrl+8 and Ctrl+2 grow and shrink the selected text); default font and size; hyphenation; non-printing characters |

## Customization

Appearance (desktop colour, page border, Outline flyout in the toolbox),
Commands (keyboard shortcuts of the tools), Command Bars (standard
toolbar, property bar, toolbox, status bar, Text, Zoom and Transform
toolbars), Color Palette (No Color well, right mouse button, document
palette; see `color-palettes.md`).

## Tools

Pick (cross hair cursor; treat all objects as filled: unfilled objects
are picked inside too), Zoom/Pan (right mouse button of the Zoom tool:
zoom out or context menu; mouse wheel action), Rectangle (corner radius),
Ellipse (ellipse, pie or arc and its angles), Polygon (points, sharpness),
Spiral (revolutions, symmetrical or logarithmic), Graph Paper and Table
(rows, columns), Eraser (thickness, round or square nib).

## Global and Workspaces

Global: user interface language; where the preferences and auto-backups
are kept. Workspaces: the workspace in use.

## Document Options

General (rendering resolution; Fill open curves, off: an open curve's
fill shows only in its closed subpaths, on screen, in hit testing and in
vector exports; duplicate distance), Page Size, Layout, Background, Bleed,
Rulers, Grid, Guidelines (see `rulers-grid-guidelines.md`). Save as
Default keeps the grid, ruler and guideline settings for new drawings.

## Checks

- `ui::options::tests::every_page_belongs_to_one_dialog`
- `ui::options::tests::cancel_puts_settings_and_document_back`
- `ui::options::tests::every_page_draws`
- `tracedraw_render` `open_curves_are_not_filled_unless_the_document_says_so`
- `tracedraw_io` `open_curves_lose_their_fill_in_vector_exports`
- `app::tests` start-up, backups and ClipFrame centring (below)
