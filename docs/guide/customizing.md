# Customizing

TraceDraw can be set up to suit the way you work: preferences, the
layout of the window, keyboard shortcuts, the interface language, and
scripts that do repetitive work for you.

Your preferences are saved as soon as they change, so they survive even
if TraceDraw does not close normally. On the desktop they live in a
settings file; in the browser, in that browser's storage on that
computer.

## Options

**Tools > Options** is a submenu with five dialogs: **TraceDraw**
(`Ctrl+J`), **Customization**, **Tools**, **Global** and **Workspaces**.
Changes show at once; **OK** keeps them, and **Cancel** (or **Esc**) puts
everything back. Highlights:

### TraceDraw

- **General**: what to show at start-up (the Welcome Screen, a new
  document, or the last edited document), the New Document dialog, the
  splash screen, and the undo levels (150).
- **Display**: proof colours, tooltips, node tracking, the mouse wheel
  (zoom or scroll) and **Ruler calibration**, so that zoom 100 % matches
  a real ruler held to the screen.
- **Edit**: the constrain angle (15 degrees) and the drawing precision.
- **Nodes and Handles**: node size, shape and colours.
- **Snapping**: what to snap to, the radius and the snapping modes.
- **Save**: backups of the original file, and auto-backup (every 20
  minutes); see [Files](files.md#auto-backup-and-recovery).
- **Text**: the keyboard text increment, the default font and size, and
  hyphenation.

### Customization

- **Appearance**: the interface size (see
  [Getting started](getting-started.md#set-the-interface-size)), the
  desktop colour round the page, the page border, and the Outline flyout
  in the toolbox.
- **Commands**: your own keyboard shortcuts (below).
- **Command Bars**: which bars show.
- **Color Palette**: the No Color well, what the right mouse button does
  on the palette, and the document palette.

### Tools

Settings for the Pick, Zoom and Pan, curve, shape and Eraser tools, such
as Freehand smoothing, how close you must be to join or close a curve,
rectangle corners, polygon points and table rows and columns.

### Global

- **General**: the interface language, the daily update check, letting
  AI agents on this computer edit the open drawing, and **Reset all
  settings...**.
- **File Locations**: where preferences and auto-backups are kept.
- **Printing**: what the Print dialog starts with.
- **File Formats**: what Export starts with, where imported objects go,
  and the `.cdr` version for new drawings.
- **Bitmap Tracing**: the Quick Trace preset and the defaults of every
  trace.
- **Warnings**: which messages to show.

**Layout > Document Options** holds the settings of the drawing itself:
page size, layout, background, bleed, rulers, grid and guidelines. Its
**Save as Default** keeps the grid, ruler and guideline settings for new
drawings, and **Tools > Save Settings as Default** keeps the default
properties of new objects.

Detail: [the Options notes](../behavior/options.md).

## Workspaces

A workspace is how the window is set up: which bars show and where, the
toolbox, shortcuts, palette options and the interface's look. To pick a
built-in one: **Window > Workspace**, then **Default**, **Lite**,
**Classic**, **Illustration**, **Page Layout** or **Touch**.

**Touch** is for touch screens: the interface is drawn 1.4 times larger,
and two fingers pan and pinch to zoom with any tool.

To keep your own setup, or move it to another computer:

1. Choose **Window > Workspace > Export Workspace...** (or the
   **Workspaces** page of Options, where you can tick which parts to
   include: keyboard shortcuts; toolbars, toolbox and status bar; colour
   palette options; appearance).
2. Save the `.tdws` file.
3. On the other computer, choose **Import Workspace...** and pick the
   file.

See [the workspace notes](../behavior/workspaces.md).

### Arrange the window

- Drag any bar by its dotted grip to another edge, or away from the
  edges to float it. Right-click a grip for **Lock Toolbars** and
  **Reset Bar Positions**.
- **Window > Toolbars** shows and hides bars, including the colour
  palette.
- The **+** at the end of the toolbox shows or hides tool groups, and
  **Reset toolbox** puts them back.
- Open panels from the **Window** menu or their shortcuts; close one
  with its **x**. Panels you close stay closed next time.

## Keyboard shortcuts

Every menu item that shows a shortcut is bound to it. The full list is
in [the shortcut notes](../behavior/shortcuts.md); a few to know:

| Keys | Action |
|---|---|
| `Ctrl+N`, `Ctrl+O`, `Ctrl+S`, `Ctrl+Shift+S` | New, Open, Save, Save As |
| `Ctrl+I`, `Ctrl+E`, `Ctrl+P` | Import, Export, Print |
| `Ctrl+Z`, `Ctrl+Shift+Z`, `Ctrl+R` | Undo, Redo, Repeat |
| `Ctrl+D` | Duplicate |
| `Ctrl+G`, `Ctrl+U` | Group, Ungroup |
| `Ctrl+Q` | Convert to curves |
| `Ctrl+Space` | the Pick tool, from any tool |
| `Space` | the Pick tool; on the Pick tool, back to the tool before it |
| `F4`, `Shift+F4` | Zoom to fit, zoom to page |
| `Alt+Q` | Snapping off and on |
| `Alt+Enter` | Properties panel |
| `Ctrl+J` | Options |

Pressing a panel's shortcut while that panel is already in front hides
the panel column. **Esc** closes any dialog without applying it.

### Your own shortcuts

1. Open **Tools > Options > Customization > Commands**. It lists every
   menu command as "Menu > Submenu > Command", with its shortcut.
2. Type in the search field to find a command.
3. Click its **Set shortcut** button, then press the keys you want (with
   **Ctrl**, **Alt** and **Shift** as needed). **Esc** stops waiting.
4. If another command had those keys, the page tells you which one lost
   them.

Your shortcut shows in the menus in place of the built-in one. The **x**
beside a shortcut removes it, and **Reset to defaults** removes all of
yours. Tool keys (single letters such as `Z` for Zoom) are listed under
**Tools** on the same page.

## Languages

The interface comes in 18 languages: English, Chinese (Simplified),
Hindi, Spanish, French, Arabic (with a right-to-left layout), Bengali,
Russian, Portuguese (Brazil), Indonesian, German, Japanese, Chinese
(Traditional), Italian, Dutch, Polish, Turkish and Czech.

To switch: **Tools > Options > Global > General**, choose the language.
It changes at once. Shortcuts are the same in every language.

## Reset your settings

To put every preference back to how TraceDraw comes:

- **Tools > Options > Global > Reset all settings...**, or
- press `F8` while the splash screen shows.

TraceDraw asks first. Your recent drawings, the last folder and the
window size are kept, and your drawings are never touched. See
[the start-up notes](../behavior/about-and-startup.md#back-to-the-defaults).

## Scripts and automation

Every menu item, tool and panel in TraceDraw works through one set of
commands with undo, so scripts can do anything a click can.

### The Scripts panel

The **Scripts** panel runs JavaScript against an object model of
documents, pages, layers and shapes. You can run a script, load and save
scripts, and **record a macro**: what you do is written down as a script
you can run again. An example is in the
[project README](../../README.md#made-to-be-automated).

A script can also add a file format: one that defines `TraceDrawFilter`
with a name, its extensions and an `Import` or `Export` function becomes
a filter with **Install as Filter**, and its format then appears in
**File > Import** and **File > Export**. See
[the script filter notes](../behavior/script-filters.md).

To find out what a script does, click in the margin by a line number to
set a breakpoint and press **Debug**: the script stops there, and you can
step through it, look at its variables with **Evaluate**, continue or
stop. See [the debugger notes](../behavior/script-debugger.md).

### AI Drawing

**AI Generate > Draw with words** (the docker's fourth mode) turns a request in words ("a red bicycle with
blue wheels") into a drawing. It uses an AI service you set up yourself,
with your own key: a Messages API service, any chat completions service,
or a model server on your own computer. The service writes a script,
which runs as one undo step without access to your files. TraceDraw has
no AI service or account of its own, and nothing is sent until you press
**Draw**. See [the AI Drawing notes](../behavior/ai-drawing.md).

### The command line

Every download includes `tracedraw-cli`. For example, to convert between
any format TraceDraw opens and any it writes:

```sh
tracedraw-cli convert poster.cdr poster.pdf
TRACEDRAW_DPI=300 tracedraw-cli convert poster.tdraw poster.png
```

A file opened while TraceDraw runs (from the file manager or the command
line) opens in a tab of the running window; `tracedraw --new-instance
file.cdr` opens a window of its own.

### AI agents

`tracedraw --mcp` serves the editor to AI agents over the Model Context
Protocol. To let an agent on your computer edit the drawing open in your
window, turn on **Tools > Options > Global > General > Let AI agents on
this computer edit the open drawing** (off to start with). See
[the MCP notes](../behavior/mcp.md).

## Where next

- Back to the start: [the guide's contents](README.md).
- What works and what is still to come: [feature status](../features.md).
