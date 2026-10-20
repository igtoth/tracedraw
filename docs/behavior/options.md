# Options dialogs

Tools > Options is a submenu: TraceDraw
(Ctrl+J), Customization, Tools, Global and Workspaces each open their own
dialog. Layout > Document Options opens the document's dialog (also a
double-click on a ruler, the page border or its shadow).

## Layout

- 887 x 663 px, centred, framed like every dialog: a 1 px `#B2B2B2` border, a white 31 px title bar with the title
  (13 px) at the left and a close button at the right (red under the
  pointer), and a white 5 px frame round the `#F4F4F4` body.
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
| General | On start-up: Welcome Screen, Start a new document, Open last edited document; Show New Document dialog box (on); Show the splash screen at start-up (on); Undo levels, Regular (150) |
| Display | Proof colors; Show tooltips (on); Hide bounding box for curve tools (off); Enable node tracking (off, see `pick-tool.md`); Full-screen preview: Show page border (on); Default action for mouse wheel: Zoom or Scroll; Ruler calibration (below) |
| Edit | Constrain angle (15 degrees: Ctrl while rotating); Drawing precision (3 decimal places in the property bar and the status bar) |
| Nodes and Handles | Node size Small, Medium, Large (7, 9, 11 px); node shape per type (cusp square, smooth circle, symmetrical diamond); Show curve direction (an arrow after the first node, secondary colour); main colour (selected nodes and handles, blue) and secondary colour (red); Show unselected nodes with fill (on, Ctrl+Shift+G) |
| clip frame | Auto-center new content: when it lies completely outside the frame (default), always, never; Show lines in empty clip frame frames (on, on screen) |
| Snapping | Snap to objects, page, guidelines, grid, baseline grid, pixels; snapping radius (10 px); show snap location marks and the screen tip (on); the modes, each with its mark (all on), Select All, Deselect All |
| Alignment Guides | On or off; guides for the selection as one or each object; edges, centres, the page, equal spacing, equal sizes; margins with offset and inset; colour and line style; Reset (see `live-guides.md`) |
| Dynamic Guides | On or off; angles (check boxes every 15 degrees, others added by value); distance and angle tip; tick marks, their spacing, snap to ticks; colour and line style; Reset (see `live-guides.md`) |
| Save | Back up original file before saving (on): `backup_of_<name>` next to the file or in a chosen folder; Auto-backup every 20 minutes (on): each open drawing with unsaved changes as `AutoBackup_of_<name>.tdraw` in `TraceDraw` under the temporary folder or a chosen folder; a session that ended without closing offers them at the next start (see `recovery.md`) |
| Text | Keyboard text increment (1 pt: Ctrl+8 and Ctrl+2 grow and shrink the selected text); default font and size; hyphenation; non-printing characters |

## Customization

Appearance (Center Dialog Boxes, interface size, theme, desktop
colour), Command Bars (the standard toolbar, property bar, toolbox,
status bar, colour palette, Text, Zoom and Transform toolbars, with
Reset; Lock Toolbars and the Outline flyout in the toolbox), Commands
(shortcuts of your own for every menu command and the tool keys; see
`shortcuts.md`), Color Palette (document palette, No Color well, right
mouse button; see `color-palettes.md`).

## Tools

Pick (cross hair cursor; treat all objects as filled, on by default:
unfilled objects are picked inside too; marquee selects touched objects,
off by default: a marquee or freehand selection takes what it touches and
Alt only what lies wholly inside), Zoom/Pan (right mouse button of the Zoom tool:
zoom out or context menu; mouse wheel action), Curve tools (Freehand
smoothing, 0 to 100, 50 by default; Join and close within, 1 to 50
screen pixels, 5 by default; see `curve-tools.md`), Rectangle (Rectangle
corners: style, the four sizes, Edit corners together; Scale corners:
Relative corner scaling, off by default so that new rectangles keep
their corner size when resized; see `shape-tools.md`),
Ellipse (ellipse, pie or arc and its angles), Polygon (points, sharpness),
Spiral (revolutions, symmetrical or logarithmic), Graph Paper and Table
(rows, columns), Eraser (thickness, round or square nib).

## Global and Workspaces

