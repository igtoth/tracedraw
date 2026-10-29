# tracedraw-cdr-format

The structure of `.cdr` files, with no document model attached: the
containers (a bare RIFF file, or a ZIP package with its RIFF stream and
external data streams), the RIFF chunk tree with its compressed lists and
redirect records, the Embedded OpenType structure around embedded fonts,
and the compound files that hold embedded objects.

It is the bottom layer of TraceDraw's `.cdr` support. The crate above it,
`tracedraw-cdr`, turns the chunks into a drawing and writes drawings back.
This crate depends on nothing else in the workspace, so it can be used on
its own to inspect files (see `tracedraw-cli inspect`).

## Clean room

Everything here was written from public format notes and from inspecting
files we own. No program was decompiled and no code of any other reader was
ported. `CLEANROOM.md` at the root of the repository records where each
piece of format knowledge came from and when; `docs/cdr-format.md` records
what is confirmed and what is still assumed.

## License

MIT OR Apache-2.0, as the whole workspace (`LICENSE-MIT` and
`LICENSE-APACHE` in this folder).
