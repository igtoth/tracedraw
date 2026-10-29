# CLAUDE.md

Read `AGENTS.md` first and follow it. It is the source of truth for every
AI agent and contributor: the golden rules, the workspace map, the visual
checks, the browser build and the feature status. In particular the
clean-room rules (rule 3): never decompile or disassemble any program,
never use decompiled code or dumps, and never port code from libcdr or any
other format implementation. `CLEANROOM.md` records where each piece of
format knowledge came from. This file only adds the day-to-day commands and
the agent tooling; where it is silent, `AGENTS.md` wins, and where they ever
disagree, `AGENTS.md` wins.

## What TraceDraw is

TraceDraw is an open-source vector editor written in **Rust only**: a native
egui/eframe desktop app and the same app compiled to WebAssembly for the
browser (no Electron, Tauri or webview). The goal is a near 1:1 recreation
of the classic professional vector-editor desktop workflow: the same tools,
dockers, menus and interactions a working designer expects, down to the
details. It reads and writes `.cdr` and the native `.tdraw` format. Honor
rule 3 everywhere: describe features with our own or generic names (Fill,
Blend, Contour, Envelope, Weld), never by another program's distinctive
names, and never describe a feature as modelled on another program. Format
names (`.cdr`, `.ai`, `.psd`) are used only to state compatibility.

## Commands

Native desktop (a recent stable Rust, 1.89 or newer; CI builds with 1.97):

```sh
cargo run --release -p tracedraw                  # the editor
cargo run --release -p tracedraw -- drawing.cdr   # open a file
cargo run --release -p tracedraw-cli -- --help    # the command-line tool
```

Tests (every change comes with tests; rule 5):

```sh
cargo test --workspace                            # the full suite (~2,200 tests)
cargo test -p tracedraw-cdr                       # one crate
cargo test --release -- --ignored frame_times --nocapture  # frame benchmarks (lag is a bug, rule 9)
```

Lint and format before pushing:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets
```

Browser build (`wasm32-unknown-unknown`; needs `wasm-bindgen`, `wasm-opt`
from binaryen, and the `web` profile). The script builds into `./site`:

```sh
rustup target add wasm32-unknown-unknown          # once
sh scripts/web/build.sh                           # builds ./site
python3 -m http.server -d site 8080               # then open http://localhost:8080
```

The app must keep compiling for `wasm32-unknown-unknown` on every change.
See `docs/behavior/web.md`.

## Layout

Workspace crates (layering is enforced by review, rule 4: `core` and
`cdr-format` have no workspace deps; `cdr` depends on `cdr-format` and
`core`; `text` and `render` depend on `core`; `io` depends on `core` and
`render`; egui stays in `apps/`):

```text
crates/core         geometry, document, command, engine, style, color, nodes, shaping, effects, id
crates/cdr-format   container (RIFF/ZIP), riff, eot, ole: file structure, no document model
crates/cdr          parse (chunks to Document), write, text, cmx
crates/text         font database, shaping, outlines
crates/render       CPU rasteriser
crates/io           svg, pdf, lib (native .tdraw)
crates/vba, crates/vba-project   scripting support
apps/tracedraw      main, app, interaction, canvas, ops, tools, tools2, ui/ (egui)
apps/tracedraw-cli  headless command-line tool
```

Everything is a command (rule 1): new user-visible behaviour is a variant of
`tracedraw_core::Command` applied through `Engine::run`; the UI only builds
commands and reads the document. Never panic on input (rule 2): return
`Result`, log and skip; opening any file must always yield a document. Units
are millimetres in the model; `.cdr` uses 1/254000 inch; points only in text
sizes (rule 6). Code, comments and commit messages in English (rule 7), no em
dashes (rule 8). `docs/features.md` is the measure of progress: move a row to
"works" only with a test or checked file behind it.

## Rust to WASM conventions

- File dialogs and file access go through `apps/tracedraw/src/files.rs`,
  never `rfd` or `std::fs` directly in UI code.
- Anything needing a desktop (processes, temp folders, system clipboard
  images) checks `files::WEB` or is `#[cfg(not(target_arch = "wasm32"))]`.
- `std::time::Instant`, `SystemTime::now`, `std::env::temp_dir` and
  `std::process::id` panic in a browser: use `web_time` or avoid them.
- Keep the `web` profile small (`opt-level = "s"`); the browser build runs on
  one thread, so heavy work (effects, tracing, large exports) blocks the page.

## Testing drag interactions (read this before "verifying" a drawing tool)

The headless visual harness in `scripts/visual/` drives the **native** app
under Xvfb with python-xlib XTest (real X pointer and key events) and
captures with ImageMagick `import`. That path does deliver true press,
move and release, so native drags work there:

```sh
cargo build -p tracedraw                          # debug binary the harness runs
scripts/visual/run.sh scripts/visual/smoke_edit.py
scripts/visual/run.sh scripts/visual/monkey.py    # random actions, MONKEY_SEED=<n>
scripts/visual/window_check.sh ./target/debug/tracedraw
```

Look at the PNGs in `$TRACEDRAW_SHOTS` before claiming a UI change works, and
confirm no `panic` in the app log after any interaction change.

**Drag interactions on the WASM canvas are not captured by synthetic mouse
automation.** Dispatching synthetic `mousedown`/`mousemove`/`mouseup` (or
pointer) events at the canvas does not reproduce a real drag for the tools
that depend on one: Rectangle, Ellipse, Freehand, Bezier, Shape-tool node
drags, marquee select, and anything else where you press, move and release to
define geometry. A naive synthetic drag will look like it ran and produce
nothing or the wrong shape. Test those tools in the browser **manually**, or
with a **Playwright**-driven browser (real input events, pre-installed
Chromium at `/opt/pw-browsers/chromium`, launch `--headless --no-sandbox`),
not with hand-rolled synthetic event dispatch. Clicks and keyboard shortcuts
are fine with synthetic events; it is the press-move-release drags that are
not.

## Tooling

Agent skills live in `.claude/skills/`, installed from
[`addyosmani/agent-skills`](https://github.com/addyosmani/agent-skills) (MIT)
with `npx -y skills add addyosmani/agent-skills --agent claude-code`;
`skills-lock.json` pins the versions. Re-run that command to update. The set
covers specs, planning, incremental implementation, TDD, debugging,
performance, frontend/UI, security hardening, CI/CD, shipping and more; list
them with `/using-agent-skills` or by reading each `SKILL.md`.

Native Claude Code already covers some needs, so we do not duplicate them with
external skills: **`/code-review`** and **`/security-review`** for auditing the
pending diff, and **`artifact-design`** for design work. Prefer those over any
overlapping third-party skill.

**Next step (needs explicit approval, not installed yet):** REA
(`morluto/rea`) is the most relevant tool for the TraceDraw goal and a
recommended addition. Install it only from its official GitHub/npm source
(fake trojaned copies have circulated), and only after Ighor approves.
