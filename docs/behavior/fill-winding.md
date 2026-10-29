# Fill winding

How the inside of a filled path is found where it crosses itself or
holds a subpath inside another (#1211).

- Default (`FillRule::Auto` on the object): text fills with the non-zero
  rule, so letters an envelope pushes together stay filled where they
  overlap; every other curve fills even-odd, so a combined curve keeps
  its holes and a complex star drawn in one stroke has an empty middle.
  A warped text outline is drawn non-zero whatever the object says.
- Fill winding (Properties docker, Fill section, a checkbox under the
  fill's settings): on sets the non-zero rule on the painted objects of
  the selection (`FillRule::NonZero`), off even-odd
  (`FillRule::EvenOdd`); the checkbox reads all, none or mixed over them.
  Command `SetFillRule`, undoable.
- Drawn by the rasteriser, written to SVG as `fill-rule` (`evenodd` when
  it applies, nothing for non-zero, SVG's default) and to PDF as the
  `f` / `f*` and `W` / `W*` operators; kept in `.tdraw` only when set.
  `.cdr` files do not set it (the format's field is not recorded in
  `docs/cdr-format.md`), so imported objects fill as the default says.

## Checks

- Given a five-pointed star drawn in one stroke, then its middle pixel is
  white with the default and with even-odd, black with Fill winding
  (test `fill_winding_fills_the_middle_of_a_crossing_star`).
- SVG keeps `fill-rule="evenodd"` for curves and drops it with Fill
  winding; PDF paints with `f*` and `f` (tests
  `fill_winding_drops_the_even_odd_rule`,
  `fill_winding_uses_the_non_zero_operator`).
- The setting round trips as `"fill_rule":"nonzero"` and is left out when
  unset (test `the_fill_rule_follows_the_kind_unless_set`).
