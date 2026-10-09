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
| `save` | `path` | Native `.tdraw`, or `.cdr` through the writer; marks the document saved |
| `export` | `path`, `page_index` (0), `dpi` (96) | SVG, PDF, EPS, DXF, EMF, WMF, PLT, PSD, HTML or PNG by extension |
| `run_script` | `source` | Runs the JavaScript object model (`Application`, `ActiveDocument`, `ActivePage`, `ActiveLayer`, `ActiveSelection`, `Shapes`, `CreateRectangle`, `CreateEllipse`, `CreateArtisticText`, `CreateCurve`, `Fill`, `Outline`, `Move`, `Rotate`, `Stretch`, `Duplicate`, `Delete`, ...) and returns what it printed; the whole script is one undo step |
| `document_info` | | Title, file, modified flag, pages with size, layers with visibility and lock, every object with id, type, name, bounds in mm, fill, outline, opacity, text and font, group children, effect count; the selection |
| `undo`, `redo` | | One step back or forward, then the description |

New objects go to the active layer: the topmost layer that is visible
and unlocked (a hidden layer never receives objects; with every layer
hidden, the topmost unlocked one). The same rule applies to drawing in
the window.

## Checks

- `initialize` answers with the protocol version and server name; a
  notification gets no reply; an unknown method gets -32601.
- `new_document` 100 x 50, then a script creating a red rectangle and
  printing `ActivePage.Shapes.Count`, prints `1`; `document_info` lists
  one rectangle with fill `R:255 G:0 B:0`; `undo` leaves none.
- `export` to svg, pdf, png and emf and `save` to cdr and tdraw each
  write a non-empty file; the saved `.cdr` opens again with its ellipse.
