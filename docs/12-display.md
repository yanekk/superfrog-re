# 12 — The level display

How the original puts a level on screen, and how `engine/crates/sf-core/src/render` draws the same
pixels from the engine's state. Addresses are `hN+$off` in `d1_c03` (docs/08: `h0` = `$C00FA0`,
`h1` = `$C492EC`, `h2` = chip `$400`, `h3` = chip `$77FC`). Claims marked **(oracle)** are checked
by `engine/crates/sf-conform/tests/render_vs_oracle.rs` and the `render-*` cases; **(code)** comes
from reading the listing; **(inferred)** neither.

## What the renderer draws from

The picture is a function of the state alone. The state keeps the display's data as the original
keeps it (owner, 2026-10-11), and the renderer reads it as Denise and Agnus would:

| Variable | Where | What |
|---|---|---|
| `level_copper` | `h2+$6bd0`, `$400` bytes | the level's copper list |
| `panel_bitmap` | `h3+$4b390`, 5 × `$2a0` | the status panel's bitplanes |
| `playfield_buffer_0`, `_1` | `h3+$10600`, `h3+$22f30`, `$12930` each | the two display buffers |
| `scroll_column`, `camera_x`, `camera_y` | `h0+$172b0`, `h0+$16664`, `h0+$16660` | where the play field is scrolled |
| `map_rows`, `level_map`, `world` | `h0+$17334`, `h1+$218e0`, `h0+$40da` | the map and its world (widescreen only) |

What the disks decide is built once by `DisplayData`: hunk 2's contents (`d1_c02`'s data block: the
HUD's sprite digits), the frog's sprite frames (below) and each world's tiles.

**The display buffers are state, not drawn from the map.** The original's buffers drift from the
map: objects write tiles into them that the map does not hold, tile animations are blitted into both
(h0+$fa7c: the clean buffer by the CPU, then five planes into each display buffer), and a bob over a
changed tile is gone until that buffer is drawn again. On W2–W5 drawing from the map was wrong on
nearly every frame; the buffers explain every pixel **(oracle)**. So T14–T18 write the buffers as the
original does (its tile, animation and bob blits), and save states grow by about 150 KB.

## The copper list

The level list (COP1LC = `h2+$6bd0`, set at `h0+$69c4`) in order **(oracle)**:

- top of frame: BPLCON2 `$22`, DIWSTRT `$2779`, DIWSTOP `$2cc7` (window 334 × 261 at `$79`,`$27`),
  DDFSTRT `$28`, DDFSTOP `$d8` (23 words a line), modulos 2, all sprites cleared;
