# Interoperability test files

Synthetic drawings saved by TraceDraw in every `.cdr` version it writes,
for testing in the programs that read those versions. The results are
reported in the issues the bundles are posted to and recorded in
`docs/cdr-format.md`.

- `interop.tdraw`: the test drawing, made by
  `cargo run -p tracedraw-cli --example interop -- <dir>`: a rectangle
  with a fill and an outline, an ellipse, a filled curve, a line, a
  gradient, a group of two shapes, an open polyline, and artistic text
  (plain with accents, runs in bold and italic with a colour of their
  own, right aligned and centred), on an A5 landscape page.
- `interop-empty.tdraw`, `interop-shapes.tdraw`, `interop-text.tdraw`:
  parts of that drawing written by the same example (the page alone,
  the shapes without the texts, the texts alone).
- `interop-0005/`: written on 2026-10-30 (issue #1139, where the
  version 8 file of interop-0004 crashed a 2010 release on opening while
  7 and 9 to 15 opened), and `interop-0005.zip` holding all 17 files:
  - `vNN_drawing.cdr` (7 to 15): the drawing from branch `fix-1139-v8`
    (main at 5e831feb with its change): version 8 without the set11
    table in the style table, as the public notes give the usual version
    8 layout (800); versions 7 and 9 to 13 are the interop-0004 files
    apart from the time saved; 14 and 15 come from the current writer.
  - `v08_empty.cdr`, `v08_shapes.cdr`, `v08_text.cdr`: the three parts
    in that layout.
  - `v08set11_drawing.cdr` (the interop-0004 file), `v08set11_empty.cdr`,
    `v08set11_shapes.cdr`, `v08set11_text.cdr`: the same with the set11
    table, as the program saves version 8 (written by main at aef7959a);
    they differ from the files above in the style table alone.
  - `v08bidi_drawing.cdr`: `v08set11_drawing.cdr` under the header of
    the bidirectional variant (form type `cdr8`, `vrsn` and `sumi` 801),
    the variant the public notes give set11 to.

  If every set11 file crashes and the others open, the style table is
  the cause; if the shapes crash in both layouts, the rectangles' single
  radius (version 7's layout; 9 has four) is.
- `interop-0004/vNN_drawing.cdr`: that drawing saved as version NN (7 to
  15) with `tracedraw-cli convert`, and `interop-0004.zip` holding all
  9 files. Written on 2026-10-29 from the branch of pull request
  `fix-1139-cdr-old-versions` (main at 21f22934 with its changes): the
  versions 7 to 13 layout corrected record by record against the same
  drawing saved by the program that reads those versions (issue #1139);
  versions 14 and 15 are byte for byte the files of interop-0003.
- `interop-0003/vNN_drawing.cdr`: that drawing saved as version NN (7 to
  27) with `tracedraw-cli convert`, and `interop-0003.zip` holding all
  21 files. Written on 2026-10-29 from `main` at 1c044358 (0.23.0) with
  the changes of pull request #1245 (palette ids after colours, the
  version 15 fill and outline layouts in version 14, argument 19170 on
  objects of versions 9 to 15).

Earlier bundles (`interop-0001`, `interop-0002`) are on the `interop`
branch.

The files are made by TraceDraw's own code from a drawing made for this
purpose; they belong to the project (MIT OR Apache-2.0, see
`ATTRIBUTION.md`).
