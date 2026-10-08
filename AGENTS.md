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
3. **Clean-room.** Study the editor for behaviour and look only. The `.cdr`
   reader comes from public notes and observed files; never port code from
   libcdr or any other implementation, only use their documentation to
   confirm layouts. Record what was confirmed, and with which file, in
   `docs/cdr-format.md`.
4. **Layering is enforced by review.** `core` has no workspace deps; `cdr`
   and `io` depend on `core` only; egui stays in `apps/`.
5. **Tests are the gate.** Every change comes with tests. Format code
   gets round-trip and malformed-input tests.
6. **Units.** Millimetres everywhere in the model. CDR files use
   1/254000 inch (= 0.0001 mm). Points only appear in text sizes.
7. **Code and comments in English.** Commit messages in English.
8. **No em dashes** in prose, code or commits.

## Workspace map

```text
crates/core   geometry.rs (kurbo re-exports + helpers), document.rs,
              command.rs, engine.rs, style.rs, color.rs, id.rs
crates/cdr    container.rs (RIFF/ZIP), riff.rs (chunk tree), parse.rs
crates/io     svg.rs, lib.rs (native format)
apps/tracedraw    main.rs, app.rs (UI + input), canvas.rs (render), view.rs
```

## Workflow

- `cargo test --workspace` before every commit.
- `cargo build -p tracedraw` must stay warning-free.
- Keep `docs/roadmap.md` honest: a feature is "done" when it has tests
  and works on a real file, not when the menu item exists.