- line `$20`: sprites 0–3 (the frog: two attached pairs, side by side);
- line `$24`: BPLCON0 `$5200`, BPLCON1 `$88`, the panel's pointers, the panel's palette A;
- line `$28`: palette B (the panel's middle); line `$34`: palette A again;
- `$34`,`$e0`: bitplanes off (two black lines), the play field's pointers and scroll, its palette;
- line `$36`: bitplanes on; the ring seam's wait and pointer reload (h0+$1700a writes it);
- `$ff`,`$7e`: sprites 0–3 again, for the coin counter's two digits (bottom right);
- `$ff`,`$de`: a second pointer reload, used when the seam is below line `$100`. When it is above,
  the copper reaches this wait after cycle `$E0` of line `$ff` and it never fires (the comparator
  wraps there; vAmiga `Copper::runHorizontalComparator`).

## Copper timing

A palette set takes 32 moves; written at line `$28` and `$34`, the later colours change in the middle
of the visible line **(oracle)**. `render/copper.rs` runs the list cycle by cycle on the reference
emulator's bus model: the copper uses even DMA cycles, two per instruction word, never cycle `$E0`;
a lores fetch unit of eight cycles from DDFSTRT takes planes `-,4,6,2,-,3,5,1`, so with five planes
the copper loses one even cycle in four inside the fetch window. A WAIT wakes up per vAmiga's
`findMatch` (the horizontal counter compared two cycles ahead). A colour written at cycle `h` shows
from lores position `2h`; the picture carries those as `Split`s. Bitplane DMA runs only inside the
vertical window, so the panel's pointers advance from line `$27`.

## Fetched lines and the panel

Window pixel `x` shows fetch index `left + x − (2·DDFSTRT + 17) − delay` of the line's data
**(oracle)**: for the level, `x + 24 − delay`. The panel is five planes of 14 lines `$2a0` apart
fetched like the play field; its last two lines read past the fifth plane into the frog's sprite
frames, as the original's do.

## Scroll

`h0+$1700a` (in the vertical blank) latches the camera: `d0 = camera_x + 7`, `d1 = camera_y + $17`;
BPLCON1 = `~d0 & 15` for both playfields; the pointer is the buffer, plus the ring row of `d1 >> 4`
(19 rows of `$300` bytes, table `h0+$174d6`), plus `(d1 & 15) × $30`, plus `2 × (d0 >> 4)`; that last
column also goes to `scroll_column`. So a buffer line at ring line `R` shows level row `R − 16`
(mod 304), and fetch pixel `i` at column `c` shows level x `16c + i − 16`. Columns scroll in by moving
every pointer two bytes; the ring's slack (`$3b70` per plane = 317 lines) holds the creep.

The renderer uses this only for widescreen: picture columns beyond the original window continue the
line's level row from the map; past the map's edges the level is colour 0 **(oracle: left edge)**.

## Bobs

`build_bob_draw_list` (h0+$19b42) and `h0+$19120` erase each bob from the clean buffer (`h3+$35860`)
and cookie-cut it (minterm `$CA`, last-word mask 0) into the back buffer, in two parts across the
ring seam; each bob record (`bob_records`, `$11c` bytes, 4 sub-records of `$46`) keeps per buffer the
destination, size, source, modulos and the frame pointer. Two details for whoever ports it
**(oracle)**: the blitter ignores bit 0 of a destination (a bob near the top records an odd one), and
the upper sub-record's x is already the next pass's position, so the shift a buffer was drawn with
is in its seam sub-record's copy (`+$8c`). The masks are the OR of the sheet's planes, at
`h3+$48190`. Bob frames (`$30` bytes) are built per world at h0+$2b1b4 and h0+$2d52c (h0+$18e8a).

## Sprites

`render/objects.rs` follows Agnus's sprite DMA (vAmiga `executeFirstSpriteCycle`, `updateSpriteDMA`):
every sprite fetches its control words on line 25, data between VSTART and VSTOP, the next control
words at VSTOP; a copper control write disarms. A sprite shows at window position `HSTART + 1 − left`,
and on OCS only on lines with a bitplane fetch (vAmiga `spriteClipBegin`), so the frog vanishes on the
two black lines under the panel **(oracle)**. In single-playfield mode the play field's priority is
BPLCON2's PF2P (4 here: all sprites in front).

The frog's frames are built by `load_hero` (h0+$19ce0) into `h3+$4c0b0`: for each rectangle of the
table at h0+$1a6f4, from `HERO` and from `HERO` with each 32-pixel group mirrored, two 16-pixel
columns of an attached pair (planes 0/1, then 2/3) **(oracle: byte-identical)**. The counter's digits
point into hunk 2 (`h0+$862e`).

## Palettes

The level palette changes only in the fade-in at a level's start (h0+$153a4), the exit fade
(h0+$155a0) and the boss's flash (h0+$151b2), all outside `level_frame_loop` **(code)**. There is no
colour cycling in a level (W1-1, 8 500 frames watched).

## Reaching every level

`sync::reach` gets the oracle into W1-1 by START GAME and into every other level of W1–W6 and the
bonus level by the options screen: OPTIONS, three times down (the code entry starts as the cursor
arrives), six digits as raw keys `01`–`0A` read from the original's own `password_table`, then fire
three times. Whenever `disk_request`'s wanted ID (h0+$af54) changes, that disk goes in. W6's second
map set needs a second drive and the final level has no code; neither is reached.

## Late redraws

A transplant takes memory at the end of a frame. When the original rewrites the copper list, the
panel or the shown buffer after the beam has passed (a busy vertical blank's sprite words, the clock
digit, a tile animation, a vertically scrolled-in row), that memory is newer than the picture. The
`render` compare leaves such a frame out, listed with the reason, when every differing pixel is one
the frame's change reaches; about 4 % of level frames. Stepping conformance (T14 on) must still
match them.
