# Crop, Knife, Segment delete, Eraser

## Crop

- A drag draws the crop area. It stays on the page until it is applied
  or cleared: the drawing outside it is dimmed, its edge is dashed and
  it has eight handles (corners and side middles). A drag inside moves
  it, a drag on a handle (within 6 pixels) resizes it (past the
  opposite side it turns round), a drag outside draws a new one. Points
  snap as drawing does.
- Applied by a double-click inside it, Enter or the bar's Crop button;
  cleared by Escape (which then does nothing else), a click outside it,
  Clear Crop Area, or another tool. An area smaller than 0.1 mm is
  dropped.
- The objects cropped are the selection, or with nothing selected every
  unlocked object on the page's unlocked, visible layers (`crop_tool.rs`):
  - wholly inside: left as they are; wholly outside: deleted;
  - closed outlines (a subpath that ends where it started counts) are
    intersected with the area; open curves are cut where they cross its
    edge and the pieces inside are kept, still open; text becomes its
    outlines first; the result is a curve in the object's own space,
    with its fill, outline and effects;
  - bitmaps, tables and symbols go into a clip frame of the area (no
    fill or outline), so their content stays whole and can be taken
    out again (Extract Contents);
  - groups are cropped member by member (a group left empty is
    deleted); a clip frame by its frame.
- One undo step; the deleted objects leave the selection.
- The bar (with an area): X and Y of the area's point at the object
  origin, W and H (sized about that point), Crop, Clear Crop Area.
- Not done: turning the crop area.

## Knife

- Two-point line (first choice): drag a straight cut; it runs on past
  both ends across the objects. Freehand: drag a line of any shape; the
  cut follows it as drawn.
- The objects cut are the selection, or with nothing selected every
  visible object on the page's unlocked layers; bitmaps, groups and text
  are not cut (`knife.rs`).
- Closed objects lose a band along the cut: Split (first choice) takes a
  hair (0.01 mm), so the pieces meet; Gap takes the width beside it.
  Each separate piece becomes an object of its own (Keep as one object:
  the pieces of each object stay one object of several parts); a cut
  that leaves one piece changes nothing. Open curves are split where the
  cut crosses them and stay open.
- Pieces keep the fill, outline and opacity; the first keeps the
  object's place. The pieces are selected; one undo step.
- The bar: Two-point line, Freehand; Split, Gap and its width; Keep as
  one object; kept in the preferences (`tools.knife`).

## Segment delete

- A virtual segment is the stretch of an outline between two places
  where it crosses another outline or itself (or an open end); a
  closed subpath that nothing crosses is one virtual segment.
