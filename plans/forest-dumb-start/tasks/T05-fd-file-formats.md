# T05 — fd-file-formats

**Phase:** 3 · **Depends on:** T02, T04 · **Weight:** heavy

## Goal

Understand the game's level and graphics files and prove it by rendering: every level map and
every sprite sheet becomes a picture in `build/`, and a person and a session point at the same
spot with `<level-id>:<col>,<row>` (DESIGN §2.6, playbook step 3's lessons). The formats stay in
Python tools this plan; the Rust readers arrive with the porting plan that consumes them.

## Design sections this implements

DESIGN §2.6, §3.1 (the tools are shell-side), §3.5 (renders live in `build/`).

## Files

- `games/forest-dumb/tools/render_levels.py` — renders every level of the six worlds
  (`_Mapa1-1.Mapa` to `_Mapa6-1.Mapa` and their `_Gen*`, `_MapaBobow*` companions) to
  `build/renders/`, one picture a level plus one sheet per sprite set.
- `games/forest-dumb/tools/test_render_levels.py` — its unittest.
- `games/forest-dumb/docs/05-file-formats.md` — the file structures: headers, offsets, the map
  cell encoding, what `_Gen*` and `_MapaBobow*` hold, with byte offsets proven by the renders.
- `games/forest-dumb/docs/06-graphics-and-rendering.md` — the tile and sprite formats, the
  palettes, how the pictures are drawn.
- `games/forest-dumb/docs/LEVEL-INDEX.md` — every level id, its file, and how a spot is named.
- `games/forest-dumb/CLAUDE.md` — index lines and the tile-addressing note, Superfrog's shape.

## Interface

`python games/forest-dumb/tools/render_levels.py [--out build/renders]` renders everything and
prints one line a picture. The documented formats are the other interface: a reader task in a
later plan implements from the doc alone. Tile addressing is `<level-id>:<col>,<row>`,
0-indexed, top-left origin, the level ids named in `LEVEL-INDEX.md`.

## Tests

- [ ] The unittest renders every level from the real files with no error and asserts each PNG is
      non-blank (pixel variance above a floor).
- [ ] The LEVEL-INDEX maps every level id to its file and its render, and a spot named as
      `<level-id>:<col>,<row>` resolves through the index.
- [ ] The parser reports a truncated or corrupt file with the offset, on synthetic bytes.

## Done when

- [ ] Every level map and every sprite sheet of all six worlds renders from the real disks, the
      pictures in `build/` (git-ignored, nothing derived committed).
- [ ] The two format chapters are written with offsets that the renders prove, and the
      addressing convention and LEVEL-INDEX are in place.

## Outside actions

None.
