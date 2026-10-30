# The workspace

The TraceDraw window holds your drawings and everything you need to work on
them. This page names its parts, so the other pages of this Help make sense.

## The parts of the window {#parts}

From top to bottom and from left to right:

- The **menu bar** holds every command, in the menus [[menu.file]],
  [[menu.edit]], [[menu.view]], [[menu.layout]], [[menu.object]],
  [[menu.effects]], [[menu.bitmaps]], [[menu.text]], [[menu.table]],
  [[menu.tools]], [[menu.window]] and [[menu.help]]. Each command shows its
  shortcut beside it. With no drawing open, only [[menu.file]],
  [[menu.tools]], [[menu.window]] and [[menu.help]] are shown. The
  [Menu commands](../reference/menus.md) page lists every command.
- The **standard toolbar** holds the common commands (see below).
- The **property bar** shows the settings of what you are doing now.
- The **document tabs** show one tab per open drawing.
- The **toolbox**, on the left, holds the tools.
- The **rulers** run along the top and the left of the drawing window.
- The **drawing window** shows the page on the desktop around it.
- The **panels** sit on the right.
- The **color palette** runs along the bottom.
- The **status bar**, at the very bottom, describes the active tool and the
  selection.

## The standard toolbar {#standard-toolbar}

The standard toolbar holds, in groups:

1. New, Open (its arrow lists the drawings you used last), Save and Print.
2. Cut, Copy and Paste.
3. Undo and Redo. Their arrows list the steps, so you can undo or redo
   several at once by choosing a step.
4. Import, Export and Publish to PDF.
5. The zoom box: type a zoom level and press <kbd>Enter</kbd>, or open its
   list for zoom to selected, to all objects, to the page, to the page
   width or height, and levels from 10% to 1600%. Then full-screen preview.
6. Show rulers, show grid and show guidelines, drawn pressed while on.
7. Snap off (<kbd>Alt+Q</kbd>) and the list of what to snap to.
8. Options, which opens the list of option pages.
9. Launch: Font Manager, Welcome Screen and Color Management.

Hover a button to see its name, its shortcut and what it does.

## The property bar {#property-bar}

The property bar changes with what you do. With nothing selected and the
[[tool.pick]] tool active, it shows the page size and orientation, the
drawing units and the nudge distance. With objects selected, it shows their
position, size, scale, rotation and mirror buttons, then the settings of
that kind of object. Each tool adds its own settings, explained on the page
of the tool.

In number fields you can type a sum such as `10+5`, or a value in another
unit such as `1 in`, which is converted to the drawing units.

## The toolbox and flyouts {#toolbox}

The toolbox has one button per group of tools. A button with a small arrow
at its corner holds a **flyout**, a list of related tools. To open a
flyout, click the arrow, right-click the button, or press and hold it. The
button then shows the tool you used last from that group. Hover a tool to
see its name, its shortcut and what it does.

Double-clicking some buttons has a shortcut action: [[tool.pick]] selects
all objects, [[tool.zoom]] zooms to fit all objects and [[tool.rectangle]]
adds a rectangle the size of the page.

The **+** button at the end of the toolbox lists the groups with a check box
to show or hide each one, and resets the toolbox. The
[Toolbox](../reference/toolbox.md) page lists every tool.

## Panels {#panels}

Panels hold settings you keep open while you work, such as
[[docker.properties]], [[docker.objects]] and [[docker.hints]], which are
open at first. Open panels are tabs down the right side; click a tab to
bring its panel to the front, or click the tab in front to fold the panels
away. Open other panels from [[menu.window > menu.window.dockers]] or with
their shortcuts; the **+** under the tabs adds one. Each panel has collapse
and close buttons in its title bar. The [Panels](../reference/dockers.md)
page lists every panel.

## The drawing window {#drawing-window}

The page sits on the desktop, the area around it, where you can also keep
objects. Below the drawing are:

- the **page navigator**: insert a page before, first page, previous page,
  the page number, next page, last page and insert a page after;
- one **tab per page** (click to go to a page, double-click to rename it);
- the horizontal scroll bar. Drag the dotted splitter between the page tabs
  and the scroll bar to share the space between them.

The **Navigator** button in the corner between the scroll bars shows a
thumbnail of the page while you hold it; move the pointer over the thumbnail
to move the view. Double-clicking the page border with the [[tool.pick]]
tool opens the page size options.

## The color palette {#color-palette}

Click a color of the palette to fill the selected objects; right-click it to
color their outline. The document palette under the drawing window collects
the colors your drawing uses. See [Color palettes](../color/palettes.md).

## The status bar {#status-bar}

The status bar shows a hint for the active tool, what is selected and on
which layer, and the fill and outline of the selection. Double-click the
fill or outline part to change them. The [Status bar](../reference/status-bar.md)
page explains each part.

## Arranging the bars {#arranging}

You can move the toolbox, the color palette, the standard toolbar, the
property bar and the status bar: drag a bar by its dotted grip to another
edge of the window, or away from the edges to make it float. Right-click a
grip to lock the bars or reset their positions. To show or hide a bar, use
[[menu.window > menu.window.toolbars]]. You can also save the whole
arrangement as a workspace; see [Workspaces](../customize/workspaces.md).

## Working with several drawings {#several-drawings}

Each open drawing has its own tab above the rulers, with its name and a
`*` while it has unsaved changes. The tab after the last one creates a new
drawing. Each drawing keeps its own pages, undo history, selection, zoom and
units.

[[menu.window > menu.window.cascade]], [[menu.window.tile_horizontally]] and
[[menu.window.tile_vertically]] show every drawing in a window of its own;
[[menu.window.combine_windows]] brings the tabs back.
[[menu.window.new_window]] opens a second window of the same drawing, for
example to see two pages or two zoom levels at once; an edit in one window
shows in all of them.

## The interface size {#interface-size}

If menus and buttons look too small or too large, choose
[[menu.tools > menu.tools.options > menu.tools.options_customization]] and set
the interface size on the [[options.appearance]] page. The interface size
never changes the size of your drawing. See
[Customization options](../customize/customization.md#appearance).
