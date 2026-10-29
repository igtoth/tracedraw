# Effects docker

Window > Dockers > Effects (or the "..." under the docker tabs) lists the
effect stack of the first selected object: its live effects (contour,
blend, distortion, envelope, perspective, extrude, bevel, block shadow,
symmetry, lens, transparency) in the order they apply, from the top
down. Code: `ui/effects_docker.rs`; the stack in
`tracedraw_core::document::Shape::effect_stack`.

Each row:

- The check box switches the effect on or off. An effect switched off is
  kept with the object, at its place in the stack (`parked_effects` in
  `.tdraw`), and is not drawn, exported or printed; switching it on puts
  it back where it was.
- The name, dimmed while the effect is off.
- Up and down arrows move the effect one place earlier or later in the
  stack (disabled at the ends).
- The cross removes the effect.

Every change is one undo step ("Effects"). Each effect is still edited
in its own panel or tool (Extrude, Contour, Envelope and so on).

Without a selection the docker says to select an object; an object
without effects says it has none.

## Checks

- `command::tests::effects_switched_off_keep_their_place_in_the_stack` (core)
- `ui::effects_docker::tests::the_stack_switches_moves_and_removes_effects`
