# Properties docker

The Properties docker shows the selected objects' settings in sections,
each with a header bar that opens and closes it: Outline, Fill,
Transparency, the kind's own section (Rectangle corners, QR code,
Character for text), FX and Summary. Closed sections are remembered.
`mixed-selection.md` says how values that differ between the selected
objects are shown; `outline-pen.md`, `gradient-fill.md`,
`vector-pattern-fill.md` and `transparency.md` have the sections'
controls.

## Fill and Outline (#1200)

Besides the editors described in `gradient-fill.md` and `outline-pen.md`,
the Fill section ends with Overprint fill and Edit Fill..., the Outline
section with Overprint outline and Outline Pen...; the checkboxes read
all, none or mixed over the painted objects of the selection and set them
all (`overprint_state`, `App::set_overprint`, test
`overprint_is_set_from_the_docker`).

## Summary (#1202)

Figures about the selection, read-only, in the drawing's units and ruler
coordinates:

- Selection: how many objects are selected.
- Layer: the names of the layers holding them, each once (an object
  inside a group or a clip frame counts for its layer).
- Type: the kinds of the objects, each once.
- Selection dimension: width x height of the selection's bounds.
- Selection centre: the middle of those bounds.
- Rotation centre: where the centre of rotation was moved to in rotate
  mode, else the selection's middle (`App::rotation_center`).
- One line per object: kind, id and name.

Then two controls that act on the selection:

- Wrap paragraph text: None or one of the wrap styles (Contour or Square,
  text on the left, on the right, on both sides, above and below); picking
  a style turns wrapping on; the text wrap offset below is enabled while a
  style is on. The same settings as the property bar's Wrap Paragraph
  Text button and the context menu (`text-wrap.md`).
- Lock Object: locks the selected objects (Object > Lock Object); they
  leave the selection, and Object > Unlock All Objects frees them.

## Checks

- Given a selected 50 x 30 mm rectangle at (10, 10), then the Summary
  reports 1 object, 1 layer, its kind, 50 x 30 and the centre (35, 25)
  in ruler coordinates, with the rotation centre at the same point; with
  an ellipse added to the selection, 2 objects, 1 layer and 2 kinds; with
  nothing selected, no figures (test
  `the_summary_section_describes_the_selection`).
