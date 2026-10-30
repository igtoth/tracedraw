# Finding and replacing by properties

The [[find.properties]] mode of the [[docker.find_replace]] panel finds objects
by how they look and what they are: every object filled with one red, every
outline 0.2 mm wide, every text in a font you no longer want, every bitmap.
It then selects them, or changes their fill, outline or font all at once.

## Finding objects {#find}

1. Choose [[menu.edit > menu.edit.find_and_replace]] (<kbd>Ctrl+F</kbd>) and
   click [[find.properties]].
2. In [[find.look_in]], choose [[find.this_page]] (the default) or
   [[find.whole_drawing]] (every page, then the master layers).
3. Under [[find.find_objects_with]], tick what to look for. An object is
   found when it has everything ticked:
   - [[find.object_type]] (off; Rectangle at first): only objects of one
     kind: [[find.type.rectangle]], [[find.type.ellipse]],
     [[find.type.polygon]], [[find.type.curve]],
     [[find.type.artistic_text]], [[find.type.paragraph_text]],
     [[find.type.group]], [[find.type.bitmap]], [[find.type.clip_frame]],
     [[find.type.table]] or [[find.type.symbol]].
   - [[find.fill]] (on; black at first): objects filled with this color, as
     a uniform fill or in a gradient or mesh fill, or [[find.none]] for
     objects without a fill.
   - [[find.outline]] (off): objects whose outline has this color, or
     [[find.none]] for objects without an outline.
   - [[find.outline_width]] (off; 0.2 mm at first, in the outline unit):
     objects whose outline has this width.
   - [[find.font]] (off): text that uses this font family, in any mix of
     capitals and small letters.
4. To look for what an object has, select it and click
   [[find.from_selection]]: its fill color (or none), outline color (or
   none), outline width and font fill the fields, ticked where it makes sense
   (the width and the type stay unticked). [[find.reset]] clears everything
   back to the defaults.
5. Click a button:
   - [[find.previous]] or [[dialog.find_next]] selects the objects found one
     at a time, going round at the end. With [[find.whole_drawing]], the page
     of each object comes into view.
   - [[dialog.find_all]] selects every object found on the current page. The
     status bar says how many were found in all and how many on this page.

The panel shows the number of objects found.

## Replacing properties {#replace}

1. Find the objects as above.
2. Under [[find.replace_with]], tick what to change and set the new value:
   - [[find.fill]]: when a fill color is looked for, that color is replaced
     by the new one wherever it is used, in gradients and mesh fills too;
     otherwise the objects get a uniform fill of the new color.
   - [[find.outline]]: when an outline color is looked for, outlines of that
     color take the new one; otherwise every outline does. Objects without
     an outline stay without one.
   - [[find.outline_width]]: every outline of the objects found takes the
     new width.
   - [[find.font]]: when a font is looked for, text in that font takes the new
     one; otherwise all the text of the objects found does.
3. Click [[dialog.replace_all]].

All the changes undo in one step. The status bar says in how many objects
something changed. Locked objects, and objects on locked layers, are found
but left as they are.

## How objects are compared {#compare}

- Colors count as the same when they are in the same model with every value
  within half a percent, or, in different models, when they show as the
  same color on screen. A spot color matches the same ink at the same tint,
  and the registration color matches only itself.
- Widths count as the same within half a thousandth of a millimetre.
- In groups and clip frames, the objects inside are checked at any depth, and
  the group is found when one of them matches. With only
  [[find.object_type]] ticked, the group's own type counts. Only objects at
  the top level are selected, since those are what you can select.

To find text or names instead, see
[Finding and replacing](find-replace.md).
