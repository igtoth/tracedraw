# Acceptance criteria

Each criterion is a test in the workspace (or will be, with the test name
given). A feature is accepted when its criteria pass in CI on all three
platforms.

## Visual regression

- Reference PNGs live in `tests/visual/expected/`; `tests/visual` renders
  each `.tdraw` fixture at 96 dpi and compares.
- Metric: per-pixel RMSE over RGB, normalised to 0..1. Threshold 0.01
  (1%) per image, and no more than 0.5% of pixels differing by more than
  32/255 in any channel.
- Fonts: fixtures that contain text embed the font file next to them so
  the comparison does not depend on system fonts.
- Updating a reference requires a reviewed change in the same commit.

## Round trips

- `.tdraw`: save, load, compare the `Document` structurally (serde
  equality). Every command variant has a round-trip fixture.
- `.cdr`: open a fixture, export `.tdraw`, re-open, compare object count,
  bounding boxes (0.01 mm), fill and outline attributes. Fixtures per
  version in `tests/cdr/`.
- SVG and PDF: export, rasterise with an independent tool (resvg for
  SVG, pdftoppm for PDF, both in CI), compare to our render with the
  visual metric at 2% threshold.

## Performance (release build, CI runner, median of 5)

| Scenario | Budget |
|---|---|
| Open a `.tdraw` with 500 objects | under 300 ms |
| Open a `.cdr` with 500 objects | under 500 ms |
| Render a page of 1000 simple objects at 100% zoom, 1600x1000 | under 16 ms |
| Render a page of 5000 objects | under 100 ms |
| Undo or redo of any command on a 1000-object document | under 50 ms |
| Select all + move on 1000 objects (one frame) | under 16 ms |
| Export PDF of 1000 objects | under 1 s |
| Startup to first frame | under 1 s |

Benchmarks are `cargo bench` targets in `crates/render/benches` and
`crates/io/benches`; CI fails when a median exceeds its budget by more
than 20%.

## Robustness

- Mutation testing of every reader: `tracedraw-cli stress <file> [n]`
  feeds `n` damaged copies of a file (truncated, byte flips, cut spans,
  overwritten words) to the reader its extension selects and fails on
  any panic. The `.cdr`, PDF, AI, EPS, DXF, PSD, EMF, WMF and SVG readers pass
  hundreds of iterations on the files in the test corpus.
- Fuzzing of the `.cdr` reader (`cargo fuzz`) with the RIFF and ZIP
  targets: no panic, no allocation over 1 GB, on 10 minutes of input.
- Opening any file yields a document, possibly empty, with a warning
  list; this is a unit test on truncated and corrupted fixtures.

## Behaviour parity

- Each page in `docs/behavior/` lists checks ("given, when, then") that
  are unit tests in the owning crate. A parity row moves to "works" only
  when those tests pass.
- Interaction smoke run (`scripts/visual/run.sh scripts/visual/smoke_edit.py`): draw one of every
  box tool, a freehand curve and text; select all; group, ungroup,
  combine, break apart and convert to curves; drop shadow and
  transparency drags; zoom and pan; add a page; open every docker tab.
  The process must still be running at the end with no panic in the log
  (Ctrl+Q once closed the application through egui's default quit
  shortcut; it is Convert to Curves). The same sequence runs headless in
  `ops::smoke_tests`. `scripts/visual/monkey.py` (MONKEY_SEED, MONKEY_STEPS)
  runs random clicks, drags, keys and chords; seeds 1 to 6 with 150 steps
  survive with no panic.
- Headless UI tests run the egui interface without a window
  (`egui::Context::run_ui`): `ui::dialogs::tests` draws every dialog over
  a document with a rectangle, text and a bitmap and presses Enter in
  each; `ui::menus::tests` clicks every enabled row of all twelve menus
  (submenus inlined through the `replay` hook) against a mixed document
  of rectangle, ellipse, text, bitmap and table, skipping only the rows
  that open a native file chooser, another program or close the window.
  Both must pass with no panic and a page left in the document.
- `.cdr` writing: a document with a rectangle (radius, dashed outline,
  name, transparency), a radial fountain ellipse, a transformed curve with
  a CMYK fill, bold text, a group, a 2 x 2 bitmap and a second page is
  written and read back with every value within 0.05 mm (tests in
  `crates/cdr/src/write.rs`); truncated and bit-flipped copies of a written
  file never panic the reader.