Global > General: user interface language; check for updates at start (on, desktop, see `updates.md`); Reset all settings... (asks
first, then puts every preference back to how TraceDraw comes, keeping
the recent drawings, the last folder and the window's size; drawings are
not touched). Global > File Locations: where the preferences and
auto-backups are kept. Workspaces: the workspace in use, and exporting
and importing the user's setup to a `.tdws` file (see `workspaces.md`).

### Printing

What the Print dialog starts with (`printing` in the preferences): Fit
to page, Print separations, Bleed, Crop marks, Registration marks, Colour
bars, Densitometer scale, Page information (all off), the sheet (fitted
to the pages) with Landscape, and the space between pages (5 mm). The
rest starts as always: one copy, all pages, 100 %, as in the document.

- Remember the settings of the last print (off): Print in the dialog
  keeps its choices above for the next time.
- Check the drawing in the Print dialog (on): the preflight summary and
  its "Preflight..." button; off, the dialog runs no checks.
- A sheet out of the list or a space that is not a number from a file
  falls back to the defaults (the space is kept within 0 to 100 mm).

### File Formats

What File > Export (Ctrl+E) starts with (`formats`): the format (SVG),
the resolution of bitmap formats (300 dpi, 1 to 2400), the JPEG and
WebP quality (90, 1 to 100), Transparent background (on), the PDF
standard (PDF 1.4), the PDF/X output condition (FOGRA39, at most 64
characters) and the PDF bleed (0 mm, 0 to 50). Remember the settings of
the last export (off): exporting keeps the dialog's choices for the next
time. Export for Web and Export for Office keep their own presets and
never change these.

Imported objects go (File > Import, Ctrl+I): where the file has them
(the default), or centred in the middle of the view; the move is part of
the import's undo step.

Saving .cdr files: the Version new drawings are saved in (`cdr_version`,
27 by default, 14 to 27; a value out of range from a file is brought
into it). Saving a `.cdr` file sets it to the version chosen there; a
drawing opened from a `.cdr` file keeps its own when TraceDraw saves
that version (`files.md`).

### Bitmap Tracing

Quick Trace uses (`tracing`): one of the eight presets (Clipart by
default). Every trace starts with: Remove background (on) and Delete
original image (off); Quick Trace and the Trace dialog's presets take
them, and the dialog can still change them for one trace.

### Warnings

Show a message (`warnings`, all on):

- When objects of an opened file could not be read: after a `.cdr` file
  opens with objects left out, a message gives their number and a Report
  button that opens a new issue for the file.
- When an opened drawing uses fonts that are not installed: the message
  names them (the text shows in a similar font and keeps its font name).
- Before an item is removed from the Shelf: Remove or Cancel.

Both parts after opening share one message. Each message has "Do not show
this message again" (the Shelf's: "Do not ask again"), which turns off
the matching choice here; ticking it again brings the message back. A
message never replaces another dialog that is open.

### Pages with no equivalent

Pen settings (pressure curves for tablets), Tablet Mode, VBA and
plug-ins have no page: TraceDraw reads no pen pressure, has its own
workspaces instead of a tablet mode, scripts in JavaScript (Scripts
panel) and hosts no plug-ins.

## Interface size

Customization > Appearance > Interface size sets how large the menus,
bars, icons and dockers are drawn, relative to the scaling the system
gives the monitor the window is on: a slider and a field from 75 % to
250 % in steps of 5 (100 % the default, Reset to Default goes back to
it), applied when the slider is let go.

- 100 % is the size of the other programs on that monitor, and it
  follows the monitor: moving the window to a monitor the system scales
  differently rescales it. 200 % is twice that size.
- The interface's own measures are 13 px text, 20 px icons and 24 px
  rows. On Windows 100 % is a zoom of 12 / 13 over the system's scaling,
  so the text comes out as large as the system's own (Segoe UI 9 pt,
  12 px at 100 % scaling, 15 px at 125 %); at 0.8 it was 10.4 px at
  100 %, too small (#229).
- On Linux 100 % is a zoom of 1 over the system's scaling: one interface
  point per pixel at the system's scaling (#1). Elsewhere (macOS, the
  browser) it is 0.8. The sizes offered and their order are the same.
- The size is a zoom factor over the monitor's own scaling, so text stays
  sharp. Values out of 50 % to 400 % count as 100 %.
