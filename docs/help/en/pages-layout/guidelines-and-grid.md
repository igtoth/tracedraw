# Rulers, grids and guidelines

Rulers, grids and guidelines help you place objects precisely. None of them
print.

## Rulers {#rulers}

The rulers run along the top and left of the drawing window, in the drawing
units; [[menu.view > menu.view.rulers]] shows or hides them. A dashed line on
each ruler follows the pointer.

- Drag from the corner where the rulers meet to move the **origin**, the
  point the rulers count from. Positions on the property bar, the status bar
  and in the panels count from it too.
- Drag from a ruler to add a guideline.
- Double-click a ruler to open its settings; see
  [Document options](document-options.md#rulers), where you also set the
  units and the drawing scale.

## The document grid {#grid}

[[menu.view > menu.view.grid > menu.view.document_grid]] shows a grid of lines
or dots over the page, behind the objects; [[menu.view.snap_document_grid]]
in [[menu.view.snap_to]] makes objects snap to it. Its spacing (10 mm at
first) is set in [Document options](document-options.md#grid). When lines
would be too close, only every second, fifth or tenth line is drawn.

The **baseline grid** ([[menu.view.baseline_grid]]) draws lines across the
page like a ruled notebook, for lining up text; the **pixel grid**
([[menu.view.pixel_grid]]) shows the pixels of the drawing in the
[[menu.view.pixels]] view from 800% zoom.

## Guidelines {#guidelines}

Guidelines are lines across the drawing window that objects can snap to.

- Drag from the top ruler for a horizontal guideline, from the left ruler for
  a vertical one.
- With the [[tool.pick]] tool, click a guideline to select it
  (<kbd>Shift</kbd>+click adds), and drag it to move it. Drop it outside the
  drawing window, or press <kbd>Delete</kbd>, to delete it.
- Click the selected guideline again to show its rotation handles, and drag
  them to turn it.
- Right-click a guideline to delete, lock or unlock it, or open its
  properties. Locked guidelines can be selected but not moved or deleted.
- [[menu.edit > menu.edit.select_all > menu.edit.select_all_guidelines]]
  selects every guideline of the page, and [[menu.view.guidelines]] shows or
  hides them.
- With guidelines selected (and no objects), [[menu.edit.copy]] and
  [[menu.edit.paste]] make copies in the same place, and
  [[menu.edit.duplicate]] makes copies moved by the duplicate offset (set in
  [[menu.layout > menu.layout.document_options]], General). The
  Transformations panel moves, rotates and scales selected guidelines as it
  does objects.
- Guidelines are saved with the drawing in `.tdraw` files. `.cdr` files do
  not keep them: saving one says so on the status bar.

## The Guidelines panel {#guidelines-docker}

The [[docker.guidelines]] panel places guidelines by their coordinates. Open
it from [[menu.window > menu.window.dockers]].

![The Guidelines panel](../../img/pages-layout-guidelines-and-grid-guidelines-docker.png)

1. [[guides.show]] and [[guides.snap]] turn guidelines on and snapping to them.
2. Choose the [[guides.type]]: horizontal, vertical or angled.
3. Type the position ([[guides.x]], [[guides.y]], and the [[guides.angle]] for
   an angled one), counted from the ruler origin.
4. Click [[guides.add]] to add a guideline, or select one in the list and
   click [[guides.modify]] to move it.

The list shows the page's guidelines of that type (<kbd>Ctrl</kbd>+click to
select several). [[guides.delete]], [[guides.lock]] and [[guides.unlock]] act
on the selected ones, and [[guides.color]] and [[guides.style]]
([[guides.style_solid]], [[guides.style_dashed]], [[guides.style_dotted]] or
[[guides.style_dash_dot]]) change their look.

Ready-made sets of guidelines (margins, columns, grids) are on the
[[options.guidelines]] page of the [document options](document-options.md#guidelines).
