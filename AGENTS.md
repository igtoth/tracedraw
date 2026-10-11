# AGENTS.md: guide for AI agents and contributors

TraceDraw is an open-source vector editor in **Rust only**: native egui/eframe
desktop app and the same app compiled to WebAssembly for the browser. No
Electron, Tauri or webview.
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
3. **Clean-room and independence.** TraceDraw stands on its own: never
   name other graphics editors or their makers anywhere in the repo (code,
   comments, docs, commit messages, packaging, file names), and never
   describe TraceDraw or a feature as modelled on another program.
   Features carry our own names: generic terms (Fill, Blend, Contour,
   Envelope, Weld) are fine, distinctive product or feature names of other
   software are never used. File formats go by their usual names (`.cdr`,
   `.ai`, `.psd`, a format owner's name in a file dialog filter, a header
   string a format requires) only to describe compatibility; copyright
   notices that a license requires stay in `ATTRIBUTION.md`. Never decompile any program and never use
   decompiled code or dumps, not even in conversations with AI tools.
   Format readers come from public format notes and from inspecting files
   we own; never port code from libcdr or any other implementation.
   Record each piece of format knowledge, its source and date in
   `CLEANROOM.md` and the details in `docs/cdr-format.md`. Bundle only
   original or openly licensed assets and list them in `ATTRIBUTION.md`.
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
before claiming a UI change works. `scripts/visual/smoke_edit.py` and
`scripts/visual/monkey.py` (random actions, `MONKEY_SEED`) must leave the
application running with no panic in the log after any interaction change.

## Browser build

The app must keep compiling for `wasm32-unknown-unknown`. File dialogs and
file access go through `apps/tracedraw/src/files.rs` (never `rfd` or
`std::fs` directly in UI code); anything that needs a desktop (processes,
temporary folders, system clipboard images) checks `files::WEB` or is
`#[cfg(not(target_arch = "wasm32"))]`. `std::time::Instant`,
`SystemTime::now`, `std::env::temp_dir` and `std::process::id` panic in a
browser: use `web_time` or avoid them. See `docs/behavior/web.md`.

## Feature status

`docs/features.md` is the measure of progress: every tool and menu item
with its status. Move a row to "works" only with a test or a checked file
behind it, and never add a feature without updating the row.

## Workflow

- `cargo test --workspace` before every commit.
- Commits carry a `Signed-off-by:` line (DCO, see `CONTRIBUTING.md`).
- `cargo build -p tracedraw` and `cargo clippy --workspace` must stay
  warning-free (CI runs clippy with `-D warnings` on the library crates,
  on the pinned toolchain 1.97.0; use the same version locally so the
  lint set matches).
- Keep `docs/roadmap.md` honest: a feature is "done" when it has tests
  and works on a real file, not when the menu item exists.

## Documentation duties

- A new tool or effect gets a page in `docs/behavior/` with its defaults,
  shortcuts and the formulas it uses, before its row in
  `docs/features.md` moves.
- User-visible strings go through the i18n table (`docs/i18n.md`), never
  as bare literals in the UI.
- Acceptance criteria in `docs/acceptance.md` are tests; when one changes,
  change the test.
