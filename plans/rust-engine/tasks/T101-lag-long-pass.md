# T101 — lag-long-pass

**Phase:** 2 · **Depends on:** T81, T82; blocks T21 · **Weight:** medium

Added by T83 (owner, 2026-10-14). T83's bounce-pad case `items-pad-w1-1` (committed, not in the
test block) is green to frame 401 and then shows two faults in how the engine follows the
original's lag (`Pacing::Late`, `level/frame.rs`, `level/beam.rs`). Neither is in the pickups.

## Goal

1. **A cut inside phase 6, before the flip (frame 402).** Frame 401's pacing is `Late { writes: 48,
   line: 311, hpos: 176, phase: 6, step: NO_STEP }`: the original's frame ended in phase 6 (after
   `Cost::AfterDraw`) before `buffer_flip` wrote its pointers. The engine cuts after the flip, so
   frame 402's vertical blank writes the other buffer's pointers: `level_copper` (+$2ac..$3f8 high
   words $1 vs $2) and `scroll_column` ($17e9c vs $2a7cc). Nothing else differs.
2. **A pass across three frames (frame 786).** In the pad bounce (frames 741–785, every frame a
   lag) a pass starts in 765 (cut phase 1 step 1), is cut at phase 5 in 766 and at phase 8 in 767,
   where the original's `vbl_count` is 2. The engine's is 0 at 767: it finished that pass and began
   the next. From there it is a pass behind; at 786 `vbl_count`, `obj_count`, the bobs, score and
   the playfield differ.
3. Make `items-pad-w1-1` green with `resync = false` and add it to
   `sf-conform/tests/frog_cases.rs`.

## Tools

`items-pad-w1-1.toml` with `resync = true` lists both re-syncs (402, 786) and is green otherwise.
A per-frame print of the original's and the engine's `vbl_count` (h0+$9d8) next to the given
`pacing` in `sf_conform::run` shows where the two part (T83 did this by hand, not committed).

## Files

`engine/crates/sf-core/src/level/{frame.rs,beam.rs}`; `engine/crates/sf-conform/tests/frog_cases.rs`.

## Done when

- [ ] `items-pad-w1-1` green against the oracle with no re-sync, in the test block.
- [ ] The existing item cases (`items-run-w1-1`, `items-fruit-w2-2`, `items-wings-w1-2`,
      `items-checkpoint-w1-1`) still green.
