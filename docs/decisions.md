# Decisions

Architecture and product decisions, each with the reason and what would
make us revisit it. Newest at the bottom.

## D1. Language and UI toolkit: Rust, egui/eframe

Single native binary per platform, no runtime, no webview. egui is
immediate mode, which keeps the UI a pure function of the document and
removes a whole class of state bugs. Revisit if egui cannot reach the
text-editing quality needed for paragraph text in place; the fallback is
a custom text widget on top of our own shaping, not a toolkit change.

## D2. Rendering: CPU first (tiny-skia), GPU later (wgpu)

The scene model and the rasteriser are separate. tiny-skia gives exact,
deterministic pixels on every platform, which the visual regression
tests need. A wgpu path with the same `render_page` contract is planned
for large documents; the trigger is the performance budget in
`docs/acceptance.md` being missed at 5000 objects.

## D3. Text: fontdb + rustybuzz + ttf-parser, own layout

Shaping is OpenType-complete through rustybuzz (HarfBuzz port). Line
breaking, justification and hyphenation are ours; hyphenation uses the
TeX pattern dictionaries (LGPL/MIT hyph files) per language. Type 1
fonts are out of scope; convert them to OpenType.

## D4. PDF and PostScript: hand-written PDF writer, EPS from the same core

No PDF crate dependency: the writer is small, auditable and emits only
what we need (shadings, images, ExtGState, CMYK). PDF/X conformance is
a validation layer on top (output intent, no transparency for X-1a).
PostScript output is a Level 3 subset generated from the same path and
fill primitives.

Reading PDF is the other way round: the object layer (xref, object
streams, filters, encryption) comes from `lopdf`, a pure-Rust crate,
because that part is large and well tested there; the content-stream
interpreter that turns operators into our objects is ours
(`crates/io/src/pdf_import.rs`), so what we support and how we
approximate (clips as clip frames, shadings as gradients) is under our
control and documented in `behavior/pdf-import.md`.

EPS import runs a PostScript interpreter of our own
(`crates/io/src/eps_import.rs`): the language core is small, the files
drawing programs write depend on their prologs running, and an
interpreter is the only way to read them without a dependency on an
external engine. Fonts are not rasterised; text stays text.

Windows metafiles (EMF, WMF) are read and written by our own GDI record
player and writer (`crates/io/src/emf.rs`) from the public format
specifications: the record set that drawing programs emit is small, and
owning both directions keeps round trips exact (text stays text,
bitmaps keep their alpha, clips become clip frames).

## D5. Colour management: a pure-Rust ICC engine

`core/icc.rs` reads ICC v2 and v4 profiles (matrix/TRC, `mft1`, `mft2`,
`mAB`/`mBA` with curves, matrices and CLUTs) and builds transforms with
the four rendering intents and black point compensation. It replaces the
earlier plan of binding `lcms2`: no C dependency, the same binary on every
platform, and the parser is small enough to audit. The engine is
installed through `color::engine::install`; without loaded profiles the
built-in conversions apply: sRGB, and for CMYK the press model in
`core/press.rs`, a Yule-Nielsen modified Neugebauer model whose
constants were fitted to a coated web offset CMYK space (see
`docs/behavior/colour-management.md`).
Profiles are not bundled beyond the built-in sRGB: the user loads the
`.icc` files they are licensed to use from the Colour Management dialog,
and the paths persist in the settings. Checked against a real output
profile (a US web coated CMYK profile embedded in a 2019 `.cdr`): process
cyan converts to sRGB (0, 174, 239), magenta to (236, 0, 140), yellow to
(255, 242, 0), black to (35, 31, 32).

## D6. Licensed content is not included

Licensed spot colour libraries
and third-party clipart, font and photo libraries are licensed and do
not ship. Replacements:

- Default palettes: RGB, CMYK, Grayscale, a process palette named by
  CMYK values, and a web-safe palette, all generated.
- User palettes: import `.ase`, `.gpl`, `.aco` and our own `.tdpal`
  (JSON). A licensed library a user owns can be loaded that way and
  stays on their machine.
- Content: only system fonts and user folders; an optional index of
  OFL fonts and CC0 clipart sources can be added later as links, not
  as bundled content.

## D7. Everything is a command

See `AGENTS.md`. The reason is undo, scripting (macros are command
scripts, see D9), the CLI and tests all sharing one code path.

## D8. Effects are baked today, live tomorrow

Blend, contour, distort, extrude and envelope currently produce static
geometry. The model will gain an `effects: Vec<Effect>` on `Shape`,
evaluated at render time, with the source geometry kept. Baked results
remain available as "Break Effect Apart". This is tracked in
`docs/features.md` as "baked".

## D9. Macros: command scripts, not VBA

A macro is a JSON list of commands with parameters, recorded from the
Undo history and replayed through the engine. A small expression layer
(selection, loop over selected objects) comes later. No VBA, no
embedded interpreter in the first versions.

## D10. User interface languages

Twelve languages from day one of the i18n layer: English, Mandarin
Chinese, Hindi, Spanish, French, Modern Standard Arabic, Bengali,
Russian, Brazilian Portuguese, Indonesian, German, Japanese. Strings
live in per-language TOML files; see `docs/i18n.md`.

## D11. Behaviour documentation precedes implementation of complex tools

Extrude, Blend, Area Fill, Mesh Fill, Lens, Bitmap tracing and colour
management each get a page in `docs/behavior/` with defaults, shortcuts
and the formulas we use before the code lands.

## D12. Acceptance is measurable

Visual regression with a tolerance, round trips and performance numbers
are tests that run in CI; see `docs/acceptance.md`.

## D13. The browser build is the same code

The web version is the desktop app compiled to `wasm32-unknown-unknown`,
not a second front end. Platform differences sit behind two small
modules: `files.rs` (file dialogs and file access: the system file
system natively; uploads kept in memory and downloads in a browser) and
`web.rs` (start-up on a canvas, downloads, local storage, the shortcut
guard). Fonts are bundled at build time because browsers expose no
system fonts; only freely redistributable fonts are bundled (`web/FONTS.md`).
