# Forest Dumb Forever (tier 3)

Rules and an index of what is known about the game. Paths are from the repo root unless they are
links. A fact true of this game goes here even if it might hold elsewhere (root CLAUDE.md, rule 1).
Start from `toolkit/PLAYBOOK.md`, whose opening steps fill most of this file.

## What the game is

Forest Dumb Forever, © 1996 L.K. Avalon (read off the title screen, 2026-10-31), developer Eagle Soft: an Amiga platformer in the manner of
the first game. This project reverse-engineers it and rebuilds it in Rust on the toolkit. Only the
first conformance case is built so far, no playable engine. Source of everything below: `plans/amiga-toolkit-prep/NORTH-STAR.md`, J1.

## Its disks and where they live

Two AmigaDOS (OFS) `.adf` images in zips, kept outside the repo and never committed.
Lookup and file names: [docs/01-disk-location.md](docs/01-disk-location.md).
Layout: [docs/02-disk-format.md](docs/02-disk-format.md). Every file with size and SHA-1:
[docs/04-file-inventory.txt](docs/04-file-inventory.txt), written by `cargo run -p fd-tools -- extract` in
`engine/`, which extracts into `build/`. What is packed and how to unpack it (`ESL!` is PowerPacker's
format): [docs/08-packers.md](docs/08-packers.md).
Boot chain, the crack's programs and why the game asks for disk 2:
[docs/03-boot-chain.md](docs/03-boot-chain.md).
The main program's map, its file table and the first case's variable:
[docs/07-executable-map.md](docs/07-executable-map.md); names in [re/symbols.toml](re/symbols.toml), listing from
`tools/re_listing.py`.

Decided 2026-10-30: the cracked `[cr WT]` set is game two's source. The faithful bar's measuring stick
is this cracked release, whose executable may differ from the original's. An uncracked copy, if one
turns up, is the thing to compare it against.

## What's known (don't re-derive)

Inspected 2026-10-27 from the WHDLoad package and the disk set. Not yet reproduced by a tool in this
folder.

- Loads its files through `dos.library` from a floppy labelled `!EAGLE_SOFT!`: AmigaDOS files, no
  trackloader. The images' volume name is `FOREVER`; `!EAGLE_SOFT!` is a marker file on disk 2.
- Six worlds of level files: `_Mapa1-1` … `_Mapa6-1`, with `.Mapa`, `_Gen*`, `_MapaBobow*`.
- Executable `FOREST_DUMB_FOREVER`, 521 640 bytes, stored unpacked on disk: an AmigaDOS hunk
  executable of 5 hunks, hand-written 68k hitting the hardware, no AMOS or Blitz signatures.
- None of the Team 17 shared items apply: no ATN!, no T7MP, no Player 4.1A.
- Needs 368 KB chip and 0.5 MB other memory, which the oracle's A500 configuration provides.

## Addressing

`h4+$195c9` is byte `$195c9` of the main program's hunk 4, as in
[docs/07-executable-map.md](docs/07-executable-map.md), which gives each hunk's base on the oracle.

A spot is `<level-id>:<col>,<row>`, 0-indexed, top-left origin, 16-pixel cells, e.g. `W1-1:10,20`. Level
ids and their files: [docs/LEVEL-INDEX.md](docs/LEVEL-INDEX.md). `tools/render_levels.py --spot W1-1:10,20`
prints what is there; the pictures are in `build/renders/`.

## Bug-fix procedure

Not written yet: there is no engine to fix. It comes with the recorder, step 8 of the playbook.

## Notes

- [docs/05-file-formats.md](docs/05-file-formats.md): the level files, byte by byte.
- [docs/06-graphics-and-rendering.md](docs/06-graphics-and-rendering.md): tiles, sprites, the palette, the renderer.
- [docs/LEVEL-INDEX.md](docs/LEVEL-INDEX.md): every level id, its files and its render.

- [docs/frame-length.md](docs/frame-length.md): the measured frame length, 313 lines, dated.
- [engine/conform/README.md](engine/conform/README.md): the conformance runner `fd-conform` and its case format.
- [docs/ROADMAP.md](docs/ROADMAP.md): backlog, the one level 1 plan the owner wants after `forest-dumb-start`.

## The first case

`cases/first-case.toml` compares the engine's `mt_counter` (`fd-core`) with the original's from power-on,
state only, on every frame of the idle title. Run it from `engine/` with
`cargo run -p fd-conform -- run cases/first-case.toml`; it also runs in `test.sh`.

## Backlog

- First plan: walk `toolkit/PLAYBOOK.md`'s opening steps. It is a future `pir plan` run, with a
  slug starting `forest-dumb-`.
