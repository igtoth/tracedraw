# Pick tool

| Action | Behaviour |
|---|---|
| Click | select the topmost object under the pointer (its fill counts only when filled; otherwise its outline within 3 px; text, groups, tables, bitmaps, clip frames and symbols count anywhere inside their bounds); an object with live effects counts where it is drawn: its warped outline and the objects its effects add, and its selection box encloses them |
| Shift+click | add or remove from the selection |
| Ctrl+click | the object inside a group or a clip frame, one level further in at each click (the first level inside the topmost group, then the group or object inside that, down to the innermost object; a clip frame's contents count only inside the frame, where they show); its handles are round, and moving, scaling, colouring or deleting it changes that object only (a group left empty goes); once chosen it is dragged on its own from anywhere on it without Ctrl, while a press on another part of the group takes the whole group (#849, `pick_drag_tests::object_chosen_in_a_group_drags_alone_from_anywhere_on_it`); Ctrl+Shift+click adds one more; a group and an object inside it are never selected together |
| Alt+click | the object beneath the selected one under the pointer, round to the top again after the bottom one (dig); with nothing there selected, the topmost; Ctrl+Alt digs through the objects inside groups and clip frames (a clip frame comes after its contents); Shift+Alt adds it |
| Click on a selected object (any of a selection), or on the centre mark | toggle the rotate/skew handles: curved arrows at the corners, straight double arrows at the sides, the centre of rotation in the middle |
| Drag on empty | marquee (a dashed blue frame); objects wholly inside are selected; with Alt held when the drag ends, every object the frame touches: its outline crosses or lies in the frame, or the frame lies in its fill (text, bitmaps, tables and clip frames anywhere in their box; a group when one of its objects counts); Shift keeps the selection and turns over the objects the frame takes (selected ones are deselected, the others added; the Freehand Pick lasso too, #401); Ctrl keeps the marquee square; with Options > Tools > Pick, Marquee selects touched objects, Alt works the other way round |
| Drag the centre mark (the "x") | moves the selection, also where the mark lies on nothing (an outline-only ring) |
| Drag object | move with the pointer (snapped); Ctrl keeps the move horizontal or vertical, whichever is longer; Space drops a copy where the selection is and the move goes on (also while scaling, turning or slanting: the copy takes the transform reached); let go over the document palette to add the objects' colours to it instead of moving them |
| Corner handle | scale proportionally from the opposite corner, by where the pointer falls along the box's diagonal: `s = (|sx| w² + |sy| h²) / (w² + h²)` with `sx`, `sy` the two sides' own ratios (each keeps its sign, for mirroring), so the corner follows the pointer and a wide, low box such as a line of text does not grow at the short side's pace (#383); the handle follows the pointer from the frame the drag starts, not in a jump of the few pixels before a press becomes a drag; Shift from the centre; Ctrl in whole steps of 100 % (crossing the other side gives the mirror image at the same size); Alt scales freely |
| Side handle | stretch one axis from the opposite side (across it: mirrored); Shift from the centre; Ctrl in steps of 100 % |
| Snapping while scaling | with snapping on, the dragged corner or side handle snaps to other objects' points and edges, guidelines, the grid and the page (never to the selection's own points), and the snap mark shows where |
| Rotate handle (rotate mode, corners) | rotate about the centre of rotation; Ctrl in steps of the constrain angle (15 degrees); without Ctrl the pointer snaps to other objects' points only, never to edges or lines (#933, see `snapping.md`) |
| A node of the selection (rotate mode; also a segment's midpoint or an ellipse's quadrant, the points the pointer marks) | dragged, turns the selection about the centre of rotation, the node going round its circle towards the pointer (the turning arrows show over it); near another object's point the turn aims at it, near an edge the node lands where its circle crosses the edge, exactly on it; Ctrl in steps of the constrain angle; elsewhere on the selection a drag moves it as outside rotate mode (#933, `pick_drag_tests::a_node_dragged_in_rotate_mode_turns_the_selection_onto_a_line`) |
| Skew handle (rotate mode, sides) | slant: the top and bottom handles sideways, the left and right ones up and down, the opposite side fixed; Shift about the centre line; Ctrl in steps of the constrain angle |
| Centre of rotation (rotate mode, a ring with a dot) | drag it to turn about another point; it is kept with the object: rotate mode going off and on, clicking elsewhere and selecting the object again find it where it was put, and it travels with the object when that is moved, scaled or turned (kept in the object's own space; several objects selected together keep theirs as a place within their bounds; each drawing has its own; not saved in the file); double-click puts it back in the middle; a click on it keeps rotate mode |
| Right click during a move, scale, rotation or skew | the original stays and a copy takes the change (the copy becomes the selection); a second right click takes it back; the status bar says which, and the pointer carries a plus in a square (beside the box of a move, at the lower right of the scale, turn and slant pointers). The transform runs until the left button goes up, whichever button is released first or both in one frame, and wherever it is dropped (bare canvas, another object): the release is never taken for a click there, a held second right press does not start it again, and a palette swatch under the pointer does not take the right click as an outline colour (#528, `pick_drag_tests::a_right_click_copy_survives_any_release_order`, `a_right_click_copy_dropped_on_another_object_copies_the_dragged_one`, `a_right_click_during_a_ctrl_mirror_leaves_a_mirrored_copy` (Ctrl and a side handle dragged across: a mirrored copy), `a_right_click_during_a_turn_or_a_slant_leaves_the_original`, `a_held_second_right_press_takes_the_copy_back_and_the_move_goes_on`) |
| Esc during a move, scale, rotation or skew | cancels it; nothing changes |
| Arrows | nudge by the nudge distance (2.54 mm default); Shift by the super nudge distance (5.08 mm), Ctrl by the micro nudge distance (0.254 mm); all three in Document Options > Rulers; with guidelines selected and no object, the guidelines move |
| Tab / Shift+Tab | cycle selection in stacking order |
| + (also on the numeric keypad) | duplicate the selection in place (no offset), the copies selected, one undo step |
| Double-click | Shape tool on a curve; on text, the Text tool editing that text with the caret at the end |
| Esc | deselect |
| Double-click the tool button | select all objects |

The toolbox button of each flyout shows the tool last used from it; the
choice is kept between sessions (`toolbox` in the settings file).

Hidden and locked objects, and the objects on hidden or locked layers,
are never hit (a right click on a locked object that is topmost under
the pointer opens a menu of its own, Unlock Object, which unlocks and
selects it, and Unlock All Objects; the object is not selected while
locked); of the others the topmost under the pointer is (the last
in the stacking order, which `.cdr` files keep: a text over a
rectangle is hit before it). Treat all objects as filled (Options >
Tools > Pick, and the property bar) is on by default, so an empty
rectangle over a text takes a click inside it; Alt+click reaches the
text beneath, or the option switched off leaves an empty object to
clicks on its outline.
Checks: `pick_drag_tests::text_over_a_rectangle_read_from_cdr_is_hit_first`,
`pick_drag_tests::an_unfilled_rectangle_over_text_takes_the_click_and_alt_reaches_the_text`,
`pick_drag_tests::ctrl_click_takes_the_contents_of_a_clip_frame`,
`pick_drag_tests::clipped_away_contents_are_not_hit`.

The status bar names an object chosen inside a group "Rectangle in a
group on Layer 1". Arrange commands (order, group, combine) act on the
objects on layers and leave an object chosen inside a group where it is.

The drag keeps going while the left button is held, also after a right
click (the windowing layer ends a drag when any button goes up); the
modifier keys are read at every step, so pressing or letting go of Shift,
Ctrl or Alt during the drag changes the result at once.

## The handles

The selection box has no frame of its own, so the objects' outlines
stay in view. Eight solid black square handles with a one-pixel white
edge (so they show on a dark drawing, #670), 10 interface points
across, stand just outside it: their centres 6.5 points out from the box
(a gap of 1.5 points). A corner or side handle scales from where it was
grabbed, so the box follows the pointer without a jump. Objects chosen
inside a group show round handles. The centre mark is a small cross.

In rotate mode the handles are black arrows 2 points thick with solid
heads, their centres 9 points out from the box: a curved double arrow
around each corner, bulging outwards, and a straight double arrow along
each side. The centre of rotation is a ring 8 points in radius with a
dot in the middle.

## Large selections

With many objects selected (Select All on a drawing of thousands of
objects) the window keeps its frame rate: the selected objects are
gathered in one pass over the layers and kept, shared, while the
drawing, the page and the selection stay as they are
(`App::selected_rc`); the bars, the status bar and the canvas read that
one list, and the checks that ran once per selected object over the
whole drawing (its layer, whether it lies in a group, linked text
frames) take one pass too. With 3000 rectangles selected a frame went
from about 200 ms to about 7 ms (#367; check
`app::tests::the_selected_objects_are_gathered_once_and_kept_while_nothing_changes`).

## Selection groups

Ctrl and a number from 0 to 9 store the selection under that number;
the number alone selects those objects again (the ones the current page
shows), and pressed again while they are selected zooms to them. Alt and
the number add the selection to the group. Groups are saved with the
drawing; objects deleted since drop out. With text selected, Ctrl+8 and
Ctrl+2 keep changing the text size.

## Node tracking

Options > Display > Enable node tracking (off by default): with the Pick
tool, the pointer over a node of a selected curve takes the Shape tool's
arrow with the node mark, and a drag there moves that node (snapping
like the Shape tool's node drags; Ctrl keeps it horizontal or vertical;
one undo step). The object can then no longer be moved by dragging it at
one of its nodes. Not in rotate mode.

## Dragging the selection

While the selection is moved, scaled, turned or slanted, the objects stay
where they are and their outline shows where they would go: a blue line
on a white halo, which reads as two light lines on a dark fill. The
outline is the one the release makes: a rectangle's corners that keep
their size stay round while it is stretched
(`Shape::page_path_after`). The handles return on release.

A move grabbed on a mark (see below) carries that point: a node,
midpoint, centre or the point of an edge lands on other objects' points
and edges and slides along an edge while the pointer stays within the
snapping radius. The edge snapped to lights up in blue while the
drag lasts. A scale or skew handle snaps the point of the box it
stands for, and turning snaps the pointer, to other objects' points and
edges, guidelines, the grid and the page (Ctrl turns in steps instead).

## Duplicate

Edit > Duplicate (Ctrl+D) copies the selection at the duplicate offset
(Options > Document > General, 6.35 mm right and up at first) and
selects the copies. While the copies stay the selection, the next
Duplicate repeats the step from each original to its copy: the offset
if they were left alone, or the move, scale, turn or skew done to them
since (smart duplicate, as in the reference program): duplicate a box,
move the copy 20 mm, and each next Duplicate is 20 mm further; scale the
copy by 10 % and each next one is 10 % larger. With anything else
selected it is the offset again (#559,
`app::tests::duplicate_repeats_what_was_done_to_the_copy`).

## Snap marks over the selection

With snapping to objects on, the pointer resting over the selection (not
over a handle or the centre of rotation) shows the selection's point or
edge within the snapping radius, the one a drag would carry: its mark
and, with screen tips on, its name in lower case (node, midpoint,
center, edge and the other snapping modes in use). Points win over edges. Handles take a
press within 6 points of their centre, short of the selection box.

## The pointer

The pointer says what a press would do (code: `ui/cursors.rs`):

| Under the pointer | Pointer |
|---|---|
| A corner handle | diagonal resize arrows (top left and bottom right, or top right and bottom left) |
| A side handle | vertical (top, bottom) or horizontal (left, right) resize arrows |
| A selected object (or the selected part of a group under the pointer), the centre mark or the centre of rotation | move arrows |
| An object that is not selected | the arrow (the cross hair with Options > Tools > Pick > Cross hair): a press picks it first, and a drag then moves it |
| A horizontal or vertical guideline (where no object is) | up and down or sideways arrows across it; the move arrows on a slanted one; the arrow on a locked one |
| A rotation handle of a guideline in rotate mode | a turning arrow |
| A corner handle in rotate mode | a turning arrow |
| A side handle in rotate mode | two half arrows along the slant (sideways for top and bottom, up and down for the sides) |
| Anything else | the arrow (the cross hair with Options > Tools > Pick > Cross hair) |

While dragging, the pointer keeps the shape of what is being done (a
move: the white arrow with a dotted box under its tail, and a plus in a
square beside the box once a right click made the move leave the
original; it stays drawn after the right click until the left button
goes up; the move arrows for the centre of rotation; the handle's resize arrows, the turning arrow, the slant
arrows, the arrows across a guideline being moved or dragged out of a
ruler, the cross hair while the ruler origin is dragged). Resize and move arrows are the system's own; the moving
arrow, the turning and slant arrows are drawn by TraceDraw, black on a
white halo.

## Dragging with the right mouse button

Pressing the right button on an object (it is selected first if it was
not) and dragging shows the selection's outline in blue where it would
go, while the objects stay put; Ctrl keeps the drag horizontal or
vertical, Esc cancels it. On release a menu opens there:

| Item | Effect |
|---|---|
| Move Here | moves the selection by the drag |
| Copy Here | copies of the selection, moved by the drag, become the selection |
| Copy Fill Here | the object under the pointer takes the fill of the first selected object |
| Copy Outline Here | ... its outline |
| Copy All Properties | ... its fill, outline and transparency |
| Place Inside | the object under the pointer becomes the clip frame of the selection |
| Cancel | nothing |

The four items that act on the object under the pointer are dimmed when
there is none. A right click without a drag opens the context menu.

## Property bar

With nothing selected the bar shows the page fields (size, orientation,
units, nudge distance). With a selection it shows X, Y (lower left corner),
W, H, scale, angle, mirror buttons, the outline width, To Front, To Back
and Convert to Curves. Text selected on its own adds the text fields of the
Text tool after them. Bitmaps selected on their own replace Convert to
Curves with the bitmap commands: Edit Bitmap, Crop Bitmap (switches to the
Crop tool), Trace Bitmap (Quick Trace and the centreline and outline
presets), Straighten Image and Resample.

With two or more objects selected, a group, or a curve of several parts,
the bar ends with the arranging buttons: Group (Ctrl+G), Ungroup
(Ctrl+U) and Ungroup All (with a group in the selection), Combine
(Ctrl+L) and Break Apart (Ctrl+K, with a curve of several parts), the
shaping operations Weld, Trim, Intersect, Simplify, Front Minus Back,
Back Minus Front and Boundary (they act as in Object > Shaping, see
`shaping.md`; Boundary works on one object too), and a button that opens the Align and
Distribute panel (Ctrl+Shift+A). Their pictures show a square behind a
circle: the result filled, an object that stays outlined, one that goes
away dashed.

## Dialogs

Enter presses OK in every modal dialog (Esc cancels). In the two
multi-line fields (QR code text and document notes) Enter inserts a line
break instead.

## Checks

- `pick_drag_tests::overlays_are_drawn_on_a_white_line`
- `envelope_edit::tests::enveloped_objects_are_hit_and_boxed_as_drawn`

- `ui::propbar::tests::shaping_and_grouping_show_for_several_objects_groups_and_parts`
- `pick_drag_tests::ctrl_keeps_a_move_straight`
- `pick_drag_tests::a_right_click_while_dragging_leaves_the_original`
- `pick_drag_tests::esc_cancels_a_drag`
- `pick_drag_tests::space_while_moving_drops_a_copy`
- `pick_drag_tests::shift_scales_from_the_centre_and_ctrl_mirrors`
- `pick_drag_tests::rotate_mode_skews_with_the_sides_and_turns_about_the_moved_centre`
- `pick_drag_tests::handle_modifiers_shape_the_scale`
- `pick_drag_tests::side_handles_skew`
- `pick_drag_tests::the_pointer_shows_what_a_press_would_do`
- `pick_drag_tests::the_status_bar_tells_how_far_how_much_and_what_angle`
- `pick_drag_tests::objects_dropped_on_the_document_palette_give_their_colours`
- `pick_drag_tests::ctrl_click_chooses_an_object_inside_a_group`
- `pick_drag_tests::alt_click_digs_to_the_object_beneath`
- `pick_drag_tests::alt_marquees_take_what_they_touch`
- `pick_drag_tests::the_touching_option_swaps_what_alt_does`
- `pick_drag_tests::clicking_the_selection_or_its_centre_mark_turns_the_handles`
- `pick_drag_tests::the_pointer_shows_moves_for_objects_and_guidelines`
- `pick_drag_tests::scale_handles_snap_to_other_objects_not_to_the_selection`
- `pick_drag_tests::plus_duplicates_in_place_and_edits_nodes_in_the_shape_tool`
- `pick_drag_tests::a_moved_centre_of_rotation_is_kept_with_the_object`
- `select_region::tests` (wholly inside, touching, groups, lassos)
- `command::tests::objects_inside_groups_can_be_changed_and_deleted` (core)
- `tests::the_drag_preview_reaches_objects_inside_groups` (render)
- `drop_menu::tests::move_here_and_copy_here`
- `drop_menu::tests::fill_outline_and_properties_go_onto_the_target`
- `drop_menu::tests::place_inside_makes_the_target_a_clip_frame`
- `drop_menu::tests::a_ctrl_drag_keeps_to_one_direction`
