# Contributing to TraceDraw

Thank you for helping. Read `AGENTS.md` first: it holds the engineering
rules (commands, no panics on input, layering, tests, units, language).
This file covers what every contribution must also respect.

## Clean contributions

- Send only work you wrote yourself, or work under a license compatible
  with MIT OR Apache-2.0 that you have the right to submit, with its
  origin stated in the pull request.
- Never send code, data or assets that were decompiled, disassembled or
  dumped from any program, or copied from proprietary software, its
  documentation or its installation files. This includes material
  produced by AI tools from such sources.
- Never port code from other implementations of a file format. Write it
  from public format notes and from files you own, and record what you
  learned, its source and the date in `CLEANROOM.md`.
- Bundle only original or openly licensed assets (icons, fonts, images,
  palettes, templates, test files) and list each one in
  `ATTRIBUTION.md` with its author, source and license.
- TraceDraw stands on its own: do not name other vendors or their
  products in code, comments, docs, commit messages or file names, and do
  not describe a feature as modelled on another program. Give features
  descriptive names of their own.

## Sign-off (DCO)

Every commit must carry a sign-off line:

```text
Signed-off-by: Your Name <you@example.com>
```

`git commit -s` adds it. By signing off you certify the Developer
Certificate of Origin 1.1 (https://developercertificate.org/): that you
wrote the contribution or otherwise have the right to submit it under
the project's licenses, and that you understand the contribution and
your sign-off are public and kept with the project.

Pull requests with unsigned commits, or with material whose origin is
unclear, are not merged.

## Workflow

- Run `cargo fmt --all`, `cargo clippy --workspace --tests` and
  `cargo test --workspace` before pushing; the build must stay free of
  warnings.
- Every change comes with tests; format code gets round-trip and
  malformed-input tests.
- User-visible strings go through the translation files (`docs/i18n.md`).
- A new tool or effect gets a page in `docs/behavior/` and a row in
  `docs/features.md`.

## License

Contributions are licensed under MIT OR Apache-2.0, like the rest of the
project.
