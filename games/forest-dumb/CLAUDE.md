# Forest Dumb Forever (tier 3)

Rules and an index of what is known about the game. Paths are from the repo root unless they are
links. A fact true of this game goes here even if it might hold elsewhere (root CLAUDE.md, rule 1).
Start from `toolkit/PLAYBOOK.md`, whose opening steps fill most of this file.

## What the game is

Forest Dumb Forever, © 1998 L.K. Avalon, developer Eagle Soft: an Amiga platformer in the manner of
the first game. This project reverse-engineers it and rebuilds it in Rust on the toolkit. Nothing of
it is built yet. Source of everything below: `plans/amiga-toolkit-prep/NORTH-STAR.md`, J1.

## Its disks and where they live

Two AmigaDOS (OFS) `.adf` images in zips, kept outside the repo and never committed.
Lookup and file names: [docs/01-disk-location.md](docs/01-disk-location.md).
Layout: [docs/02-disk-format.md](docs/02-disk-format.md). Every file with size and SHA-1:
[docs/04-file-inventory.txt](docs/04-file-inventory.txt), written by `cargo run -p fd-tools -- extract` in
`engine/`, which extracts into `build/`.

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

Not known yet; the first plan settles it.

## Bug-fix procedure

Not written yet: there is no engine to fix. It comes with the recorder, step 8 of the playbook.

## Notes

None yet besides the numbered docs/ chapters.

## Backlog

- First plan: walk `toolkit/PLAYBOOK.md`'s opening steps. It is a future `pir plan` run, with a
  slug starting `forest-dumb-`.
