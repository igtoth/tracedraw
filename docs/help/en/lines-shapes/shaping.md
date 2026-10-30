# Combining shapes: weld, trim and intersect

Shaping builds new shapes from overlapping objects: join them into one, cut
one with another, or keep only where they overlap. The commands are in
[[menu.object > menu.object.shaping]], on the property bar when two or more
objects are selected, and in the [[docker.shaping]] panel.

## The target and the sources {#target}

The **last object you select is the target**: it gives the result its fill,
outline and transparency. The other selected objects are the **sources**. To
choose the target, select the sources first, then <kbd>Shift</kbd>+click the
target.

The result goes on top of the active layer and is selected. Each operation is
one undo step. When there is no result (for example two objects that do not
overlap), nothing changes and the status bar says so.

## The operations {#operations}

| Operation | Needs | Result |
|---|---|---|
| [[menu.object.weld]] | 2 objects | one object covering all of them |
| [[menu.object.trim]] | 2 objects | the target without what the sources cover |
| [[menu.object.intersect]] | 2 objects | only the area all of them share |
| [[menu.object.simplify]] | 1 object | each object without what the objects above it cover |
| [[menu.object.front_minus_back]] | 2 objects | the front object without the ones behind it |
| [[menu.object.back_minus_front]] | 2 objects | the back object without the ones in front of it |
| [[menu.object.boundary]] | 1 object | an outline around the whole selection |

From the menu and the property bar, [[menu.object.weld]] removes the
originals, [[menu.object.trim]] keeps the sources, [[menu.object.intersect]]
keeps all of them, and [[menu.object.boundary]] keeps them all.

### Open lines and pictures {#open-lines}

- **An open line cuts a shape.** [[menu.object.trim]] with an open line,
  polyline or curve as a source crossing a closed target cuts the target
  along it into separate pieces, each keeping the target's fill and outline.
  Several lines make more pieces; a line that ends inside the shape cuts
  nothing, and the status bar says so. [[menu.object.front_minus_back]] and
  [[menu.object.back_minus_front]] split along open lines the same way.
- **An open line as the target** of [[menu.object.weld]],
  [[menu.object.trim]] or [[menu.object.intersect]] is cut by the closed
  objects and stays open: Trim keeps the parts outside them, Intersect the
  part inside, and Weld adds the parts outside to the welded shape.
- An open line used as a source of Weld or Intersect on a closed object takes
  no part; to cut a shape along a line, use Trim or the Knife tool (see
  [Cutting and erasing](cut-and-erase.md#knife)).
- **A picture as the target** of Trim, Intersect or Weld is cropped to the
  result: its frame takes the new shape and the picture shows inside it.

Areas filled side by side, and the results of Weld, Trim and Intersect, meet
exactly where outlines cross, with no hairline gap between them.

## The Shaping panel {#shaping-docker}

Open the [[docker.shaping]] panel from
[[menu.object > menu.object.shaping > menu.object.shaping_docker]].

![The Shaping panel](../../img/lines-shapes-shaping.png)

1. Click one of the seven picture buttons to choose the operation. The panel
   shows its name, a picture and what it does.
2. For [[menu.object.weld]], [[menu.object.trim]] and
   [[menu.object.intersect]], choose what to keep:
   [[docker.leave_original_source]] and [[docker.leave_original_target]].
   Each operation remembers its own choice.
3. Select the objects, the target last.
4. Click the button named after the operation. It stays pale until enough
   objects are selected, and a line says how many are needed.

Related: [[menu.object.combine]] (<kbd>Ctrl+L</kbd>) makes several objects one
curve without changing their shapes, and [[menu.object.break_apart]] splits a
curve into its parts again. See
[Objects and layers](../objects/objects-and-layers.md#combine).
