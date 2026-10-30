# Copying and cloning effects

Instead of setting up an effect again, you can copy it from an object that
already has it.

## Copying an effect {#copy}

1. Select the objects that should get the effect.
2. Choose [[menu.object > menu.object.copy_effect]] and the kind of effect:
   [[menu.object.copy_shadow_from]], [[menu.object.copy_transparency_from]],
   [[menu.object.copy_envelope_from]], [[menu.object.copy_perspective_from]],
   [[menu.object.copy_distortion_from]], [[menu.object.copy_extrude_from]],
   [[menu.object.copy_contour_from]], [[menu.object.copy_blend_from]],
   [[menu.object.copy_lens_from]], [[menu.object.copy_bevel_from]] or
   [[menu.object.copy_block_shadow_from]].
3. Click the object to copy from. A right-click cancels.

The effect is fitted to each object: an envelope or perspective is stretched
to its size, a distortion keeps its center at the same place within it. When
the object you click has no such effect, nothing changes and the status bar
says so. The copy is one undo step.

## Cloning an effect {#clone}

[[menu.object > menu.object.clone_effect]] copies the shadow
([[menu.object.clone_shadow_from]]) or the transparency
([[menu.object.clone_transparency_from]]) of another object, and keeps
following it: change the source and the clones change too, until you edit a
clone's own effect.

[[menu.edit > menu.edit.clone]] makes a clone of a whole object, and
[[menu.edit.copy_properties_from]] (<kbd>Ctrl+Shift+A</kbd>) copies the fill,
outline or text properties of another object: tick what to copy (outline pen,
outline color, fill, text properties), click **OK**, then click the object to
copy from.
