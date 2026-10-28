# Keyboard shortcuts

The default bindings. Every menu
item that shows a shortcut is bound to it; Tools > Options >
Customization > Commands lists them and takes shortcuts of your own
(see "Your own shortcuts" below). `Ctrl` is the Command key on macOS.

## Files and editing

| Keys | Action |
| --- | --- |
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | New, Open, Save, Save As |
| Ctrl+I, Ctrl+E, Ctrl+P | Import, Export, Print |
| Ctrl+F4, Alt+F4 | Close window, Exit |
| Ctrl+Z, Ctrl+Shift+Z, Ctrl+R | Undo, Redo, Repeat |
| Ctrl+X, Ctrl+C, Ctrl+V, Ctrl+Shift+V | Cut, Copy, Paste, Paste in view |
| Ctrl+D, Ctrl+Shift+D | Duplicate, Step and Repeat panel |
| + (Pick tool, with a selection) | Duplicate in place |
| Ctrl+Space | the Pick tool, from any tool |
| Ctrl+0 to Ctrl+9, 0 to 9, Alt+0 to Alt+9 | store, select (twice: zoom to) and add to selection groups |
| Ctrl+A, Delete or Backspace | Select all, Delete |
| Ctrl+F, Ctrl+J, Ctrl+W | Find and Replace panel, Options, Refresh window |


### Repeat

