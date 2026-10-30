# Undoing, zooming and panning

You can undo any change, and look at your drawing at any zoom level and from
any position. Zooming and panning never change the drawing.

## Undoing and redoing {#undo}

- [[menu.edit > menu.edit.undo]] (<kbd>Ctrl+Z</kbd>) takes back the last
  change; [[menu.edit.redo]] (<kbd>Ctrl+Shift+Z</kbd>) does it again.
- The arrows beside Undo and Redo on the standard toolbar list the steps.
  Choose a step to undo or redo every change up to it.
- [[menu.edit.repeat]] applies the last command again, for example to the
  next object you select. A move, resize or rotation is repeated about
  the selected object's own centre, side or corner, so the object turns or
  grows where it is instead of moving away.

### The Undo panel {#undo-docker}

The [[docker.undo]] panel lists every change of the drawing, oldest first.
Click a step to take the drawing back to how it was after that step; click a
later step to go forward again. Open it from
[[menu.window > menu.window.dockers]].

![The Undo panel](../../img/get-started-undo-and-views-undo-docker.png)

## Zooming {#zoom}

- Turn the mouse wheel to zoom in or out about the pointer.
- Use the zoom box on the standard toolbar: type a level, or choose one from
  its list (10% to 1600%), or zoom to the selection, all objects, the page,
  the page width or the page height.
- [[menu.view > menu.view.zoom_in]] and [[menu.view.zoom_out]] zoom one
  step; [[menu.view.zoom_to_fit]] (<kbd>F4</kbd>) shows all objects,
  [[menu.view.zoom_to_page]] (<kbd>Shift+F4</kbd>) the page and
  [[menu.view.zoom_to_selected]] (<kbd>Shift+F2</kbd>) the selection.
- A trackpad pinch zooms, and on a touch screen two fingers pan and pinch
  with any tool.

### The Zoom tool {#zoom-tool}

Choose the [[tool.zoom]] tool (<kbd>Z</kbd> or <kbd>F2</kbd>), then:

- click to zoom in by two at that point;
- <kbd>Shift</kbd>+click or right-click to zoom out by two;
- drag a rectangle to zoom so the rectangle fills the window.

With <kbd>F2</kbd> the tool zooms once: after the click or the
rectangle, the tool you had before comes back. With <kbd>Z</kbd> it stays
until you choose another tool.

Double-click the tool's button to zoom to fit all objects.

## Panning {#pan}

- Hold the middle mouse button and drag, with any tool.
- Choose the [[tool.pan]] tool (<kbd>H</kbd>) and drag.
- <kbd>Ctrl</kbd>+wheel scrolls up and down, <kbd>Alt</kbd>+wheel sideways.
- Use the scroll bars, which cover the page and one page size around it.
- Hold the Navigator button in the corner between the scroll bars and move
  the pointer over the thumbnail of the page.
- <kbd>PgUp</kbd> and <kbd>PgDn</kbd> go to the previous and next page.

To make the wheel scroll instead of zoom, change the mouse wheel action on
the [[options.zoom_pan]] page of
[[menu.tools > menu.tools.options > menu.tools.options_tools]]. Then the
wheel scrolls, <kbd>Shift</kbd>+wheel scrolls sideways and
<kbd>Ctrl</kbd>+wheel zooms. See [Tool options](../customize/tool-options.md#zoom-pan).

## Views {#views}

The [[menu.view]] menu chooses how objects are drawn:
[[menu.view.wireframe]] shows outlines only, [[menu.view.normal]] and
[[menu.view.enhanced]] draw fills and effects, and [[menu.view.pixels]]
shows the drawing as the pixels of an export. [[menu.view.fullscreen_preview]]
shows the page alone on the screen; press any key to come back.
[[menu.view.preview_selected_only]] limits that preview to the selection.
The [[menu.view]] menu also shows or hides the rulers, grids, guidelines,
bleed and printable area; see
[Rulers, grids and guidelines](../pages-layout/guidelines-and-grid.md).
