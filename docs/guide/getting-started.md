# Getting started

This page takes you from installing TraceDraw to saving your first
drawing. On the way it names the parts of the window, so the other pages
of this guide make sense.

## Install TraceDraw

### On your computer

Every version of TraceDraw has a release with files for each system. To
install it:

1. Open the [latest release](https://github.com/igtoth/tracedraw/releases/latest).
2. Download the file for your system (`<version>` is the version
   number):

   | System | File |
   |---|---|
   | Windows (x64) | `tracedraw-<version>-windows-x86_64-setup.exe` (installer) or `tracedraw-<version>-windows-x86_64-portable.zip` |
   | macOS (Apple silicon) | `tracedraw-<version>-macos-arm64.dmg` |
   | macOS (Intel) | `tracedraw-<version>-macos-x86_64.dmg` |
   | Linux (x86_64) | the `.deb` package (`_amd64`), `tracedraw-<version>-linux-amd64.AppImage` or `tracedraw-<version>-linux-x86_64.tar.gz` |
   | Linux (arm64: Raspberry Pi 4/5 with a 64-bit system, ARM laptops) | the `.deb` package (`_arm64`), `tracedraw-<version>-linux-arm64.AppImage` or `tracedraw-<version>-linux-arm64.tar.gz` |

3. Install or unpack it as you would any program on your system.

The downloads are not code-signed yet:

- On Windows, if SmartScreen warns you, choose **More info > Run
  anyway**.
- On macOS, right-click the app and choose **Open** the first time.

Every download also contains `tracedraw-cli`, a command-line tool for
converting files (see [Customizing](customizing.md#scripts-and-automation)).

If you want to try newer work before a release, the **Nightly build**
pre-release on the releases page is refreshed with every change.

### In your browser

You can also [open TraceDraw in the browser](https://igtoth.github.io/tracedraw/)
with nothing to install. While it loads, the page shows a progress bar;
the editor starts when the bar is full.

The browser version is the same editor, with a few differences:

- Opening and importing use the browser's file picker; saving, exporting
  and printing download a file.
- Your settings are kept in that browser on that computer.
- Recent drawings are not listed, because uploaded files cannot be
  reopened.
- Fonts come with the page, and more are fetched the first time your
  text needs them.
- It suits a desktop browser with a mouse and keyboard; on a tablet, use
  the Touch workspace (see [Customizing](customizing.md#workspaces)).
  A phone screen is too small for editing.
- `Ctrl+N`, `Ctrl+T` and `Ctrl+W` stay with the browser, so use the
  **File** menu for New and Close.

The full list of differences is in
[the browser notes](../behavior/web.md).

## Start TraceDraw

When TraceDraw starts, a splash screen shows while fonts load. Then the
main window opens, maximized the first time, and later at the size you
left it.

To turn the splash screen off: **Tools > Options > TraceDraw > General**,
clear **Show the splash screen at start-up**.

If your settings ever get into a muddle, press `F8` while the splash
screen shows. Once the editor is up, TraceDraw asks whether to reset
every preference. Your drawings are never touched. See
[Customizing](customizing.md#reset-your-settings).

## The Welcome Screen

The Welcome Screen is the first tab. From it you can:

- create a **New Document**;
- open one of the drawings you opened or saved last, each shown with a
  thumbnail of its first page (a click opens it; **Clear list** forgets
  them);
- start from a template, read news and find learning material.

While only the Welcome Screen is open, the menu bar shows File, Tools,
Window and Help. To bring it back later, use the standard toolbar's
**Launch** button and choose **Welcome Screen**.

To choose what TraceDraw shows when it starts: **Tools > Options >
TraceDraw > General > On start-up**, then pick Welcome Screen, Start a
new document, or Open last edited document.

## Create a new drawing

To start a drawing:

1. Choose **File > New** (`Ctrl+N`), the **New** button on the standard
   toolbar, the **+** after the last tab, or **New Document** on the
   Welcome Screen.
2. In **Create a New Document**, set:
   - **Name** (`Untitled-1` and so on to start with);
   - **Preset**: TraceDraw default, Default CMYK, Default RGB or Web;
   - **Number of pages** (1 to 999);
   - **Primary color mode**: CMYK (for print) or RGB (for screens);
   - **Page size** (A4 to start with), or a width and height with your
     units, and portrait or landscape;
   - **Resolution** in dpi (300 to start with), used for effects that
     become pixels.
3. Press **OK** (or `Enter`).

The drawing opens in its own tab. The dialog remembers your values for
next time. If you tick **Do not show this dialog again**, new drawings
open at once with the last values you used.

Detail: [the New Document notes](../behavior/new-document.md).

## The parts of the window

From top to bottom and left to right:

- **Menu bar**: every command, grouped in menus such as File, Edit,
  View, Layout, Object, Effects, Text, Tools, Window and Help, with each
  command's shortcut shown beside it.
- **Standard toolbar**: New, Open (its arrow lists recent drawings),
  Save and Print; Cut, Copy and Paste; Undo and Redo (their arrows list
  the steps, so you can undo several at once); Import, Export and
  Publish to PDF; the zoom box; Show Rulers, Show Grid and Show
  Guidelines; Snap Off and Snap To; Options; and Launch (Font Manager,
  Welcome Screen, Color Management).
- **Property bar**: the settings for what you are doing right now. With
  nothing selected it shows the page size, orientation, units and nudge
  distance. With a selection it shows position, size, scale, rotation
  and mirror buttons, then the settings for that kind of object. Each
  tool adds its own fields. You can type sums such as `10+5` into number
  fields, and a value with another unit (`1 in`) is converted.
- **Document tabs**: one per open drawing; `*` marks unsaved changes.
- **Toolbox**, on the left: one button per group of tools. A small arrow
  at a button's corner means it holds a **flyout** of related tools. To
  open a flyout, click the arrow, right-click, or press and hold the
  button. The button then shows the tool you used last from that group.
  The **+** at the end of the toolbox shows or hides groups.
- **Rulers** along the top and left of the drawing window
  (`Alt+Shift+R` turns them on and off). Drag from a ruler to create a
  guideline.
- **Drawing window**: the page on a white desktop. At its bottom are the
  page navigator (first, previous, next, last page and insert page
  buttons), one tab per page, and the scrollbars. The **Navigator**
  button in the corner shows a thumbnail of the page to move around in.
- **Panels** (dockers), on the right: Hints, Properties and Objects to
  start with, as tabs down the side. Open others from the **Window**
  menu or their shortcuts. A click on the tab in front folds the panel
  area away.
- **Colour palette**, along the bottom: click a colour to fill the
  selection, right-click it for the outline. The **document palette**
  under the drawing window collects the colours your drawing uses.
- **Status bar**, at the very bottom: a hint for the active tool, what
  is selected and on which layer, and the fill and outline of the
  selection. Its gear button chooses what the first field shows.

You can move the toolbox, the palette, the standard toolbar, the
property bar and the status bar: drag a bar by its dotted grip to
another edge, or away from the edges to make it float. Right-click a
grip for **Lock Toolbars** and **Reset Bar Positions**. See
[the toolbar notes](../behavior/toolbars.md).

## Move around the page

- **Zoom**: turn the mouse wheel (zooms about the pointer), or use the
  zoom box on the standard toolbar. `Ctrl++` and `Ctrl+-` zoom one step.
- `F4` zooms to fit all objects, `Shift+F4` to the page, `Shift+F2` to
  the selection.
- The **Zoom** tool (`Z`, `F2`): click to zoom in, `Shift`+click or
  right-click to zoom out, drag a rectangle to zoom to it.
- **Pan**: hold the middle mouse button and drag with any tool, or use
  the **Pan** tool (`H`). `Ctrl`+wheel scrolls up and down, `Alt`+wheel
  sideways.
- `PgUp` and `PgDn` go to the previous and next page.

If you prefer the wheel to scroll: **Tools > Options > Tools**, set the
mouse wheel action to Scroll. Detail: [zoom and pan](../behavior/zoom-and-pan.md).

## Set the interface size

If the menus and buttons look too small or too large:

1. Choose **Tools > Options > Customization**.
2. On the **Appearance** page, set **Interface size**: 80 %, 90 %,
   100 % (the default), 110 %, 125 %, 150 %, 175 % or 200 %.

The size is relative to the scaling your system gives the monitor, and
it never changes how large your drawing is: zoom 100 % always shows the
real size. To make zoom 100 % match a real ruler held against the
screen, use **Tools > Options > TraceDraw > Display > Ruler
calibration**. See [the Options notes](../behavior/options.md#interface-size).

## Save your drawing

To save: **File > Save** (`Ctrl+S`). The first time, and with **File >
Save As** (`Ctrl+Shift+S`), the **Save Drawing** dialog opens:

1. Pick a folder (shortcuts to Home, Desktop, Documents, Downloads and
   Pictures are down the left).
2. Type a **File name**.
3. Choose **Save as type**: TraceDraw (`*.tdraw`), which keeps
   everything, or CDR files (`*.cdr`) with a **Version**.
4. Optionally fill in **Title**, **Subject** and a **Rating**.
5. Press **Save**.

TraceDraw also keeps auto-backups of unsaved work and offers them after a
crash. Both are explained in [Files](files.md).

## Where next

- Select, move and turn objects: [Selecting and transforming](selecting-and-transforming.md).
- Draw your first shapes: [Drawing](drawing.md).
