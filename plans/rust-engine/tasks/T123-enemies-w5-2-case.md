# T123 — enemies-w5-2-case

**Split from:** T39 (owner, 2026-10-14: split W5-2…W5-4 cases and the completions into new tasks). Carries T39's test "frame conformance per level of the world" for W5-2.

**Phase:** 2 · **Depends on:** T39; blocks T18 · **Weight:** light

## Goal

`engine/conform/cases/enemies-w5-2.toml` and its script: W5-2 from its first frame with every
object live, meeting the pacer between markers 2 and 3 (79/80) and the second dart (54), which no W5-1 case meets, and every other W5-2 type, hurt by one of them; green against the original.

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

## Interface

None new: the case `enemies-w5-2`, its script, and a `#[test]` in
`sf-conform/tests/enemies_w5_cases.rs`.

## Tests

- [ ] `enemies_w5_cases::enemies_w5_2`: the case green but for its pinned re-syncs; each type's
      updates met (`met`), the frog hurt by an enemy.

## Files

`engine/conform/{scripts,cases}/enemies-w5-2*`, `engine/crates/sf-conform/tests/enemies_w5_cases.rs`.

## Done when

- [ ] `enemies-w5-2` green (re-syncs pinned, or none), in `enemies_w5_cases.rs`.
- [ ] Every W5-2 type met, or a FINDINGS row naming which case meets it instead.
