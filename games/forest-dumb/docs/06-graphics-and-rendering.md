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
- `WYBIJACZKA` is loaded twice, to hunk 4 `$9618` and `$9898`, and `$23dc` picks a copy by a flag. World 1's
  load list is the exception: it loads `$9618` twice and never `$9898`, which keeps the main program's own
  bytes, not a picture (measured 2026-11-05, `level_files.rs`). Whether world 1 ever picks that copy is not traced.
- Which world loads which set of `_ZNIKAJKA`, `_WYBIJACZKA` and `_Plomien`: 1 in W1 and W2, 2 in W3 and W4,
  3 in W5 and W6. These pictures carry their world's background, so they have no mask file.

## The palette

Level colours are 32 pairs of `COLORnn` register and 12-bit value in hunk 1's copper list, file offset
`$5e564 + $404`. Levels start by copying `$384` (W1, W2, hunk 1 `$484`) or `$404` (W3 to W6, `$49e`) over
`$304`. The two blocks hold the same values, so all six worlds share one palette. The tool reads it from the
main program and checks each register number. Colours are not written down here: they are disk-derived.
Measured at the W1-1 sync point (T10, `drawing_routines.rs`): the target block `h1+$304` holds hunk 1's
`h1+$384` block, the second of the two above; the file's own `h1+$304` differs. The engine copies it so.

## What the renderer writes

Into `build/renders/`, git-ignored, 41 pictures: per level `W<n>-1.png` (the map), `-collision.png` (white is
solid), `-objects.png` (the map with object cells boxed by group, chapter 05), `-tiles.png`, `-shapes.png`;
and `sprites-<sheet>.png`. One map pixel block is one cell, so `<level-id>:<col>,<row>` is at pixel
`(col * 16, row * 16)`.

## How far this is proven

The Rust readers in `engine/crates/fd-assets` (`read_level`, `read_sprites`) read the same files, and
`engine/crates/fd-conform/tests/level_files.rs` checks them against the original on the oracle (plan
`forest-dumb-level-1` T05, 2026-11-05):

- **Bytes.** At the level sync point (frame 6301) all five W1-1 files and the seven sprite files of world 1
  equal, byte for byte, what the game holds at the targets in [07](07-executable-map.md)'s memory layout,
  the object map included: nothing has written to it yet. World 1's second `_WYBIJACZKA` copy is the one
  difference (above).
- **Picture.** In play, every whole cell in the 272 x 179 view, 160 of them, shows the tile the map names,
  pixel for pixel through the level palette at hunk 1 `$304`, except cells under a creature or item (0, then 11):
  at camera (0, 51) and again scrolled to (522, 464). The view's top-left pixel is level pixel
  (`camera_x + 17`, `camera_y + 1`). That proves the column order, the tile bit order and the palette.
- **Column order.** Between neighbouring different tiles, edges agree in at least half their pixels
  0.46 of the time read by columns and 0.32 read as 96 across; the test holds the gap.
- **Not proven.** The first 31 frames of play fade the palette in, so the first play frame does not show
  the level. Only W1-1 and world 1's sprites are compared with the original; the other worlds rest on the
  formats above. Sprite frames are compared as bytes, not as drawn: the character is a hardware sprite and the
  bobs' drawing is T10's. Collision shapes are compared as bytes; their meaning in play is T08's.

## The engine's picture (T10)

`fd-core/src/render/` draws the frame as Denise would: the level copper list (`h1+$d4`, from the main
program) is run for the frame over the engine's copy of the memory it reads, each line fetched from where
its bitplane pointers point, the character's sprites over it, outside each line's DIW colour 0. Window:
DIWSTRT `$2f81`, DIWSTOP `$29c1` (play area and status bar). `amiga-chips` gives the copper run, the fetch
and, moved there by T10 from the first game's renderer, the sprite composition (`amiga_chips::sprite`).

**The display buffers are picture state, kept, not drawn from the map** (T00's answer, docs/09). The play
screen (`h3+$398`, 256 lines, and its slack to `collision_map`) is what the original's blits leave in it:
the engine applies T55's `Blit` list (tile copies, bobs cut through their masks) as it goes. A blit word
landing outside it is dropped: the stray bob after a full creature table lands in low chip RAM. The status
bar's buffer (`h4+$6400`) is the program's picture with the numbers `h0_6a96` draws. `Game`'s equality
leaves both out. A new game's play screen holds `level_init`'s first view (T65, docs/12), its 16 bands only.

What the start-up and level start leave in hunk 1, which the engine sets up the same way: the sprite
pointers (`h0_9a34`), the status bar's bitplane pointers (`h0_988a`), the palette block above.

Interruptible points of the ported routines (porting rule 1), listed before porting:

| Routine | Runs in | Interruptible points |
|---|---|---|
| `char_sprite_copy`, `h0_94ba`, `h0_94fe`, `h0_9550`, `char_sprite_position`, `h0_9754` | level-3 interrupt (`game_frame`) | none; no waits |
| `h0_3ab4` | callers in `game_frame` | a blitter-busy loop; the engine's blits finish at once |
| `h0_6a96` and its digit routines | foreground (`main_loop`) | any instruction: a pass can cut a redraw short; picture only, for one frame |
| `h0_9a34`, `h0_988a` | start-up | none |

Measured (2026-11-06): with the original's display memory transplanted, the engine's picture equals the
oracle's `picture_in` at frame 6340 in every pixel; at 6640, scrolled and moving, 1 211 pixels differ around
the character and a creature, which the original rewrites while the frame is drawn (T44's). The oracle's
picture at the sync point (6301) is its blank, not play's.
