# Attribution

Every asset bundled with TraceDraw, with its author, source and license.
Code dependencies (Rust crates) keep their own licenses, recorded in
their packages; `cargo tree` lists them from `Cargo.lock`.

| Asset | Path | Author and source | License |
|---|---|---|---|
| Selawik Regular and Bold (interface font) | `apps/tracedraw/assets/fonts/` | Copyright 2015 Microsoft Corporation, the open-source Selawik project | SIL Open Font License 1.1, see `Selawik-OFL.txt` next to the fonts |
| Application icon (PNG sizes and `.ico`) | `assets/icon/` | Made for TraceDraw | MIT OR Apache-2.0 |
| Tool, toolbar and panel icons | painted in code, `apps/tracedraw/src/ui/icons.rs` | Made for TraceDraw | MIT OR Apache-2.0 |
| Default colour palette (colour names and values) | `apps/tracedraw/src/palette.rs` | TraceDraw contributors; common process colour tints with descriptive names | MIT OR Apache-2.0 |
| Spell-check core word list | `apps/tracedraw/lang/core-words.txt` | Made for TraceDraw | MIT OR Apache-2.0 |
| Hyphenation patterns | `crates/text/src/hyphen.rs` | Made for TraceDraw (a small Liang-style set) | MIT OR Apache-2.0 |
| Interface translations | `apps/tracedraw/lang/*.toml` | Made for TraceDraw | MIT OR Apache-2.0 |
| Sample drawing | `sample.tdraw` | Made for TraceDraw | MIT OR Apache-2.0 |
| Screenshots | `docs/screenshots/` | TraceDraw itself, captured with `scripts/visual/` | MIT OR Apache-2.0 |

Not bundled on purpose: licensed spot colour libraries, third-party
clipart, templates, fonts other than the one above, and colour profiles
other than the ones a user loads. Test files that belong to someone else
are never committed.
