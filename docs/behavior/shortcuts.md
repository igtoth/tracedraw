# Keyboard shortcuts

The default bindings. Every menu
item that shows a shortcut is bound to it; the Options dialog's
Shortcuts page lists them too. `Ctrl` is the Command key on macOS.

## Files and editing

| Keys | Action |
| --- | --- |
| Ctrl+N, Ctrl+O, Ctrl+S, Ctrl+Shift+S | New, Open, Save, Save As |
| Ctrl+I, Ctrl+E, Ctrl+P | Import, Export, Print |
| Ctrl+F4, Alt+F4 | Close window, Exit |
| Ctrl+Z, Ctrl+Shift+Z, Ctrl+R | Undo, Redo, Repeat |
| Ctrl+X, Ctrl+C, Ctrl+V, Ctrl+Shift+V | Cut, Copy, Paste, Paste in view |
| Ctrl+D, Ctrl+Shift+D | Duplicate, Step and Repeat panel |
| Ctrl+A, Delete or Backspace | Select all, Delete |
| Ctrl+F, Ctrl+J, Ctrl+W | Find and Replace panel, Options, Refresh window |

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
| Ctrl+Q, Ctrl+Shift+Q | Convert to curves, Convert outline to object |
| Ctrl+Home, Ctrl+End | To front and back of the page |
| Shift+PgUp, Shift+PgDn | To front and back of the layer |
| Ctrl+PgUp, Ctrl+PgDn | Forward one, back one |
| Alt+Enter, Ctrl+Shift+A, Alt+F7 | Properties, Align and Distribute, Transformations panels |
| Ctrl+M | Merge table cells |

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
  shortcuts above are not interpreted.
- Tools keep their own keys (see `pick-tool.md`, `shape-tool.md` and
  the tool pages); the Options dialog's Shortcuts page can rebind them.

## Checks

- Given a dialog open, when Esc is pressed, then the dialog is closed
  and the tool and selection are unchanged.
- Given a selected object, when Ctrl+Home is pressed, then it is the
  last object of the top layer; Shift+PgDn makes it the first of its
  layer.
