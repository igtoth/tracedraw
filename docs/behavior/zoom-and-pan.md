# Zoom and Pan

## Zoom tool (Z, F2)

- Click zooms in by 2 at the point; Shift+click or right-click zooms
  out by 2 (with this tool the right button never opens the context
  menu). Dragging a marquee zooms to fit that rectangle.
- Zooming to a rectangle (marquee, selection, all objects, page) centres
  that rectangle in the window, wherever it lies on the page, with a
  40 px margin: `zoom = min((W - 80) / w, (H - 80) / h)`.
- Double-clicking the tool button zooms to fit all objects (F4).
- F2 picks the tool for one zoom: after a click, a right click or a
  marquee the tool before it comes back (#1039,
  `pick_drag_tests::f2_zooms_once_and_gives_the_tool_back`); Z or the
  toolbox button keeps it until another tool is chosen. Shift+F2 zooms
  to the selection; F4 zooms to fit
  all objects; Shift+F4 zooms to the page.
- `+`/`=` and `-` zoom by 1.25 about the canvas centre (except that `+`
  duplicates the selection in place with a Pick tool, and with the Shape
  tool and nodes chosen `+` and `-` add and delete nodes); the zoom combo
  on the standard toolbar steps through 10, 25, 50, 75, 100, 150, 200,
  300, 400, 800 and 1600 % (beyond the last step the factor doubles).

## Mouse wheel

- Default (Options > Tools > Default action for mouse wheel: Zoom): the
  wheel zooms by 1.15 per notch about the pointer, in a glide over a few
  frames as a scroll moves (the smoothed wheel delta, 40 points a notch),
  not in one jump; Ctrl+wheel scrolls vertically; Alt+wheel scrolls
  horizontally (#766, `pick_drag_tests::a_wheel_notch_zooms_in_a_glide`).
- With the Scroll setting: the wheel scrolls vertically; Shift+wheel
  scrolls horizontally; Ctrl+wheel zooms.
- A trackpad pinch zooms by the gesture's factor with either setting.
- On a touch screen two fingers pan the view and a pinch zooms, with any
  tool (the first finger's drag is dropped).

## Pan (H)

- Drag moves the view; the middle button pans with any tool. Pan never
  changes the document.

## Navigator and scrollbars

- The scrollbars cover the page plus a margin of one page in every
  direction; their layout and buttons are in `drawing-window.md`.
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
- Given an object far from the page origin, when zooming to fit it, then
  its centre is the window centre (`view::tests::fit_centres_rectangles_anywhere_on_the_page`).
