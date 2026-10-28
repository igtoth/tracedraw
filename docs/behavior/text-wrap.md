# Wrap paragraph text and the baseline grid

Paragraph text flows around the objects marked to wrap it, and its
baselines can sit on the page's baseline grid. Code: `crates/core/src/text_flow.rs`
(what each frame wraps around), `crates/text/src/lib.rs` (the layout).

| Setting | Default |
|---|---|
| Wrap Paragraph Text | off (`Shape::wrap_text`) |
| Wrap style | contour, text on both sides (`Shape::wrap_style`) |
| Text wrap offset | 2 mm (0 to 1000 mm) |

## Turning it on

- The property bar's Wrap Paragraph Text button (Pick tool, after the
  object fields) toggles wrapping for the selected objects
  (`Command::SetWrapText`, one undo step "Wrap Paragraph Text").
- The context menu's Wrap Paragraph Text submenu holds the same toggle
  ("Text flows around it"), the seven wrap styles (the one in use marked
  while the object wraps) and the text wrap offset field; a style or an
  offset turns wrapping on.
- The arrow beside the button lists the styles as the reference
  program's list does (#661): the title Wrap style, None, then under a
  Contour heading and a Square heading their sides (text flows left,
  text flows right, straddle text; and above/below for Square), then the
  text wrap offset (in the ruler unit). None is marked while no selected
  object wraps text, and picking it turns wrapping off for the selection
  (the style and offset are kept). Picking a style turns wrapping on for
  the selection with that style (`Command::SetWrapStyle`, "Wrap Style");
  the list marks the style of the first selected object while any
  wraps. The offset field is greyed out while no selected object wraps,
  so it cannot turn wrapping back on; while one wraps, changing it sets
  the offset for the selection.

## Which objects push which text

- An object pushes the lines of every paragraph frame beneath it in the
  stacking order of its page: all the page's layers from bottom to top,
  the objects inside groups in their place (a group that wraps counts
  with its whole outline). Master layers do the same among themselves.
- Only objects that overlap the frame, offset included, count. Hidden
  objects and objects on hidden layers do not push text. The frame
  itself never does; neither do objects below it.
- A paragraph frame that wraps pushes text with its frame rectangle; any
  other object with its outline (artistic text with its letters).
- The frame's layout gets these outlines in its own space (under its
  transform, so a turned frame wraps around turned shapes). The engine
  works them out again after every change, undo and redo, and when a
  drawing is opened; they are not saved (`ParagraphStyle::flow`).
  Drawing, printing, exports, hit testing and the caret all use the same
  layout.

## Wrap styles

For a line whose band runs from its descent to its ascent, each object
takes an x range of the line, `offset` added on every side and to the
band's height:

| Style | The range it takes |
|---|---|
| Contour, text on the left | from the outline's leftmost x inside the band, to the right edge |
| Contour, text on the right | from the left edge to the outline's rightmost x inside the band |
| Contour, text on both sides | between the outline's leftmost and rightmost x inside the band |
| Square, text on the left | from the bounding box's left side to the right edge |
| Square, text on the right | from the left edge to the bounding box's right side |
| Square, text on both sides | the bounding box's width |
| Square, text above and below | the whole line, wherever the box is at the line's height |

- The outline is flattened to segments (0.1 mm); its extent inside a band
  is that of the segments clipped to the band, so a line beside a
  triangle stops where the slanted side is at its height.
- What is left of the line (the column less its indents and the drop cap)
  minus every range is a list of parts, left to right; parts narrower
  than one em of the line's first letter are dropped. Each part takes a
  line of text of its own, in reading order: the left part first, then
  the right one at the same baseline, then the next line.
- A paragraph's first line keeps its first-line indent from where its
  first part starts, also when an object pushes that part in: beside an
  object at the left the first line starts the indent past the object's
  edge, as the next lines start at the edge. A negative (hanging) indent
  reaches back only at the column's edge, never over an object (#1110,
  `tracedraw_text::tests::the_first_line_indent_holds_beside_a_wrap_object`).
- With no part left the line moves down (a quarter of the line spacing,
  at least 0.25 mm, or to the next baseline grid line) until one shows,
  then to the next column, and the text overflows past the last column
  as usual. The baseline of a line that did not have to move comes from
  its own letters, as without wrapping; a line that had to move takes the
  baseline where room was found.
- Alignment and justification work inside each part; the bullet and the
  drop cap go with the paragraph's first part.
- Up and Down move the caret to the line above or below, the parts of one
  line counting as one; a click picks the nearest part of the line under
  the pointer.

## Baseline grid

- Text > Align to Baseline Grid sets the selected frames' grid pitch to
  the document's baseline grid spacing (and off again).
- A frame that is not turned or slanted puts its baselines on the page's
  baseline grid: the lines `page height - start from top - k * pitch`
  (page coordinates), so frames at different heights line up. Each
  baseline goes to the grid line at or below where it would be.
- A turned or slanted frame measures the grid down from its own top.

## Checks

- `tracedraw_core::text_flow::tests` (segments, sides, contour, which
  objects reach a frame, the page grid origin).
- `tracedraw_text::tests::paragraph_text_flows_around_a_wrap_object`,
  `wrap_sides_keep_the_text_left_right_or_above_and_below`,
  `the_page_baseline_grid_lines_up_frames_at_different_heights`.
- `pick_drag_tests::the_wrap_style_list_starts_with_none`.
- `text_wrap_tests::paragraph_text_wraps_around_an_object_above_it_and_undo_takes_it_back`,
  `objects_below_the_frame_do_not_push_its_text`,
  `wrap_styles_and_offset_set_from_the_menus_turn_wrapping_on`,
  `the_context_menu_lists_the_wrap_styles_and_picks_one`.
