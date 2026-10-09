# ClipFrame

Object > ClipFrame: objects placed inside a container that clips them.

## Model

`ShapeKind::ClipFrame { frame, contents }`: the frame is any closed
object, the contents keep their own page-space transforms. The frame's
fill and outline are drawn, the contents are clipped to the frame's
path.

## Commands

- Place Inside Frame: with contents selected, click the frame; the
  contents keep their position.
- Extract Contents: the contents become ordinary objects again; the frame
  stays (an empty frame shows a hatched preview).
- Edit ClipFrame: enters the clip; only the contents are editable, the
  rest of the page is dimmed and the frame is drawn as an outline. Finish
  Editing leaves it. While editing, a bar on the property bar shows Finish.
- Lock Contents to ClipFrame (default on, stored as
  `clip_frame.unlocked` in the object data when off): with the lock on,
  moving, scaling or rotating the frame transforms the contents too.
  With it off, the frame moves alone and the contents stay where they
  are (`compensate_unlocked_clip_frames` undoes the transform on them).
- Create Empty ClipFrame Frame, Frame Type > Text Frame:.

## Checks

- Given a locked ClipFrame, when the frame is moved by (10, 0), then the
  contents move by (10, 0).
- Given an unlocked ClipFrame, when the frame is moved by (10, 0), then
  the contents' page bounds are unchanged.
- Given a `.cdr` bitmap with a crop outline smaller than the image, then
  the reader produces a ClipFrame of the bitmap inside the crop path
  (confirmed with the 2019 banner file).
