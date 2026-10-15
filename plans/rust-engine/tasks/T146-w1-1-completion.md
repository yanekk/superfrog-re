# T146 — w1-1-completion

**Phase:** 2 · **Depends on:** T106, T131; blocks T18 · **Weight:** light

Added by T106 (owner, 2026-10-15). T106 played W1-1 to its exit and tally, but every W1-1 exit it
found meets T131's open fault "the exit fade, palette step a frame early". This task carries T106's
done-when items for W1-1 once T131 has fixed that fault.

## Goal

`complete-w1-1` green against the oracle with `resync = false` and no exclusions, in the test
block, with a test in `engine/crates/sf-conform/tests/frog_cases.rs` as `complete_w1_x`'s.

## What T106 found

- `engine/conform/{cases,scripts}/complete-w1-1*` (committed, outside the test block): planned by
  `examples/plan_exit W1-1 --coins 0`, all 90 coins, the exit at frame 4473 (`level_state` 1), fire
  70 frames later in the tally; 4658 frames. `sf-conform scan` finds frames 4475 and 4477 differing:
  one palette colour a step off (`$bbb` for the original's `$ccc`), every variable equal.
- W1-1 lags all the way to its exit, so the loop ends late (line 254) and the fade's set-up
  (`FadeSetup`, `FadeKind::Out`, a masked job) runs past the frame's end: the original from line
  256 to line 2 of the next frame (58 lines), the engine from 254 for about 62.
- Three other exits re-planned from 4330, 4360 and 4400 showed the same 1–2 frames.

## Files

`engine/conform/{cases,scripts}/complete-w1-1*`, `engine/crates/sf-conform/tests/frog_cases.rs`.

## Done when

- [ ] `complete-w1-1` green with `resync = false` and no exclusions, in the test block (T106's
      first done-when item, for W1-1).
- [ ] Its test asserts the frame after the tally is the next level's, the level completed.
