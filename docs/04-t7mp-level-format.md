# 04 — `T7MP` level format

A decompressed Superfrog level is a **`T7MP`** ("Team 17 MaP") file: a flat
sequence of chunks, each `[4-byte ASCII tag][u32 big-endian length][payload]`.
All 29 shipped levels are `53,588` bytes and `300 × 84` blocks.

## Layout (as shipped)

| Off | Tag | Len | Meaning |
|----:|-----|----:|---------|
| 0x000 | `T7MP` | 4 | form id; value = total file size (0xD154 = 53588) |
| 0x00C | `XBLK` | 4 | **map width in blocks** (300) |
| 0x018 | `YBLK` | 4 | **map height in blocks** (84) |
| 0x024 | `VERS` | 28 | version string |
| 0x048 | `COM1` | 28 | comment line 1 |
| 0x06C | `COM2` | 28 | comment line 2 |
| 0x090 | `REMX` | 4 | start / remembered X (blocks) |
| 0x09C | `REMY` | 4 | start / remembered Y (blocks) |
| 0x0A8 | `IFFP` | 64 | tileset reference, e.g. `SDIFF:L6BM-IFF` |
| 0x0F0 | `PALA` | 192 | palette A |
| 0x1B8 | `PALB` | 192 | palette B |
| 0x280 | `PALC` | 192 | palette C |
| 0x348 | `PALD` | 192 | palette D |
| 0x410 | `COLS` | 4 | colour count |
| 0x41C | `CCCL` | 64 | colour-cycling list |
| 0x464 | `IFFC` | 2048 | tile attribute / collision table |
| 0x6C6C | `BODY` | 50400 | **tilemap** |

Tag order/size is not guaranteed across versions — always parse by walking
`[tag][len]`, not fixed offsets. `tools/t7mp.py` does this.

## `BODY` — the tilemap  (decoded ✅)

`BODY` length `= XBLK × YBLK × 2 = 300 × 84 × 2 = 50,400` bytes exactly.
It is a row-major grid of **16-bit big-endian cells**, `XBLK` per row, `YBLK` rows.
Each cell splits as:

```
  tile index = cell & 0x03FF     # 0..839 — the world tileset has exactly 840 tiles
  flags      = cell >> 10        # 6 bits — collision / behaviour
```

The 10-bit index was confirmed because its maximum across the shipped levels is
**836 < 840**, matching the tileset tile count exactly (see below). Rendering with
this split produces coherent, recognisable levels. `tools/t7mp.py` exposes both
grids; `tools/render_levels.py` renders them to PNG.

The 6-bit `flags` field is collision/behaviour metadata (solid, hazard, water,
one-way, …) — the exact bit meanings still need cross-checking against `IFFC`.

## Palettes

The level's own `PALA..PALD` are empty in the shipped data; `IFFP`/`PALA` instead
hold a reference string `SDIFF:L<n>BM-IFF` naming the world tileset (`L1`..`L6`,
plus `LB` bonus and `LW` world-map). The real 32-colour palette lives in the
world's graphics chunks — see [`06-graphics-and-rendering.md`](06-graphics-and-rendering.md).

## Still open

- **No object/entity data in the level file.** A full tag walk shows a level is
  *only* the tags above — header + `IFFC` + `BODY` (terrain). There is **no chunk**
  for enemies, coins, extra-life frogs, moving platforms, level exits or bonus
  triggers. Where those are defined is not yet known (candidates: encoded in tile
  indices / `IFFC`, or a separate table in the game HUNK exes). **A map editor will
  need this** — it is the biggest missing piece for editing beyond terrain.
- Exact meaning of each `flags` bit (cross-check `IFFC`).
- `REMX/REMY` (plausibly the start position; values vary per level, e.g. 10,10 and
  34,34 — unverified), `COM1/2`, `VERS`, `CCCL` (colour-cycle index list, 32
  entries) semantics.
