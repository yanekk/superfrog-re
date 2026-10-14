# T126 — w5-completions

**Split from:** T39 (owner, 2026-10-14: split W5-2…W5-4 cases and the completions into new tasks). Carries T39's done-when item "a scripted completion of each non-boss level of the world matches the oracle (in `sf-conform -- all`)".

**Phase:** 2 · **Depends on:** T39, T101, T121, T123, T124, T125; blocks T18 · **Weight:** heavy

## Goal

One conformance case per W5 level (`enemies-w5-N-complete`, N = 1..4) whose script plays the level
from its first frame to its exit: every coin the exit needs taken (`frog+$5c` to 0), the exit
(marker 54) reached, `level_state` 1, into the tally (T38's). Green against the oracle with enemies
and items live, each re-sync pinned in its test (none once T101 and T121 have landed). It waits on
the per-level cases for their routes.

## What T39 left for this task

- World 5's twelve types (28–31, 79–85, 140) are ported in `sf-core/src/enemies/w5/` and pass routine
  conformance (`sf-conform/tests/enemies_w5_routines.rs`). W5-1's cases `enemies-w5-1`,
  `enemies-w5-1-hurt` and `enemies-w5-1-activation` are green in `sf-conform/tests/enemies_w5_cases.rs`;
  add this task's test there with `run(name, &[resyncs])` and `met(..)`.
- World 5's markers (`obj_flag_jumptab` row `$400`): 2/3 pacer between them (80/79), 10 faller (28),
  11 drip (31, passive), 12 faller that also falls once in view (30), 13/14 pacer (82/81), 15 scenery
  (32), 16 crusher (140), 17 mace (63, links 64), 18/19 darts (53/54), 20/21 shooter (84/83; its shot
  85), 22 watcher (29), 24 flow (98). Every enemy is immune (`$7d00`): none can be killed.
- `examples/enemies_map -- W5-N` lists where each marker is. The planner (`examples/complete`)
  stalled on W5-1's icy slopes, spike pits and wall jumps; W5-1's scripts were written by hand,
  each try checked with `sf-conform record <case> --only --every --watch fx=h0+7d6:W --watch
  fy=h0+7d8:W --watch hearts=h0+830:W` (`fy`/16 is the map row + 3). Jump height grows with how long
  up is held (about 50 frames for the highest); the frog cannot jump again while hurt (about 100
  frames).
- Re-syncs after a lag are pinned (T121's play field buffer; the panel after a lag, W5-1 979);
  check each also happens with enemies frozen before pinning it. W5-1 frame 595 left out the frog's
  sprites in the engine with every variable equal (excluded, FINDINGS).
- W5-1's `enemies-w5-1` route ends at frame 1000 against the dart's wall (col 196) with both lives.
  Jumping on at 900 (up held 40 frames) clears it, but the frog dies at about 1010 near col 224 and
  the restart after it diverges at 1094 (copper, Paula, the picture): look at that before routing a
  completion through a death.

## Interface

None new: the case `enemies-w5-N-complete` (N = 1..4), its script, and a `#[test]` in
`sf-conform/tests/enemies_w5_cases.rs` using `run(name, &[resyncs])`.

## Tests

- [ ] One test per level in `enemies_w5_cases.rs`: the case green to the tally, `level_state` 1 at
      the exit, re-syncs pinned (or none).

## Files

`engine/conform/{scripts,cases}/enemies-w5-*-complete*`, `engine/crates/sf-conform/tests/enemies_w5_cases.rs`.

## Done when

- [ ] A scripted completion of each level in world 5 matches the oracle (in `sf-conform -- all`).
- [ ] Each is in the test block, re-syncs pinned (or none).
