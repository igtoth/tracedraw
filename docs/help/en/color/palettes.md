# Color palettes

The color palette along the bottom of the window is the quickest way to color
objects. Below it, the **document palette** collects the colors your drawing
uses. You can open more palettes, make your own and edit them.

## Using the palette {#using}

| Action | Result |
|---|---|
| Click a color | fills the selection (with nothing selected, sets the fill of new objects) |
| Right-click a color | colors the outline of the selection |
| <kbd>Ctrl</kbd>+click a color | mixes a tenth of it into the selection's fill |
| Click and hold a color | shows lighter, darker, duller and brighter shades of it to choose from |
| Drag a color onto an object | fills the object, or colors its outline when dropped on the edge |
| Click the crossed-out well | removes the fill (right-click: the outline) |

The arrows at the ends scroll the palette, and the button at the right end
shows every color at once. Hover a color to see its name and values. The
palette menu button at the left end opens the palette menu.

The palette can be moved to another edge of the window, or made to float:
drag it by its dotted grip. With two or three rows, the window showing every
color opens clear of all the rows.

Every palette ticked in [[menu.window > menu.window.color_palettes]] or in the
Palette Manager shows in a strip of its own next to the default palette, with
**Close Palette** in its menu.

## The document palette {#document-palette}

Every color you apply to an object joins the document palette, which is saved
with the drawing. Its eyedropper adds the color under the pointer, and its
menu has **Automatically update** (to stop adding colors by themselves),
**Add from selection**, **Add from document**, **Delete color** and **Reset
palette** (keeps only the colors the drawing still uses). You can also drag
colors or objects onto it. The color used last has a white frame: a color
taken with the eyedropper that the palette already has gets the frame, so you
see where it is, and applying a color of the palette moves the frame to it.
To show or hide it, see
[Customization options](../customize/customization.md#color-palette).

## The Palettes panel {#palettes-docker}

The [[docker.palettes]] panel lists the palettes you have, with the number of
colors in each. Open it with
[[menu.window > menu.window.color_palettes > menu.window.palette_manager]].

![The Palettes panel](../../img/color-palettes.png)

- Tick a palette to show it under the drawing window.
- [[docker.edit]] opens it in the palette editor.
- [[menu.window.palette_open]] opens a palette file (`.tdpal`, `.gpl`,
  `.ase` or `.aco`).
- [[menu.window.palette_from_document]] and
  [[menu.window.palette_from_selection]] make a palette of the colors used in
  the drawing or in the selection.
- The document palette's colors show at the bottom; click one to fill the
  selection, right-click to color the outline.

## Editing a palette {#palette-editor}

[[menu.window > menu.window.color_palettes > menu.window.palette_editor]] opens
the palette editor.

![The Palette Editor](../../img/color-palettes-palette-editor.png)

1. Choose the [[dialog.palette]] to edit, or click [[dialog.new_palette]] to
   start one.
2. Type its [[dialog.name]].
3. Click a color to select it; [[dialog.add_color]] adds the current color and
   [[dialog.remove_color]] removes the selected one.
4. Click [[dialog.save_palette]] to save it as a file.
5. Click [[dialog.close]].

Built-in palettes cannot be edited; make a new palette, or edit a copy.
