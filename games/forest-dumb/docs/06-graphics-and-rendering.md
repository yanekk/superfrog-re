# 06 — Graphics and rendering

How `tools/render_levels.py` draws the files of [chapter 05](05-file-formats.md). Run
`venv/bin/python games/forest-dumb/tools/render_levels.py [--out build/renders]`, after `fd-tools extract` and
`unpack.py --all`.

## The bitmap format

Everything is five bitplanes, **interleaved line by line**: one line of plane 0, then that line of planes
1 to 4, then the next line. Bit 7 of a byte is the leftmost pixel; plane 0 is the low bit of the colour
number. That lets the blitter copy from these files like from the game's screen. Reading them plane after
plane draws horizontal stripes, which is how this was found.

## Tiles

`_GenN-1`: 160 bytes a tile, 16 x 16 pixels: 16 lines of 5 planes of 2 bytes. Tile `n` starts at `160 * n`.

## Sprite sets

Frames sit side by side in a bitmap, like a screen. Bytes a line is per plane.

| Sheet | Files | Bytes a line | Frame | Frames | Addressed by (hunk 0) |
|---|---|---|---|---|---|
| `StworkiNew` | `StworkiNew.rawm`, `.mask` | 40 | 32 x 32 | 70, in rows of 10 | tables from `$3a18`, `$3658` |
| `ELEMENTY` | `_ELEMENTY.RAWM`, `.MASK` | 40 | 16 x 16 | 40, two rows of 20 | `$66f2`: frame number times 2; how the second row is reached is not traced |
| `Plomien1-3` | `_Plomien1-3` | 8 | 32 x 32 | 2 | `$6508`: animation bit times 4 |
| `ZNIKAJKA1-3` | `_ZNIKAJKA1-3` | 24 | 32 x 16 | 6 | table `$63ec`: 0, 4, ... 20 |
| `WYBIJACZKA`, `2`, `3` | `_WYBIJACZKA`, `2`, `3` | 4 | 32 x 32 | 1 | `$23e6`: blit modulo `$2a` on a `$2e` line |

- `StworkiNew` frames 42 to 69 hold no drawing, one flat colour under a set mask. The tables from `$3a18`
  hold byte offsets, stepping 4 a frame and 6400 a row; creature kind 6's table points at blank frame 68.
- A mask has the size and layout of its picture, one plane repeated five times (tested). The render paints
  magenta where the mask is clear. 13 bytes of `_ELEMENTY` draw outside the mask; `StworkiNew` none.
- `WYBIJACZKA` is loaded twice, to hunk 4 `$9618` and `$9898`, and `$23dc` picks a copy by a flag.
- Which world loads which set of `_ZNIKAJKA`, `_WYBIJACZKA` and `_Plomien`: 1 in W1 and W2, 2 in W3 and W4,
  3 in W5 and W6. These pictures carry their world's background, so they have no mask file.

## The palette

Level colours are 32 pairs of `COLORnn` register and 12-bit value in hunk 1's copper list, file offset
`$5e564 + $404`. Levels start by copying `$384` (W1, W2, hunk 1 `$484`) or `$404` (W3 to W6, `$49e`) over
`$304`. The two blocks hold the same values, so all six worlds share one palette. The tool reads it from the
main program and checks each register number. Colours are not written down here: they are disk-derived.

## What the renderer writes

Into `build/renders/`, git-ignored, 41 pictures: per level `W<n>-1.png` (the map), `-collision.png` (white is
solid), `-objects.png` (the map with object cells boxed by group, chapter 05), `-tiles.png`, `-shapes.png`;
and `sprites-<sheet>.png`. One map pixel block is one cell, so `<level-id>:<col>,<row>` is at pixel
`(col * 16, row * 16)`.

## How far this is proven

The formats are proven by the code that addresses them, by the tile and collision maps agreeing, and by a
worker looking at every picture (plan FINDINGS, 2026-10-31). The renders were not compared with the
emulator's screen in play, so run-time colour changes the copper makes during a level are not covered.