Edit > Repeat (Ctrl+R) does again, on the selection of the time, the
last repeatable action still in effect: a move, scale, rotation or
skew, a fill, an outline, a transparency, or a copy made with a right
click during a drag (which makes another copy, transformed as far again
from the new selection). Each action is kept with the undo step it made
(`Engine::last_step_id`, `App::repeatables`): an action undone is never
repeated, the one before it is (#366); redone, it is the last again; and
an undone step replaced by another change is gone for good. A copy made
by a right click during a drag, or by its Repeat, is one undo step, the
copies and their move together (`Engine::begin_group` and
`end_group`, which also keep the undo memory limit from splitting
them; it was two, the first Undo only moving the copy back onto the
original, #471). Checks:
`pick_drag_tests::repeat_after_an_undo_takes_the_last_action_still_in_effect`,
`pick_drag_tests::repeat_after_a_right_click_copy_makes_another_copy` (#363),
`engine::tests::steps_have_ids_that_follow_undo_and_redo`.

## View

| Keys | Action |
| --- | --- |
| F2, Shift+F2, F3, F4, Shift+F4 | Zoom tool, zoom to selection, zoom out, zoom to fit, zoom to page |
| Ctrl++ (or Ctrl+=), Ctrl+- | Zoom in and out one step |
| Alt+Shift+R | Rulers on and off |
| Alt+Q, Alt+Shift+A, Alt+Shift+D | Snapping off, alignment guides, dynamic guides |
| Alt+Z, Ctrl+Y | Snap to objects, snap to the document grid, on and off |
| PgUp, PgDn | Previous and next page |

## Objects

| Keys | Action |
| --- | --- |
| Ctrl+G, Ctrl+U | Group, Ungroup |
| Ctrl+L, Ctrl+K | Combine, Break apart |
| While typing text: Ctrl+L, Ctrl+E, Ctrl+R, Ctrl+J, Ctrl+H, Ctrl+N | Align left, center, right, justify, force justify, no alignment (the whole text typed at a point, which stays in place at the reference point of the X and Y fields; the paragraphs at the caret in a frame) (`pick_drag_tests::alignment_keys_while_typing`, `text_wrap_tests::a_new_alignment_keeps_the_reference_point_in_place`) |
| Ctrl+Q, Ctrl+Shift+Q | Convert to curves, Convert outline to object (a dashed outline keeps its dashes, each a closed shape, the pattern in widths as drawn; #490, `tools2::tests::a_dashed_outline_becomes_one_shape_per_dash`) |
| Ctrl+Home, Ctrl+End | To front and back of the page |
| Shift+PgUp, Shift+PgDn | To front and back of the layer |
| Ctrl+PgUp, Ctrl+PgDn | Forward one, back one |
| Alt+Enter, Ctrl+Shift+A | Properties, Align and Distribute panels |
| Alt+F7, Alt+F8, Alt+F9, Alt+F10 | Transformations panel on Position, Rotate, Scale and Mirror, Size |
| Ctrl+M | Merge table cells |
| Alt+Shift+F8, Alt+F11 | Run Macro, Macro Editor (Tools > Macros) |

## Text and effects

| Keys | Action |
| --- | --- |
| Ctrl+T, Ctrl+F11, Ctrl+F12 | Text, Insert Character, Fonts panels |
| Ctrl+Shift+T, Alt+F12 | Edit text, Align to baseline |
| F11, Shift+F11, F12, Shift+F12 | Gradient fill, Colour, Outline pen, Outline colour |
| Ctrl+F7, Ctrl+F8, Ctrl+F9, Ctrl+F10, Alt+F3, Ctrl+F3 | Envelope, Blend, Contour, Extrude, Lens, Symbols panels |

Panel shortcuts toggle: pressing one when its panel is already the
active tab hides the panel column.

## Align keys and tool keys

With a selection, the plain letters L, R, T, B, E, C and P align it
(left, right, top, bottom, centre horizontally, centre vertically,
centre of page); a single object aligns to the page. Without a
selection the same letters reach the tool keys (E Roughen, C none, B
none). Tools with letter keys: Z Zoom, H Pan, Y Polygon, A Spiral, D
Graph Paper, G Interactive Fill, M Mesh Fill, X Eraser, W Smear, V
Smudge, E Roughen, S Sketch, Shift+S Shape Recognition, I Artistic
Media; the Bezier, Pen, Crop and Knife tools have no key.

## Modal dialogs

- Esc closes any dialog without applying; while a dialog is open the
  shortcuts above are not interpreted, and typing does not reach text
  or a table cell being edited on the canvas.
- Dialogs are modal: while one is open, a transparent layer under it
  takes every click, drag and scroll aimed at the menus, toolbars,
  dockers, tabs and the drawing; only the dialog (and the lists it
  opens) can be used until it closes
  (`ui::dialogs::tests::an_open_dialog_takes_the_clicks_outside_it`).
- Esc with a menu or a drop-down list open closes it and changes
  nothing else (the selection stays).
- Tools keep their own keys (see `pick-tool.md`, `shape-tool.md` and
  the tool pages); the Commands page of the options can rebind them.

## Your own shortcuts

Tools > Options > Customization > Commands lists every command of the
menu bar (commands not made yet are left out) as "Menu > Submenu >
Command" with its built-in shortcut. A search field filters the list by
name.

- Clicking a command's button ("Set shortcut") makes it wait for keys:
  the next key pressed, with the Ctrl, Alt and Shift held at the time,
  becomes its shortcut (shown as `Ctrl+Alt+Shift+Key`). Esc, Tab,
  Enter, Backspace and Space alone are not taken (they edit text, move
  focus or close dialogs); Esc stops the wait without closing the
  options. Changing page or closing the options stops it too.
- A shortcut of your own runs its command wherever the command's menu
  item would: the menus are walked without being shown and the item
  reports a click, so the command does exactly what the menu does. A
  command not available at that moment (Duplicate with nothing
  selected, Edit commands with no drawing open) runs nothing and the
  status bar says so.
- Your shortcuts are looked at before the built-in ones, so a key a
  built-in command uses runs your command instead; the page names the
  commands and tools that lost it. Another command of yours with the
  same keys loses them. A command's built-in shortcut keeps working
  beside yours until another command takes it.
- The menus show your shortcut in place of the built-in one. Delete
  leaves the command without a shortcut, a built-in one too: it is
  kept as an empty shortcut, the menus show none and its keys do
  nothing. Reset to defaults gives back the built-in shortcuts of the
  commands in the category shown (one menu, the toolbox or the
  macros); with all commands shown it asks first, as Reset all
  shortcuts does, which gives back every built-in shortcut, tool keys
  included. Shortcuts are kept in the preferences (`shortcuts`: the
  menu item's key or tool id and the keys, empty for none).
- Tools are listed under the toolbox on the same page and take keys as
  commands do: a key alone or with Ctrl, Alt and Shift. Alt and a
  letter you give a command or tool runs it instead of opening the
  menu with that access letter, and while the page waits for keys no
  menu opens.
- Rows a menu shows only at times are listed all the same and take keys
  (Object > Clip to Frame > Finish Editing Contents beside Edit Contents,
  Text > Vertical Text), #1148.
- The bars' own buttons (listed under Toolbar Buttons) that run a menu
  command show that command's keys on their Shortcut Keys tab and set
  them: New, Open, Save, Print, Cut, Copy, Paste, Undo, Redo, Import,
  Export, Publish to PDF, Full-screen Preview, Rulers, Grid,
  Guidelines, Snap Off, Zoom In, Zoom Out and Zoom to Fit
  (`bar_items::menu_command`, #1148). The buttons the menus lack (the
  zoom levels list, Snap To, the other zoom buttons, Options, Launch)
  have no Shortcut Keys tab.

## Checks

- `ui::options::tests::bar_buttons_and_rows_shown_at_times_take_shortcuts`

- Given a dialog open, when Esc is pressed, then the dialog is closed
  and the tool and selection are unchanged.
- Given a selected object, when Ctrl+Home is pressed, then it is the
  last object of the top layer; Shift+PgDn makes it the first of its
  layer.
- Given Select All given Alt+Shift+K and nothing selected, when
  Alt+Shift+K is pressed, then every object is selected and the Edit
  menu shows Alt+Shift+K beside Select All.
- Given Select All given Ctrl+D, when Ctrl+D is pressed, then the
  objects are selected and nothing is duplicated; Ctrl+A still selects
  all.
- Given Export waiting for its keys, when Ctrl+D is pressed, then Export
  has Ctrl+D and the page says it was Duplicate's; Enter alone is not
  taken and Esc stops waiting with the options still open.

## Access keys in menus

- Every menu title and row has one letter underlined (`ui/menus.rs`,
  `access`): the first letter or digit starting a word that no earlier
  row of the same menu took, else the first free one anywhere. Letters
  outside A to Z and 0 to 9 are skipped, so labels in other scripts may
  have none.
- With a menu open, pressing a row's letter (without Ctrl or Alt) works
  as a click: an item runs, a submenu opens. Only the innermost open
  menu takes the key.
- Alt and a title's letter opens that menu, except E, S, X, Q and Z,
  which commands use with Alt.
- While any menu or list is open, the drawing's shortcuts do not run.
