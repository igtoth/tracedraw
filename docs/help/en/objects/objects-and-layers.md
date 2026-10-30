# Organizing objects and layers

The [[docker.objects]] panel shows how the current page is organized: its
layers, the objects on each layer, the groups and the master layers that
appear on every page. Use it to select, name, hide, lock and order objects
and layers. Objects higher in the list are drawn in front.

![The Objects panel](../../img/objects-objects-and-layers.png)

## The Objects panel {#objects-docker}

The panel lists the page with its layers, topmost first, then the
[[docker.master_page]] with the master layers, the guidelines
([[docker.guides]]) and the [[menu.view.document_grid]]. Each object shows a
swatch of its fill framed in its outline color, and its name, or what it is
when it has none ("Rectangle", "Curve with 8 nodes").

- Click an object to select it; <kbd>Shift</kbd>+click adds it to the
  selection. Its layer becomes the active layer.
- Click the arrow of a group to see the objects inside it.
- Each layer has three buttons: [[docker.show_hide]], [[docker.print_layer]]
  and [[docker.lock_layer]]. A hidden or locked layer cannot be edited. The
  padlock of [[docker.guides]] locks all the guidelines of the page.
- Hidden objects show their name dimmed.

## Layers {#layers}

A layer is a sheet on which objects lie. The **active layer**, shown in bold
with a colored bar, is where new objects go when you draw, paste, duplicate
or import. Click a layer to make it active.

- [[docker.new_layer]], at the bottom of the panel, adds a layer on top and
  makes it active.
- Right-click a layer for [[docker.rename]], [[docker.move_up]],
  [[docker.move_down]] and [[docker.delete_layer]]. A page always keeps one
  layer.
- [[docker.delete]] at the bottom of the panel deletes the selected objects,
  or the active layer when nothing is selected.

### Master layers {#master-layers}

Objects on a **master layer** appear on every page, under the page's own
layers: use them for a logo, a header or page numbers. Click the
[[docker.new_master_layer]] button to add one for all pages; right-click it
to add one for odd or even pages only. A master layer's menu has
[[docker.show_on]] to choose [[options.scope_all]], [[options.scope_odd]] or
[[options.scope_even]]. You can select and edit master objects from any page
they show on.

## Renaming objects and layers {#rename}

Double-click a layer or an object in the panel, or choose [[docker.rename]]
from its menu, to open the [[dialog.rename_layer]] or [[dialog.rename_object]]
dialog box. Type the new name and click [[dialog.ok]]. An object with an empty
name shows what it is again. Double-click the page row to rename the page.

![The Rename Object dialog box](../../img/objects-objects-and-layers-rename.png)

Names help you find objects in the panel and in
[Find and replace](find-replace.md), and are kept in exported SVG files.

## Changing the order {#order}

[[menu.object > menu.object.order]] moves the selected objects:
[[menu.object.to_front_of_page]], [[menu.object.to_back_of_page]],
[[menu.object.to_front_of_layer]], [[menu.object.to_back_of_layer]],
[[menu.object.forward_one]], [[menu.object.back_one]],
[[menu.object.in_front_of]], [[menu.object.behind]] (click the object to go
in front of or behind) and [[menu.object.reverse_order]].

## Grouping and combining {#combine}

- [[menu.object > menu.object.group > menu.object.group_group]]
  (<kbd>Ctrl+G</kbd>) makes the selection one group that moves and resizes
  together. [[menu.object.group_ungroup]] (<kbd>Ctrl+U</kbd>) splits it again,
  and [[menu.object.group_ungroup_all]] also splits the groups inside it. To
  edit an object inside a group without ungrouping, <kbd>Ctrl</kbd>+click it.
- [[menu.object.combine]] (<kbd>Ctrl+L</kbd>) makes the selected objects one
  curve of several parts, with one fill and outline; where the parts overlap,
  holes appear. [[menu.object.break_apart]] (<kbd>Ctrl+K</kbd>) splits a curve
  into its parts.
- [[menu.object.convert_to_curves]] (<kbd>Ctrl+Q</kbd>) turns a rectangle,
  ellipse, polygon or text into a curve you can edit node by node.

## Locking and hiding objects {#lock-hide}

- [[menu.object > menu.object.lock > menu.object.lock_object]] keeps the
  selected objects from being selected or changed;
  [[menu.object.unlock_object]] and [[menu.object.unlock_all]] release them.
- [[menu.object > menu.object.hide > menu.object.hide_object]] hides the
  selected objects; [[menu.object.show_all]] shows them again.

To keep objects out of the way while you work on others, see also
[Focus mode](focus-mode.md).
