# 06 — Graphics & level rendering

`tools/render_levels.py` renders every T7MP level to a full-resolution PNG
(`4800 × 1344`, = `300 × 84` blocks of `16 × 16`) plus a `_montage.png` overview,
into `build/levels/`.

## Tilesets

Each world's tileset is a **134,400-byte** chunk = **840 tiles**, each tile
`16 × 16` pixels at **5 bitplanes (32 colours, OCS/ECS)** stored **per-tile
plane-major** (5 planes × 16 rows × 2 bytes = 160 bytes/tile). Rendering a tileset
as a tile grid shows the terrain/platform/decoration blocks directly.

Tilesets appear once per world, in world order:

| world | tileset chunk | levels |
|-------|---------------|--------|
| L1 | d1_c18 | d1_c21–24 |
| L2 | d1_c28 | d1_c31–34 |
| L3 | d2_c09 | d2_c12–15 |
| L4 | d2_c19 | d2_c22–25 |
| L5 | d2_c29 | d2_c32–35 |
| L6 | d4_c07 | d4_c02–05, c14, c15, c17 |
| LB (bonus level, 640 tiles) | d2_c40 | d2_c42 |
| LW (final level) | d4_c10 | d4_c13 |

A level is matched to its tileset by the world id in its `IFFP` string
(`SDIFF:L<n>BM-IFF`), **not** by disk position — on disk 4 the L6 levels precede
their tileset.

## Palettes

Each world's 32-colour OCS palette (12-bit `0x0RGB`, expanded `nibble×17`,
colour 0 = black) is the one after the world's **loading picture** `LnLP`, at
offset `0x11300` (docs/10, checked against the game's copper list). The ~71–72 KB
chunk before each tileset is that picture, not a sprite bank; for world 6 it is
`d2_c45` (`L6LP`). `render_levels.py` takes W6's from `d4_c01` at `0xC7FE`, which is
the `L2ET` picture read two bytes out of line: its W6 renders are in the wrong
colours. The `P41A` chunks are music modules (docs/08) and are **not** the level
palettes — using them was the earlier bug that made worlds come out the wrong
colours.

The base palette is identified by the game's signature **grey shading ramp**
`0x111, 0x222, 0x444, 0x0AAA` sitting consecutively near the top of the 32
entries; `base_palette()` scans a chunk from the end for the black-0 window
carrying that ramp (the picture's bitplanes can hold palette-like runs earlier in
the file). Verified against a real in-game screenshot: **World 2 renders as the
grey stone "Spooky Castle"**, World 1 as a blue-sky grassland, etc.

Palette confirmed accurate for worlds 1–5. The bonus (`LB`) and final (`LW`)
levels have their own tilesets (`d2_c40`, 640 tiles, and `d4_c10`); the script
draws them with W6's as a fallback. docs/10 has the right ones.

## Other decoded chunk types

- `FORM…ANIM` (disk 3) — IFF ANIM cut-scene animations.
- `PAGE…` (d4_c06) — plain-text story/help page.
- `0x000003F3` — Amiga HUNK executables (the game code).
- Many raw 5-bitplane sprite/background banks (e.g. 320-wide planar images).

## Reproduce

```bash
python tools/extract_disks.py
python tools/unpack.py
python tools/render_levels.py     # -> build/levels/*.png + _montage.png
```
