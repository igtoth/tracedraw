# Zoom and Pan

## Zoom tool (Z, F2)

- Click zooms in by 2 at the point; Shift+click (or right-click) zooms
  out by 2. Dragging a marquee zooms to fit that rectangle.
- F2 picks the tool; Shift+F2 zooms to the selection; F4 zooms to fit
  all objects; Shift+F4 zooms to the page.
- `+`/`=` and `-` zoom by 1.25 about the canvas centre; the zoom combo
  on the standard toolbar steps through 10, 25, 50, 75, 100, 150, 200,
  300, 400, 800 and 1600 % (beyond the last step the factor doubles).

## Mouse wheel

- Default (Options > Tools > Default action for mouse wheel: Zoom): the
  wheel zooms by 1.15 per notch about the pointer; Ctrl+wheel scrolls
  vertically; Alt+wheel scrolls horizontally.
- With the Scroll setting: the wheel scrolls vertically; Shift+wheel
  scrolls horizontally; Ctrl+wheel zooms.
- A trackpad pinch zooms by the gesture's factor with either setting.

## Pan (H)

- Drag moves the view; the middle button pans with any tool. Pan never
  changes the document.

## Navigator and scrollbars

- The scrollbars cover the page plus a margin of one page in every
  direction.
- The navigator button sits in the corner between the two scrollbars.
  Pressing and holding it opens a pop-up with a thumbnail of the current
  page (longest side 220 px, aspect kept) and a blue rectangle for the
  visible area. While the button is held, moving the pointer over the
  thumbnail centres the view on that point; releasing closes the pop-up.
  The thumbnail is rendered once per opening at
  `dpi = 220 / page width in mm * 25.4`.

## Checks

- Given 100 % zoom, when the zoom-in step runs, then the zoom is 150 %.
- Given a click with the Zoom tool at the page centre, then the page
  centre stays under the pointer and the zoom doubles.
