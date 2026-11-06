# 12 — The level's first view (`level_init`)

What `level_init` (`h0+$1ee`) draws before play, and how the engine does it (plans/forest-dumb-level-1
T65). Ported in `fd-core/src/level/start.rs`; `Game::new(Start::Level1)` runs it.

## What the original does

After `load_world`, `level_init` calls, in order:

| Routine | Address | Instructions (own) | Does |
|---|---|---|---|
| `view_tiles` | `h0+$62e` | 11 | for column `d7` 0 to 22, row `d6` 0 to 15: `view_tile` |
| `view_tile` | `h0+$672` | 26 | tile word at `h0+$16318 + d7.w * $c0 + d6 * 2`, copied by the CPU from `h3+$13540 + tile * $a0` onto the play screen at `h3+$398 + d6.w * $e60 + d7 * 2`: five words a line (`$2e` apart), 16 lines `$e6` apart |
| `view_shapes` | `h0+$6c4` | 11 | the same loop, `view_shape` |
| `view_shape` | `h0+$708` | 31 | shape byte at `h0+$28f18 + d7.w * $60 + d6`, its 16 words from `h0+$bda0 + byte * $20` into `collision_map` at `h3+$fa50 + d6.w * $2e0 + d7 * 2`, `$2e` apart, as `h0_7cc` copies one |

The cells are always the map's top-left 23 by 16, whatever the camera: the level starts with the view
there and the edge redraws (T55, docs/09) bring in the rest as it scrolls. Map row `r` lands in band
`r`, which is band `r & 15` of the edge redraws' pointer tables when the view is at y 0.

## Interruptible points

Listed from the listing on 2026-11-05, before porting: none that matters. The four have no raster or
blitter wait and run with interrupts on, so an interrupt can come between any two instructions. Only
the level-3 handler could read what they write (`game_frame` probes `collision_map`), and it does
not run during them: `irq_count`, which `level3_handler` adds 1 to on every entry and nothing clears,
reads 1 on play's first frame (docs/09, T03's test), so the first level-3 interrupt since the program
started is play's first. The keyboard handler touches neither the screen nor `collision_map`.

## The picture

The CPU's copy of a tile writes exactly the bytes the blit `h0_774` writes for the same tile and
destination (A to D, `BLTSIZE=$1401`, D modulo `$2c`). So the engine lists each as a `Blit::Tile`, and
`Game::new` leaves the 368 of them, column by column, in `Game::blits()` until the first step; the
picture (T10) applies them as it applies an edge redraw's. `level_start.rs` holds them against the
original's play screen at the sync point.

## Measured at the sync point

Measured 2026-11-06 by `fd-conform/tests/level_start.rs` (smoke), against the original at the level
sync point (frame 6301 of `boot-to-level1`):

- `collision_map`, `$2e00` bytes, equals the engine's first view byte for byte.
- The kept slack around it (`$40` bytes before, the rest up to the tiles at `h3+$13540` after) is not
  zero: it is `_BALLA`, the title's music, which `load_list_title` reads unpacked to `h3+$f000`
  (`song`) and which runs across `collision_map` and on. Nothing writes it between the title and
  `level_init`, which overwrites only the map. So `GameData::read` reads `_BALLA` too and
  `Game::new` lays it under the plane before drawing (`level::start::lay_title_song`); without it
  3 281 kept bytes differ. The edge redraws overwrite the slack as the view moves right; whether play
  ever reads it before then was not measured.
- The play screen's 16 bands equal the 368 listed tiles copied from the original's own tiles.
- `creature_active` (`h0+$67da`) and `anim_cell_used` (`h2+$1680`) are all 0: `level_init` starts no
  creature and adds no animated cell, and no edge redraw has run before the first pass. The engine's
  empty tables match, so nothing goes to the owner.

## Proven on the rig

`fd-conform/tests/level_start_routines.rs`, generators in `fd-conform/src/gen/level_start.rs`: the two
cell routines over 10 000 states each, the two loops over 1 000 (368 000 cells each; 10 000 passed
once too, in 32 minutes, 2026-11-06). The cell routines add their offsets to the bases the loops pass
in `a3` to `a5`, so their states set those. Tile numbers from `$90e` read past the oracle's 512 KB of
chip RAM, which there reads back from 0 (a later cell can read the screen an earlier one drew); the
rig's flat memory does not model that, so the generators stay below it. The level's tiles stop at `$3ed`.
