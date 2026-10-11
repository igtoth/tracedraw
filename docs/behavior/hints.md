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
- **Tool**: a task title, the tool row, one bullet per instruction of the
  tool's status bar hint (split at semicolons, key names in bold), "Learn
  more" with the help topic (the tool's behaviour notes), and "See also"
  linking the topic that lists the tool.

Every page ends with "Learn more" and its help topic link.

## Navigation

- Choosing another tool shows that tool's page (the panel follows the
  active tool; it starts on Home with the Pick tool).
- The bar under the page: Home at the left, Back and Forward at the
  right (dimmed when there is nowhere to go). Going to a new page drops
  the pages ahead of the current one.

All titles, descriptions and introductions are in the language files
(`hinttitle.*`, `tooldesc.*`, `hinttopic.*`, `status_hint.*`). The tool
descriptions are the same ones the toolbox tooltips show.

## Checks

- `ui::hints::tests::history_goes_back_and_forward_and_drops_the_future`
- `ui::hints::tests::every_tool_has_a_page_title_description_and_topic`
- `ui::hints::tests::hints_split_into_instructions_with_keys_in_bold`
