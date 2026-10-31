# 05 — File formats

The level files, as `tools/render_levels.py` reads them. Offsets are into the **unpacked** files in
`build/unpacked/disk2/` ([08-packers.md](08-packers.md)). Graphics encoding: [06](06-graphics-and-rendering.md).
Naming a spot: [LEVEL-INDEX.md](LEVEL-INDEX.md). All values are big-endian. None of these files has a
header: position alone gives meaning, and the sizes below are the whole check.

## One level a world

A level is five files plus the shared sprites. `N` is the world, 1 to 6.

| File | Holds | Bytes | Layout | Loaded at |
|---|---|---|---|---|
| `_MapaN-1` | tile map | 53760 (W6 61440) | words, 96 to a column | hunk 0 `$16318` |
| `_GenN-1` | tile pictures | 160 x tile count | see chapter 06 | hunk 3 `$13540` |
| `_MapaN-1.Mapa` | collision map | 23040 (W6 30720) | bytes, 96 to a column | hunk 0 `$28f18` |
| `_GenN-1.Mapa` | collision shapes | 32 x shape count | see below | hunk 0 `$bda0` |
| `_MapaBobowN` | object map | 38400 | bytes, 96 to a column | hunk 0 `$cb38` |

Each load is a block in hunk 4 from `$19fc4` (name, then `move.l #dest,$9086`), one section a world in the
order 2, 3, 4, 5, 6, 1. The sprite files of chapter 06 load in the same sections.

## Tile map

All three maps are stored a column at a time: a level is 96 cells tall, and the next column follows the
last. The drawing loop at hunk 0 `$708` proves it, reading index `col * $60 + row` for the screen word at
`row * $2e0 + col * 2`. Read as 96 across, a level draws as terrain that does not join up at the tile edges.

Cell at (col, row): the word at `2 * (col * 96 + row)`. The value is a tile number, an index into `_GenN-1`.
The game goes through a table at hunk 0 `$3b988` holding `number * 160`, so it is a plain index. Every
level uses every tile of its set at least once (tested).

Columns: 280, W6 320. Past column 240 the five shorter levels hold only repeating filler rock, with no collision
under it.

## Collision map and shapes

Cell at (col, row): the byte at `col * 96 + row`, an index into `_GenN-1.Mapa`. Columns: 240, W6 320.

A shape is 32 bytes: 16 words, one a line, top line first, most significant bit leftmost. A set bit is
solid. The drawing loop at hunk 0 `$708` copies a cell's shape line by line into the screen's collision
plane, multiplying the cell by `$20`.

Shape 0 is fully solid in every level. The fully empty shape is number 2 in W1 and W2, number 1 in W3 to W6
(W6 also has empties from 18 up). The rest are slopes and partial cells.

The tile and collision maps share a grid. Proof, tested on all six levels: the tiles that fill most open
cells (76% to 88% of them) never occur in a solid cell, not once.

## Object map

Cell at (col, row): the byte at `col * 96 + row`, 0 for nothing. The file always fills 400 columns; the used
columns stay inside the level's collision columns (tested). The game scans one row across the 20 screen
columns (hunk 0 `$970`, index `(x / 16) * $60 + y / 16`) and also writes to it at run time (hunk 0 `$2e54`).

What the code proves, from the scan at hunk 0 `$9a8`. Its compares are signed, so `$87` to `$fe` would take
the creature branch; the highest value in any file below `$ff` is `$48`:

| Value | Group in the render | The code |
|---|---|---|
| `$ff` | end (white) | stops the scan |
| 1 to 6 | creature (red) | branches to `$abc`; the name is an inference from `$3a18`'s creature frames, not proven |
| 7 to `$1f` | ignored (grey) | the scan skips them; `$10`, `$11` occur |
| `$20` to `$3f` | element (green) | calls `$6882`; the draw loop at `$6384` subtracts `$20`: below 8 cycles `_ELEMENTY` frames, 8 and `$a` also animate, `$10` and up reach the flame and vanishing-block paths |
| `$40` and up | high (blue) | calls `$a06`, indexing tables at hunk 4 `$c80` and `$2580` by `(value - $40) * 2`; in the render they cover whole areas |

**Not established:** what each value does in play. Chapter 06's render boxes the groups and nothing more;
the porting plan settles the meanings.

## Not decoded here

`_NAPIS` (18720 bytes), `_CZOLOWKA` (31200), `_OPCJE` (4320) and `_END` (61440, stored unpacked) load before
play, at hunk 3 `$0`, `$4920`, `$c300`. They are the front-end's pictures, outside this task's levels and
sprite sets. `_BALLA` is the music ([08-packers.md](08-packers.md)).