- Crossings count against every visible object of the page's unlocked
  layers (flattened to 0.02 mm), the object's other subpaths and its
  own other segments, the neighbouring ones included anywhere but at
  the node they share (within 0.05 mm of it), and a cubic segment
  crossing itself in a loop (`segment_delete.rs`, `self_crossings`:
  found between 48 chords of the segment and refined on the curve), so
  a single curve drawn over itself loses only the stretch clicked
  (`segment_delete::tests::a_curve_crossing_itself_loses_only_the_stretch_clicked`).
  Each cutter reaches 0.001 mm past its ends, so a piece left by an
  earlier cut, which ends on the outline it crossed, still cuts it
  (`segment_delete::tests::a_cutter_ending_a_hair_short_of_the_outline_still_cuts`).
  Each crossing found against a flattened outline is then moved onto the
  true crossing of the two curves (Newton steps), so the end a cut leaves
  lies on the outline it crossed and the next cut finds that crossing
  again: a curve whose tail comes back across an earlier segment loses
  the start, then the tail, and keeps the loop between
  (`segment_delete::tests::a_tail_crossing_a_cut_curve_loses_only_itself`, #354).
- Click (within 5 screen pixels; the nearest outline, the topmost of
  equals): its virtual segment under the pointer is removed. Drag: every
  virtual segment the rectangle touches is removed whole, on all objects
  at once: a stretch with any part inside the rectangle (an end inside,
  or the curve crossing one of its sides) goes as far as the crossings
  at its ends, and a closed outline nothing crosses goes whole, though
  it reaches past the rectangle; what it does not reach stays (#528,
  `segment_delete::tests::a_drag_removes_every_stretch_it_touches`).
  Earlier only the stretches lying wholly inside went, so a box drawn
  across an ornament removed nothing. A level or upright drag (a
  rectangle of no height or width) takes what it crosses; a curve through
  a corner of the rectangle counts (`segment_delete::tests::what_a_box_touches`).
  A click whose pointer slipped into a drag of under 8 pixels each way
  and whose box reaches nothing deletes as a click at the press,
  whichever way the pointer slipped
  (`segment_delete::tests::a_click_that_slips_into_a_little_drag_still_deletes`).
- Only the objects whose box meets the cut object's box are searched for
  crossings, their outlines worked out once per click or drag: a drag
  across 300 objects took about a third of a second in a debug build and
  takes a tenth of that.
- Objects inside ordinary groups are cut as objects on their own (their
  transforms composed with the groups'), for both the click and the
  drag, and cut the others; a group left empty goes. Linked groups
  (text, pictures, clip frames) are not cut (#427,
  `segment_delete::tests::curves_inside_a_group_are_cut`). A drag looks
  at every object whose box meets the rectangle, a level or upright line
  (a box of no height or width) included, and takes the stretches of
  theirs the rectangle touches.
- Freehand (#1150): the property bar's Box and Freehand choose what a
  drag draws (Box at first, kept with the tool settings); Alt held at the
  press takes the other one for that drag. A freehand stroke (points
  kept a screen pixel apart, drawn as a black line on a white one)
  removes every virtual segment it crosses or touches, whole, as far as
  the crossings at its ends, on all objects at once, one undo step;
  stretches it passes beside stay, so a stroke across a crowded spot
  takes only what it goes through, where the box would take everything
  in it (`segment_delete::tests::a_freehand_stroke_removes_the_stretches_it_crosses`,
  `pick_drag_tests::segment_delete_drags_a_freehand_stroke_with_alt_or_the_bar`).
  Only the objects whose box meets the stroke's box are looked at. A
  click that slipped into a stroke of under 8 pixels that crosses
  nothing deletes as a click at the press. Some Linux desktops move the
  window on Alt+drag; there, the bar's Freehand does the same without
  Alt.
- Curves, rectangles, ellipses and polygons can lose segments; they
  become curves. A closed subpath that loses a stretch opens (what is
  left of it runs on over its start as one piece); an object with
  nothing left is deleted. One undo step.

## Eraser (X)

- Thickness: default 6.35 mm (property bar, in the document units).
  Nib: round (default) or square.
- Drag: the band swept by the nib (`shaping::stroke_band`, round caps
  and joins for the round nib, square caps and mitre joins for the
  square one) is subtracted from the selected objects, or from the
  object under the start point when nothing is selected. Objects become
  curves; an object that vanishes completely is deleted. Objects whose
  bounds the band does not touch are left alone.
- Click: erases a dot of the nib's size.
- Double-click an object: deletes it whole.
- Bitmaps, groups and clip frames are not erased (erasing bitmaps by
  masking is not implemented).

## Checks

- `segment_delete::tests::a_click_removes_the_stretch_between_crossings`
- `segment_delete::tests::closed_outlines_open_where_they_lose_a_stretch`
- `segment_delete::tests::a_drag_removes_every_stretch_it_touches`
- `segment_delete::tests::a_drag_between_crossings_takes_only_that_stretch`
- `segment_delete::tests::what_a_box_touches`
- `segment_delete::tests::a_click_that_slips_into_a_little_drag_still_deletes`
- `segment_delete::tests::pieces_and_stretches`
- `segment_delete::tests::a_curve_crossing_itself_loses_only_the_stretch_clicked`
- `crop_tool::tests::the_area_waits_and_moves_until_it_is_applied`
- `crop_tool::tests::open_curves_groups_bitmaps_and_text`
- `crop_tool::tests::handles_and_resizing_stay_sane`
- `ui::crop_bar::tests::the_bar_and_the_area_draw`
- `knife::tests::straight_cuts_split_closed_objects`
- `knife::tests::freehand_cuts_and_open_curves`

- Given a 40 x 20 mm rectangle and a 4 mm vertical band through its
  middle, when erased, then the object is a path with two subpaths and
  the same bounds.
- Given a dot 100 mm away, then the object is unchanged.
- Given a 100 mm band across the whole rectangle, then the object is
  deleted.
