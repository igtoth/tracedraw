# Placing objects inside a frame

A **clip frame** is a closed object that holds other objects and shows only
the parts of them that lie inside it, like a picture in a window. The frame
keeps its own fill and outline; the objects inside stay whole and can be
taken out again. The commands are in [[menu.object > menu.object.clip_frame]].

## To place objects inside a frame {#place-inside}

1. Select the objects to place inside.
2. Choose [[menu.object > menu.object.clip_frame > menu.object.clip_frame_place_inside]].
3. Click the closed object that will be the frame.

The objects keep their position. You can also drag the objects onto the frame
with the right mouse button and choose to place them inside; see
[Dragging with the right button](selecting.md#right-drag).

## Editing what is inside {#edit-contents}

- [[menu.object.clip_frame_edit]] enters the frame: only the objects inside
  can be edited, the rest of the page is dimmed and the frame shows as an
  outline. [[menu.object.clip_frame_finish]], or the button on the property
  bar, leaves it.
- [[menu.object.clip_frame_extract]] makes the objects inside ordinary
  objects again; the frame stays, empty.
- [[menu.object.clip_frame_lock]] (on at first): moving, scaling or rotating
  the frame takes the contents along. Turned off, the frame moves alone and
  the contents stay where they are on the page.

Double-clicking a frame with the [[tool.pick]] tool also enters it; see
[Focus mode](focus-mode.md).

## Opacity masks {#opacity-mask}

[[menu.object > menu.object.clip_frame > menu.object.clip_frame_opacity_mask]]
turns the selected clip frames into opacity masks: the frame is no longer
drawn, and its brightness decides how much of the contents shows: fully where
it is white, not at all where it is black, and nothing outside the frame.
Fill the frame with a gradient for a soft fade. [[menu.object.clip_frame_invert_mask]]
swaps that, so black shows. Choose the command again to turn the mask back
into a plain frame.

## Empty frames {#empty-frames}

Right-click an object and use its frame type commands:

- **Create empty clip frame** turns each selected object into an empty clip
  frame, ready to hold objects.
- **No frame** turns an empty clip frame back into the object it was.
- **Create empty text frame** replaces the object by an empty paragraph text
  frame of its size, ready for the [[tool.text]] tool.

Cropping a bitmap with the [[tool.crop]] tool or the [[tool.shape]] tool also
places it in a clip frame; see [Cropping, cutting and erasing](../lines-shapes/cut-and-erase.md#crop).
