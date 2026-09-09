# Superfrog RE — toward a map editor

Reverse-engineering the disk, packing, and level formats of **Superfrog**
(Team 17, 1993, Amiga) with the goal of building a **map editor**.

This project works on the four Superfrog floppies as
`Superfrog (1993)(Team 17)(Disk N of 4)[cr CSL].zip` (each containing one 880 KB
`.adf`). **Those disk images are copyrighted and are NOT included** (git-ignored) —
supply your own copies in the repo root before running the tools.

> **Continuing / fresh start:** run the [Quickstart](#quickstart). `build/` (adf,
> decompressed chunks, rendered PNGs) is git-ignored and must be regenerated with
> `extract_disks.py` → `unpack.py` (with your own disk zips in place) before the
> render/secret/annotate tools will work. The reverse-engineering findings are in
> [`docs/`](docs/).

## Status

| Milestone | State |
|-----------|-------|
| Disk format identified (custom trackloader, no AmigaDOS FS) | ✅ |
| `ATN!` container format cracked | ✅ |
| `ATN!` decompressor working (via 68k emulation of the game's own routine) | ✅ 103/106 chunks |
| All chunks extracted & classified (levels / gfx / exe / anim / text) | ✅ |
| **T7MP level format decoded** (dimensions, palettes, tilemap `BODY`) | ✅ |
| `BODY` cell split: tile index = `cell & 0x3FF`, flags = `cell >> 10` | ✅ |
| Tileset graphics decoded (840× `16×16` 5bpp tiles per world) + palettes | ✅ |
| **All 29 levels rendered to PNG** (`build/levels/`, + montage) | ✅ |
| Collision flags decoded; **secret passages (fake walls) found** — 144 across the game | ✅ |
| Map editor (view + edit + repack `ATN!` + rebuild `.adf`) | ⬜ next |

**29 levels** found (all `300 × 84` blocks), stored as `ATN!`-packed `T7MP` files.

## Quickstart

```bash
python3 -m venv venv && . venv/bin/activate
pip install -r requirements.txt

python tools/extract_disks.py      # zips -> build/adf/*.adf
python tools/unpack.py             # -> build/decomp/*.bin  (all chunks, classified)
python tools/t7mp.py build/decomp/d1_c21_level_53588.bin   # inspect one level
python tools/render_levels.py      # -> build/levels/*.png  (all 29 levels + montage)
```

**29 rendered levels** land in `build/levels/` (full-res `4800×1344` PNGs, plus
`_montage.png`). L1–L5 are colour-accurate; L6 / world-map palettes are plausible
(see [`docs/06`](docs/06-graphics-and-rendering.md)).

`build/` is git-ignored and fully regenerated from the disk images by the tools.

## How extraction works (short version)

1. Each disk is a back-to-back sequence of sector-aligned **`ATN!` chunks**
   (`magic`, `decomp_size`, `comp_size`, compressed data + trailer).
2. `ATN!` is a custom cruncher not handled by public tools. Instead of porting
   it, we **run the game's own decompressor** (68k routine at disk-1 offset
   `0x400+0x2da`, found by its `cmpi.l #$41544e21,(a0)+` magic check) on a
   Musashi CPU core (`machine68k`) — exact by construction.
3. Decompressed level files are **`T7MP`** — a flat `[tag][u32 len][data]`
   format whose `BODY` tag is an `XBLK × YBLK` grid of 16-bit tile indices.

Full analysis in [`docs/`](docs/) (read in order):
- [`01-disk-format.md`](docs/01-disk-format.md) — trackloader disks, `ATN!` container
- [`02-boot-chain.md`](docs/02-boot-chain.md) — bootblock → stage-2 → trackloader
- [`03-atn-codec.md`](docs/03-atn-codec.md) — the `ATN!` compression & how we decode it
- [`04-t7mp-level-format.md`](docs/04-t7mp-level-format.md) — the level file format (+ `BODY` cell split)
- [`05-chunk-inventory.txt`](docs/05-chunk-inventory.txt) — every chunk on every disk
- [`06-graphics-and-rendering.md`](docs/06-graphics-and-rendering.md) — tilesets, palettes (`0x11300`), world→tileset mapping
- [`07-secret-passages.md`](docs/07-secret-passages.md) — collision flags, fake-wall (flag 38) secret passages
- [`LEVEL-INDEX.md`](docs/LEVEL-INDEX.md) — level ids + tile addressing (`<level-id>:<col>,<row>`)

## Tools

| File | Purpose |
|------|---------|
| `tools/extract_disks.py` | unzip `.adf` images to `build/adf/` |
| `tools/atn.py` | `ATN!` container carver + emulation-based decompressor |
| `tools/unpack.py` | carve + decompress + classify every chunk |
| `tools/t7mp.py` | parse a `T7MP` level file (tags + tilemap/flags grid) |
| `tools/render_levels.py` | render all levels to PNG (tileset + palette + tilemap) |
| `tools/find_secrets.py` | find fake-wall secret passages, overlay + `SECRETS.md` |
| `tools/export_annotated.py` | per-level PNG with a coordinate grid + level id (tile addressing) |
