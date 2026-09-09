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
| LW (world map) | d4_c10 | d4_c13 |

A level is matched to its tileset by the world id in its `IFFP` string
(`SDIFF:L<n>BM-IFF`), **not** by disk position — on disk 4 the L6 levels precede
their tileset.

## Palettes

Each world's 32-colour OCS palette (12-bit `0x0RGB`, expanded `nibble×17`,
colour 0 = black) lives in that world's **sprite bank** — the ~71–72 KB object
chunk that sits just before the world's tileset — at a fixed offset `0x11300`
(for world 6, whose bank is `d4_c01`, at `0xC7FE`). The `P41A` chunks are
*unrelated* graphics (title/menu/bonus pictures) and are **not** the level
palettes — using them was the earlier bug that made worlds come out the wrong
colours.

The base palette is identified by the game's signature **grey shading ramp**
`0x111, 0x222, 0x444, 0x0AAA` sitting consecutively near the top of the 32
entries; `base_palette()` scans a chunk from the end for the black-0 window
carrying that ramp (the sprite bank also holds colour-cycle variants earlier in
the file). Verified against a real in-game screenshot: **World 2 renders as the
grey stone "Spooky Castle"**, World 1 as a blue-sky grassland, etc.

Palette confirmed accurate for worlds 1–6; the bonus (`LB`) and world-map (`LW`)
special levels reuse a world tileset as a best-effort fallback.

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
