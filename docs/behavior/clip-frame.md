# Clip frames

Object > Clip to Frame: objects placed inside a container that clips
them.

## Model

`ShapeKind::ClipFrame { frame, contents }`: the frame is any closed
object, kept whole with its own transform, fill and outline; the
contents keep their own page-space transforms. The frame's fill and
outline are drawn, the contents are clipped to the frame's path. The
contents are drawn over a copy of what lies beneath the frame, which
then takes the page's place inside the frame (with the frame's
opacity), so a merge mode inside a frame (Multiply on a photo) works on
the backdrop as it does outside one
(`tracedraw_render::band_tests::merge_modes_inside_a_clip_frame_reach_the_backdrop`).

The clip frame keeps the id of the object it was made from; the frame
inside it gets an id of its own (Place Inside Frame), so it can be
found, painted and reshaped, and Extract Contents gives it the clip
frame's id back. A fill or outline given to the clip frame (palette,
Properties, the outline fields) goes to its frame, never to the
contents (`for_group_deep`); the status bar and the Properties docker
show the frame's fill and outline (`mixed::leaves`). With the Shape tool
the clip frame's nodes are its frame's: the corner nodes of a rectangle
frame round its corners, an ellipse frame takes a pie or arc, a curve
frame is edited node by node, and the clip frame stays one
(`App::shape_tool_shapes`, `node_target`).

## Commands

- Place Inside Frame: with contents selected, click the frame; the
  contents keep their position. A frame that is a clip frame already
  takes them in over the contents it holds (`Command::PlaceInside`).
  Until the click the pointer is a thick black arrow pointing right on
  a white halo, its tip the hot spot; Order > In Front Of and Behind
  show it too while they wait for their object (#379,
  `ui::cursors::tests::tools_get_their_cursors`).
- Extract Contents: the contents become ordinary objects again; the frame
  stays (an empty frame shows a hatched preview).
