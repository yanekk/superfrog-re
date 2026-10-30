# T7MP level format

Moved from Claude's memory 2026-10-30; checked against games/superfrog/docs/04, 06, 14 and the render tool.
Corrected: World 6 colours (fix/world6-palette is merged: each level's own PALA list), and the "next"
line, since a level edit now plays as a `.sflevel` with no repack.

Decompressed Superfrog levels are **`T7MP`** ("Team 17 MaP") files: flat sequence of
`[4-byte ASCII tag][u32 BE length][payload]`. All 29 shipped levels are 53588 bytes,
`300 × 84` blocks. Parse by walking tags (`games/superfrog/tools/t7mp.py`), not fixed offsets.

Key tags: `XBLK`/`YBLK` = map w/h in blocks; `PALA..PALD` (192 B each) palettes;
`IFFP` (64 B) tileset ref e.g. "SDIFF:L6BM-IFF"; `IFFC` (2048 B) tile attr/collision;
`CCCL` colour-cycle list; `REMX`/`REMY` start coords; `VERS`/`COM1`/`COM2` text.

**`BODY` = XBLK*YBLK*2 bytes = the tilemap**: row-major grid of 16-bit BE cells.
Split (confirmed): tile index = cell & 0x3FF (0..839, matches 840-tile set); flags =
cell >> 10 (6-bit collision/behaviour, exact bits TBD via IFFC). This is the core data
a map editor edits.

Graphics/rendering (`games/superfrog/tools/render_levels.py` renders all 29 → games/superfrog/build/levels/*.png):
- Tileset = 134400-byte chunk = 840 tiles of 16x16, 5bpp, per-tile plane-major
  (160 B/tile). One per world; level→tileset matched by IFFP world id (L1..L6, LB
  bonus, LW world-map), NOT disk order (disk4 L6 levels precede their tileset d4_c07).
- Palette = 32-colour OCS (12-bit 0x0RGB, nibble*17), colour 0 black. It lives in
  each world's SPRITE BANK (the ~71-72KB object chunk just before that world's
  tileset) at offset 0x11300 (world 6's bank = d4_c01 @0xC7FE). NOT in the P41A
  chunks (those are unrelated title/menu gfx; using them was a bug that gave wrong
  colours). Each level also carries its own 32 colours in PALA, which the render tool uses
  (24/24 against the engine on World 6).
- Verified vs a real screenshot: World 2 = grey stone castle, World 1 = blue-sky grassland.

Next: map editor (view/edit, saved as `.sflevel`, [14-sflevel-format.md](14-sflevel-format.md)). Writing back to an `.adf` needs an ATN!
*compressor* (only the decompressor exists, see [atn-decompression.md](atn-decompression.md)). Also
refine flags-bit meanings.
