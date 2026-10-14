# T106 — w1-completions

**Phase:** 2 · **Depends on:** T42, T86, T101; blocks T18 · **Weight:** heavy

Added by T42 (owner, 2026-10-14: split the completions out of T42). T42 ported world 1's enemies
and gave each W1 level an enemy frame case; this task carries T42's done-when item "a scripted
completion of each non-boss level in world 1 matches the oracle (in `sf-conform -- all`)".

## Goal

One conformance case per W1 level (`enemies-w1-N-complete`, N = 1..4) whose script plays the level
from its first frame to its exit: every coin the exit needs taken (`frog+$5c` to 0; W1-1 needs 90),
the exit (marker 54) reached, `level_state` 1, the loop ended into the tally (T38's). Green
against the oracle with enemies and items live, each re-sync pinned in its test as
`frog_cases::items_run_w1_1` pins T101's (none once T101 has landed).

## What T42 found

- `sf-conform/examples/complete.rs` plans a completion in legs: `--coins 8 --paced --prefix
  <script>` starts each leg from the engine as the original paces it through the prefix (a plan
  made unpaced leaves the original's route: its passes lag, so it takes fewer coins and the frog
  died where the plan's did not). A leg ends with a coin taken and the frog on the ground. Driven
  leg by leg, W1-1 reached 58 of 90 coins in about two hours of compute, then found no coin within
  its horizon from frame 3240 (frog at x `$58f`, y `$2b1`): the remaining coins need a longer
  horizon, waypoints, or a route order (an unpaced run reached all 90 by frame 2890).
- That route diverged at frame 2256 in a spring launcher (marker 7): the frog's sprite, every
  variable equal. Launchers are T86's; route around them until it lands.
- W1-1 lags often with its enemies (passes across up to four frames: T101).

## Files

`engine/conform/{scripts,cases}/enemies-w1-*-complete*`, `engine/crates/sf-conform/tests/frog_cases.rs`
(or a new test file), `engine/crates/sf-conform/examples/complete.rs`.

## Done when

- [ ] A scripted completion of each non-boss level in world 1 matches the oracle (in `sf-conform -- all`).
- [ ] Each is in the test block, re-syncs pinned (or none).
