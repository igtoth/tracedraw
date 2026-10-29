# MCP server (`tracedraw --mcp`)

`tracedraw --mcp` runs the editor without a window and speaks the Model
Context Protocol over stdin and stdout: JSON-RPC 2.0, one message per
line, protocol version 2024-11-05, the `tools` capability. An AI agent
(or any MCP client) connected to it opens, edits, inspects, exports and
saves documents with the same commands and undo history a user has.

Client configuration, for example:

```json
{ "mcpServers": { "tracedraw": { "command": "tracedraw", "args": ["--mcp"] } } }
```

## The open window (`tracedraw --mcp-window`)

The same tools can work on the drawings of a TraceDraw window the user
has open. Options > Global > General > "Let AI agents on this computer
edit the open drawing (MCP)" (off by default, desktop only) makes the
window listen on a free port of `127.0.0.1` and write the port, a random
128-bit token and its process id to `mcp-window.json` in the preferences
folder (removed when the option is turned off or TraceDraw closes). The
client is configured with the bridge as its command:

```json
{ "mcpServers": { "tracedraw-window": { "command": "tracedraw", "args": ["--mcp-window"] } } }
```

The bridge reads the file and forwards each line of standard input as an
HTTP `POST /mcp` with `Authorization: Bearer <token>` (never through a
proxy) and writes the answer to standard output. The window answers on
its own thread between frames, as if the user had run the command: the
tools act on the active drawing (`open` and `new_document` open tabs),
each change is one undo step, and the window shows it at once. The
status bar says when agents are let in and on which port, and the option
page shows the port.

Refused: requests without the token or with another one (403), requests
carrying an `Origin` header, which web pages send (403), other methods
(405) and paths (404), bodies over 16 MB (413). Notifications are
answered `202 Accepted` with no body. A request waits up to 120 s for the
window. Without a window that lets agents in, the bridge answers every
request with a JSON-RPC error (-32000) saying how to turn the option on.

## Methods

| Method | Reply |
| --- | --- |
| `initialize` | protocol version, `tools` capability, server name and version |
| `ping` | `{}` |
| `tools/list` | the tools below with their JSON schemas |
| `tools/call` | `content: [{type: "text", text}]`, `isError` on failure |
| notifications (no id) | no reply |
| anything else | error -32601 |

## Tools

| Tool | Arguments | Effect |
| --- | --- | --- |
| `open` | `path` | Opens any supported file (`.cdr`, `.tdraw`, SVG, PDF, AI, EPS, DXF, PSD, EMF, WMF, images) and returns the document description |
| `new_document` | `width_mm`, `height_mm`, `title` (all optional; A4 default) | A fresh document |
| `save` | `path`, `version` (optional, `.cdr` only: 7 to 27) | Native `.tdraw`, or `.cdr` through the writer in `version` (default: the version of the `.cdr` the document came from or was last saved in, else the one in Options > Global > File Formats); marks the document saved |
| `export` | `path`, `page_index` (0), `dpi` (96) | SVG, PDF, EPS, DXF, EMF, WMF, PLT, PSD, HTML or PNG by extension |
| `run_script` | `source` | Runs the JavaScript object model (`Application`, `ActiveDocument`, `ActivePage`, `ActiveLayer`, `ActiveSelection`, `Shapes`, `CreateRectangle`, `CreateEllipse`, `CreateArtisticText`, `CreateCurve`, `Fill`, `Outline`, `Move`, `Rotate`, `Stretch`, `Duplicate`, `Delete`, ...) and returns what it printed; the whole script is one undo step |
| `document_info` | | Title, file, modified flag, pages with size, layers with visibility and lock, every object with id, type, name, bounds in mm, fill, outline, opacity, text and font, group children, effect count; the selection |
| `undo`, `redo` | | One step back or forward, then the description |

New objects go to the active layer: the layer picked in the Objects
docker while it shows on the page, visible and unlocked, else the
topmost layer that is visible and unlocked (a hidden layer never
receives objects; with every layer hidden, the topmost unlocked one).
The same rule applies to drawing in the window.

## Checks

- `initialize` answers with the protocol version and server name; a
  notification gets no reply; an unknown method gets -32601.
- `new_document` 100 x 50, then a script creating a red rectangle and
  printing `ActivePage.Shapes.Count`, prints `1`; `document_info` lists
  one rectangle with fill `R:255 G:0 B:0`; `undo` leaves none.
- `export` to svg, pdf, png and emf and `save` to cdr and tdraw each
  write a non-empty file; the saved `.cdr` opens again with its ellipse.
- `save` with `version` 21 writes a file that reads back as version 21;
  versions 3, 23, -1 and a string are refused and write nothing.
- The open window: with the option on, `initialize`, a notification and
  a script drawing an ellipse sent through the bridge get the server's
  name, no answer and `ok`, the window's drawing has one more object and
  Undo takes it back; the link file goes when the link does
  (`mcp_link::tests::agents_edit_the_drawing_in_the_window`).
- Requests without the token, with a wrong one or with an `Origin`
  header get 403, a GET 405, another path 404, the right token 200
  (`mcp_link::tests::strangers_and_web_pages_are_refused`).
- Without a window, or with one that went away, the bridge answers with
  an error naming the option (`mcp_link::tests::without_a_window_the_bridge_says_so`);
  the option starts and stops listening
  (`mcp_link::tests::the_option_starts_and_stops_listening`).
- Checked on screen: `tracedraw --mcp-window` drew a star and a line of
  text in the open window.