- Edit Contents: takes the contents out of the frame to edit them; the
  rest of the page fades to a quarter of its strength while the frame
  and the contents are drawn in full (`App::render_focus`, the
  renderer's focus option with several objects, `CLIP_EDIT_VISIBILITY`).
  Finish Editing Contents (the bar's Finish) puts them back. The
  contents are every object on the page that was not there besides the
  frame when editing began (`App::clip_edit_contents`): a group inside
  ungrouped stays in as its members, drawn in full, and objects drawn or
  pasted while editing go in too; the members of an ungrouped group were
  left out and faded, and Finish emptied the frame (#537,
  `a_group_inside_a_clip_frame_being_edited_drags_and_ungroups`).
  While editing, only the contents can be picked: the faded objects and
  the frame are out of reach of a click, Alt+click, a marquee, Select
  All and Tab (`App::outside_clip_edit`). Undo takes back the steps of
  the edit; past them it undoes the step that began the edit, which
  leaves the edit with the frame and its contents as they were, so
  nothing done outside the frame is undone while the edit still shows.
  Redo of that step enters the edit again; undoing Finish goes back
  into the edit and redoing it leaves (#851,
  `clip_frame_edit_keeps_to_its_contents_and_its_own_undo`).
- Lock Contents to Frame (default on, stored as `clip_frame.unlocked` in
  the object data when off): with the lock on, moving, scaling or
  rotating the frame transforms the contents too. With it off, the frame
  moves alone and the contents stay where they are
  (`compensate_unlocked_clip_frames` undoes the transform on them).

## The clip frame bar

With one clip frame selected (Pick tool) a bar floats at the top left
of the drawing window (`ui/clip_bar.rs`): Edit (Edit Contents), Select
Contents (the contents become the selection, with their round handles),
Extract Contents, Lock Contents (pressed while the contents follow the
frame; see Lock Contents to Frame) and Fit Contents, a list of Center
Contents (the contents' centre on the frame's centre), Fit Contents
Proportionally (scaled alike to fit inside the frame, centred), Fill
Frame Proportionally (scaled alike to cover it) and Stretch Contents to
Fill (scaled to the frame's box), each one "Fit Contents" undo step
(`App::fit_clip_contents`, `ops2::fit_contents_transform`). While the
contents are edited the bar shows Finish, as the symmetry bar does
(`ops2::clip_frame_tests::contents_fit_the_frame_four_ways`).

## Dropping onto a frame

While the selection is moved over a clip frame that is not part of it,
the frame shows filled in the selection colour and a note under the
pointer says what the release does: drop here, outside the frame
(nothing held); hold W to add the objects to the frame (Place Inside
Frame, the objects where they were dropped); hold K to replace the
frame's contents with them (the old contents are deleted; one "Place
Inside Frame" undo step, `App::replace_clip_contents`). W and K pick
no tool during the move (`App::frame_under_pointer`, `drop_frame`,
`drop_keys`; `pick_drag_tests::dropping_on_a_clip_frame_with_w_or_k_puts_the_objects_in`).

## Picking

A click selects the clip frame (anywhere in its box). Ctrl+click on a
clip frame lying on a layer starts Edit Contents: the contents come out
of the frame with the object under the pointer selected (every content
when the click is off them, inside the frame's box), and while they are
edited Ctrl+click goes into a group among them as anywhere else (#982,
`pick_drag_tests::ctrl_click_on_a_clip_frame_edits_its_contents`).
Ctrl+Alt+click digs through the contents and then the clip frame (see
`pick-tool.md`).

## Opacity mask

Object > Clip to Frame > Opacity Mask (a check item) turns the selected clip
frames into opacity masks and back: the frame is no longer drawn and no
longer clips hard; its brightness (its fill, gradients and patterns
included, through the luminance of the colour) sets how much of the
contents shows, white fully, black not at all, nothing outside the
frame. Invert Mask swaps that (black shows). Place inside, extract, edit
in place and lock work as for any clip frame. One undo step each.

- Screen and bitmaps: the frame is drawn alone and its luminance masks
  the contents' layer (`Shape::opacity_mask`, `crates/render`).
- SVG: a `<mask>` holding the frame's fill (inverted through a
  `feColorMatrix` filter), on a group of the contents.
- PDF: the masked object goes in as a picture with its transparency
  (300 dpi, at most 40 million pixels); PDF/X-1a flattens it with what
  lies under it.

## Frame Type (object context menu)

- Create Empty Clip Frame: each selected object that is not a clip
  frame becomes one holding nothing, in place (turned or not), as Place
  Inside Frame with no contents does; one undo step. Pale when nothing
  selected can become one.
- No Frame: each selected empty clip frame becomes the object it was
  made from, with its id; one undo step. Pale without one.
- Create Empty Text Frame (one object selected): the object gives way to
  an empty paragraph text frame of its size (its bounds) in its place,
  in the font, size, style, alignment and hyphenation new text takes;
  one undo step. The frame stays selected (and stays when left empty)
  for the Text tool.

## Checks

- `tracedraw_core::command::tests::a_clip_frames_frame_is_found_and_painted`
- `pick_drag_tests::a_clip_frame_is_painted_and_reshaped_through_its_frame`

- Given a locked clip frame, when the frame is moved by (10, 0), then the
  contents move by (10, 0).
- Given an unlocked clip frame, when the frame is moved by (10, 0), then
  the contents' page bounds are unchanged.
- Given a `.cdr` bitmap with a crop outline smaller than the image, then
  the reader produces a clip frame of the bitmap inside the crop path
  (confirmed with the 2019 banner file).
- Given a clip frame saved as `.cdr` version 12 or later, then the file
  holds a clip container (the frame object naming a group of the
  document's clip contents list, as files of that format hold one) and
  the frame comes back with its look and its contents (#1222;
  `write::tests::clip_frames_go_out_as_clip_containers_from_version_12`,
  see `docs/cdr-format.md`, Writing).
- `render::tests::opacity_masks_show_the_contents_by_the_frame_brightness`,
  `svg::tests::opacity_masks_are_svg_masks`,
  `pdf::tests::opacity_masks_go_in_as_transparent_pictures`,
  `command::tests::only_clip_frames_take_an_opacity_mask`,
  `app::tests::clip_frames_turn_into_opacity_masks_and_back`.
- `frame_type::tests::empty_clip_frames_come_and_go_in_place`,
  `frame_type::tests::an_object_gives_way_to_an_empty_text_frame_of_its_size`.
