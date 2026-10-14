# T127 — front-vbl-318

**Phase:** 2 · **Depends on:** T100 · **Weight:** small

Added by T100 (owner, 2026-10-14): the one frame of `frog-pit-w1-1` T100's handler timing does
not explain.

## Goal

`frog-pit-w1-1`'s picture compared on frame 318: its `picture` exclusion (task `T127`) gone.

## What is known (T100)

- Frame 318 is `level_start`'s fade in, eleventh step. The copper reads colour 30 (copper word
  h2+$731e) on line `$34` with only red stepped: the original shows `$877`, the engine `$887`
  (green stepped too).
- Everything else about the frame matches: `fade_start` puts the fade on the original's line (the
  music ends on 8.12, `h0+$12a14` waits to line 16), no keyboard poll, no row, no blit (oracle
  blit events none before line 64), no other interrupt, window top 39.
- The original's acknowledge is 58.16; on frame 648 (the same wait line, no keyboard, 80 channels
  changed, the same counts per colour) it is 58.02: about 64 CPU cycles longer, with no difference
  T100 found. Moving the fade 80 cycles later fixes 318 but breaks 323, 645, 648, 651 and 653.
- Ruled out: the per-colour copper read position (MOVE order on line `$34`), a uniform display
  slowdown (the window term in `fade::level_step_times`), the streaming term.
- Frames 613 and 658 are excluded for another reason (the copper reads a channel between the
  fade's `andi` and `add`); they are not this task's.

## Interface

None new: T100's `vbl::fade_start` and `fade::level_step_times` keep their signatures.

## Files

- `engine/crates/sf-core/src/front/{vbl.rs,fade.rs}`, `audio/music.rs` (`tick_work`)
- `engine/conform/cases/frog-pit-w1-1.toml`

## Tests

- [ ] `frog-pit-w1-1` with no `picture` exclusion on 318; the other frames still green.

## Done when

- [ ] The above, in the test block (`frog_cases::frog_pit_w1_1`).