- The interface size never changes how large drawings are: zoom 100 %
  shows them at their real size (see Ruler calibration) at any size.
- Preferences written before sizes were relative (`ui_scale`, an absolute
  size) are ignored: the size starts at 100 % (`ui_size`).
- Ctrl+plus and Ctrl+minus zoom the drawing, never the interface.

## Ruler calibration

Options > Display > Ruler calibration sets the screen resolution in
pixels per inch at the system's scaling (96 by default, 48 to 400,
`screen_ppi` in the preferences) and draws a 100 mm ruler and a 4 inch
ruler at that resolution: hold a real ruler against the screen and
change the value until the marks match. Zoom 100% then shows a drawing
at its real size (the zoom box percentages follow; the Pixels view shows
one pixel at the system's scaling per document pixel). Interface points
per inch are that resolution over the interface's zoom (104 at 100 %
on Windows, 96 on Linux, 120 elsewhere), so the interface size never changes real sizes. Reset puts
96 back.

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
- `ui::tests::interface_size_is_relative_to_the_system_scaling`
- `prefs::tests` (print, export and trace preferences; the dialogs
  starting with them; imports centred in the view; the messages after
  opening; the Shelf asking before removing)
- `ui::options_global::tests::the_global_pages_draw`
- `drawing_scale::tests::zoom_100_follows_the_ruler_calibration`
- `tracedraw_render` `open_curves_are_not_filled_unless_the_document_says_so`
- `tracedraw_io` `open_curves_lose_their_fill_in_vector_exports`
- `app::tests` start-up, backups and clip frame centring (below)

## Theme

Customization > Appearance > Color > Theme: Light (the default), Medium
Light, Medium Dark, Dark or High Contrast (`theme.rs`, one palette each). Panels, bars,
menus, dialogs, lists, rulers, tabs and icons take the theme's colours;
the page, the drawing, the accent blue and the desktop colour (its own
setting) do not. The button with a moon (light themes) or a sun (dark
themes) at the right end of the status bar switches to Dark or to
Light. The choice is kept in the preferences and applies at once.

## Coloured tool icons

Customization > Appearance > Coloured tool icons (off by default): the
toolbox, its flyouts and the Hints panel draw each tool in its group's
colour (pick blue, shape editing purple, crop red, zoom teal, curves
green, drawing orange, shapes light blue, text grey, dimensions slate,
connectors sea green, effects magenta, transparency indigo, eyedroppers
ochre, fills orange, outlines navy), 35 % lighter on dark themes. The
High Contrast theme keeps one icon colour.

## Desktop accessibility settings

Read once at start-up (`system_prefs.rs`, with the `reg` tool on
Windows; elsewhere nothing is read and the defaults apply):

- Text size (Windows Settings > Accessibility > Text size, the
  `TextScaleFactor` value, 100 to 225 %): with "Grow with the desktop
  text size" (Appearance, on by default) the interface size is
  multiplied by it, up to 400 %. The option shows the size read.
- High contrast (the `Flags` value of the HighContrast key, bit 1):
  with "Use high contrast while the desktop has a high contrast theme
  on" (on by default) the High Contrast theme is used whatever theme is
  picked. Its colours are the desktop's (`Control Panel\Colors`:
  Window, WindowText, Hilight, HilightText, ButtonFace, ButtonText,
  GrayText, HotTrackingColor): panels in the button face colour, fields
  and menus in the window colour, text in the button text colour,
  disabled text in the grey text colour, hovered and picked rows in the
  highlight colour with the highlight text colour on them, links and
  focus frames in the hot-track colour. Picked by hand, or when a colour
  cannot be read, the classic black theme's colours are used.

## Where the preferences are kept

- `TRACEDRAW_CONFIG_DIR` when it is set.
- Otherwise a folder called `tracedraw-settings` next to the program,
  when there is one: a portable copy (on a USB stick, say) keeps its
  preferences there and leaves nothing on the computer.
- Otherwise the system's place: `%APPDATA%\TraceDraw` on Windows,
  `~/Library/Application Support/TraceDraw` on macOS,
  `$XDG_CONFIG_HOME/tracedraw` (or `~/.config/tracedraw`) elsewhere; the
  browser keeps them in its local storage.

Menus, lists and dialogs appear at once, without fading in.

