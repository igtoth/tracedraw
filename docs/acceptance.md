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
  any panic. The `.cdr`, PDF, AI, EPS, DXF, PSD and SVG readers pass
  hundreds of iterations on the files in the test corpus.
- Fuzzing of the `.cdr` reader (`cargo fuzz`) with the RIFF and ZIP
  targets: no panic, no allocation over 1 GB, on 10 minutes of input.
- Opening any file yields a document, possibly empty, with a warning
  list; this is a unit test on truncated and corrupted fixtures.

## Behaviour parity

- Each page in `docs/behavior/` lists checks ("given, when, then") that
  are unit tests in the owning crate. A parity row moves to "works" only
  when those tests pass.
