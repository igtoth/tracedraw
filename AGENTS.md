# AGENTS.md: guide for AI agents and contributors

TraceDraw is an open-source vector editor in **Rust only**: native egui/eframe
desktop app, later a WebAssembly build. No Electron, Tauri or webview.
The product name is **TraceDraw** in prose; machine names are `tracedraw`
(`tracedraw-core`, `tracedraw-cdr`, `.tdraw` files).

## Golden rules

1. **Everything is a command.** New user-visible behaviour is a variant of
   `tracedraw_core::Command`, applied through `Engine::run`. The UI only builds
   commands and reads the document; it never mutates it directly.
2. **Never panic on input.** Non-test code does not use `unwrap`, `expect`,
   `panic!`, `todo!` or raw indexing on data that came from a file or the
   user. Return `Result`, log a warning, skip the object. Opening any file
   must always yield a document.
3. **Clean-room.** Study the target design for behaviour and look only.
   Never name it, or any other vendor or product, anywhere in the repo:
   not in code, comments, docs, commit messages or packaging. Write
   "the target design". The `.cdr`
   reader comes from public notes and observed files; never port code from
   libcdr or any other implementation, only use their documentation to
   confirm layouts. Record what was confirmed, and with which file, in
   `docs/cdr-format.md`.
4. **Layering is enforced by review.** `core` has no workspace deps; `cdr`,
   `text` and `render` depend on `core` only; `io` depends on `core` and
   `render` (rasterised fills in exports); egui stays in `apps/`.
5. **Tests are the gate.** Every change comes with tests. Format code
   gets round-trip and malformed-input tests.
6. **Units.** Millimetres everywhere in the model. CDR files use
   1/254000 inch (= 0.0001 mm). Points only appear in text sizes.
7. **Code and comments in English.** Commit messages in English.
8. **No em dashes** in prose, code or commits.

## Workspace map

```text
crates/core   geometry.rs, document.rs, command.rs, engine.rs, style.rs,
              color.rs, nodes.rs, shaping.rs, effects.rs, id.rs
crates/cdr    container.rs (RIFF/ZIP), riff.rs (chunk tree), parse.rs
crates/text   font database, shaping, outlines
crates/render CPU rasteriser
crates/io     svg.rs, pdf.rs, lib.rs (native format)
apps/tracedraw    main.rs, app.rs, interaction.rs, canvas.rs, ops.rs,
                  tools.rs, tools2.rs, ui/ (menus, toolbar, toolbox,
                  dockers, palette, status, dialogs, icons)
apps/tracedraw-cli
```

## Visual checks

The app can be driven headlessly: start `Xvfb :99`, run the debug binary
with `DISPLAY=:99 LIBGL_ALWAYS_SOFTWARE=1`, send pointer and key events
with python-xlib (XTest) and capture with ImageMagick `import`. See
`scripts/visual/` for the driver and example scripts. Look at the PNGs
before claiming a UI change works.

## Parity

`docs/parity.md` is the measure of progress: every tool and menu item of
the target design with its status. Move a row to "works" only with a test or a checked
file behind it, and never add a feature without updating the row.

## Workflow

- `cargo test --workspace` before every commit.
- `cargo build -p tracedraw` must stay warning-free.
- Keep `docs/roadmap.md` honest: a feature is "done" when it has tests
  and works on a real file, not when the menu item exists.

## Documentation duties

- A new tool or effect gets a page in `docs/behavior/` with its defaults,
  shortcuts and the formulas it uses, before the parity row moves.
- User-visible strings go through the i18n table (`docs/i18n.md`), never
  as bare literals in the UI.
- Acceptance criteria in `docs/acceptance.md` are tests; when one changes,
  change the test.
