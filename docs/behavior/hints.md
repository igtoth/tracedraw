# Hints panel

The Hints panel is open by default, docked on the right with Properties
and Objects (the tab strip shows an icon above each name; the active tab
is tinted with an accent line).

## Pages

- **Home**: an introduction and the list of topics (Lines, Connector
  lines, Dimension lines, Shapes, Select objects, Move, scale, and stretch
  objects, Rotate and skew objects, Shape objects, Special effects,
  Outline objects, Fill objects, Add text, Get help), each a link.
- **Topic**: a title, an introduction with the keys in bold, and the
  tools that serve it, each with its icon, its name in bold and what it
  does; a tool opens its page.
- **Tool**: a task title, the tool row, then the tool's steps: one bullet
  per step (key names in bold), most with a picture under it that shows
  the object, its handles or nodes, the drag and the tool's cursor (drawn
  in `ui/hint_pics.rs`, at most 320 points wide, on a white page), then
  "Learn more" with the help topic (the tool's behaviour notes) and "See
  also" linking the topic that lists the tool.

- **Object**: the page of what is selected: its kind as the title
  (Curve, Rectangle, Ellipse, Polygon or star, Artistic text, Paragraph
  text, Bitmap, Group, Clip frame, Blend, Dimension, Connector, Callout,
  Table, Symbol, Object with an effect, or Several objects when the
  selected objects differ), two bullets on what can be done with it,
  "Tools for it" with the rows of the tools that work on it (each opens
  its page), "On the property bar" with what the bar offers for it, a
  dim line saying a click on an empty spot goes back to the tool's page,
  and "Learn more". Dimensions, connectors, callouts, blends and objects
  with a live effect (contour, distortion, envelope, perspective,
  extrusion, bevel, block shadow) are known before their plain shape.

Every page ends with "Learn more" and its help topic link.

## Navigation

- Choosing another tool shows that tool's page (the panel follows the
  active tool; it starts on Home with the Pick tool).
- Selecting objects shows the object page of what was picked; letting
  go of the selection (a click on an empty spot, Esc) goes back to the
  active tool's page. With the Pick, Freehand Pick, Free Transform and
  Shape tools every new selection shows its page. With other tools only
  a selection made without changing the drawing does (a click on an
  object): the object a drawing tool has just drawn leaves the tool's
  steps in view. A change to the drawing that keeps the selection (Convert
  to Curves) updates an object page already in view, and leaves any other
  page alone.
- The panel compares a small key of the selection each frame (the
  document, its revision, the page, the number of selected objects and
  the first and last of them) and reads the selected objects only when
  the key changes, so following the selection costs nothing per frame.
- The bar under the page: Home at the left, Back and Forward at the
  right (dimmed when there is nowhere to go). Going to a new page drops
  the pages ahead of the current one.

All titles, descriptions and introductions are in the language files
(`hinttitle.*`, `tooldesc.*`, `hinttopic.*`, `hintstep.<tool>.<n>`,
`hintobj.<kind>.title`, `.1`, `.2` and `.bar`). The tool
descriptions are the same ones the toolbox tooltips show.

## Checks

- `ui::hints::tests::history_goes_back_and_forward_and_drops_the_future`
- `ui::hints::tests::every_tool_has_a_page_title_description_and_topic`
- `ui::hints::tests::step_keys_are_in_bold`
- `ui::hints::tests::every_object_kind_has_its_texts_tools_and_a_help_page`
- `ui::hints::tests::the_selection_shows_its_page_and_letting_go_shows_the_tool`
- `pick_drag_tests::the_hints_panel_shows_the_page_of_the_picked_object`
- `ui::hint_pics::tests::every_tool_has_steps_with_text_and_pictures`
- `ui::hint_pics::tests::every_picture_draws_on_a_page`
