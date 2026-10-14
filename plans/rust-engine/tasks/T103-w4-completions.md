# T103 — w4-completions

**Phase:** 2 · **Depends on:** T41, T101, T102; blocks T18 · **Weight:** heavy

Added by T41 (owner, 2026-10-14). T41 ported every W4 object and proved each level's enemies in
its own frame case (`enemies-w4-*`), but not a level played through: no task has scripted a level
completion yet (T38's tally cases force the exit). This task carries T41's "Done when" item "a
scripted completion of each non-boss level of the world matches the oracle". All four W4 levels
are non-boss (docs/09: no per-world boss).

## Goal

A scripted playthrough of each of W4-1, W4-2, W4-3 and W4-4, from the level's start to its exit:
coins collected until `coins_left_for_exit` (h0+$832) is 0 and the exit lock (marker 29) is gone,
the exit (marker 54) reached and the level's end (`level_state` 1, the exit fade, T38's tally)
played, matching the original on every frame with no exclusions and `resync = false`.

## Design sections this implements

DESIGN §2.1, §2.7; docs/09.

## Files

- `engine/conform/scripts/complete-w4-{1,2,3,4}.txt`, `engine/conform/cases/complete-w4-{1,2,3,4}.toml`
- `engine/crates/sf-conform/tests/frog_cases.rs`: one test per green case.
- `engine/crates/sf-conform/examples/`: a route planner if `plan` cannot do it (see Tools).

## Tools

- `cargo run --example enemies_map -- W4-1` (T41) lists each level's object markers, start
  (`$35`), exit (54) and coins (8, 9) with `col,row`.
- `examples/plan` beam-searches the engine toward map cells, but with the enemies frozen and the
  engine unpaced (FINDINGS, T84 review): check a planned route against the original with
  `sf-conform record`, and plan with the enemies live here.
- T41's `enemies-w4-*` cases are the start of each level's route.

## Tests

- [ ] `complete-w4-1` … `complete-w4-4` green in `sf-conform -- all`, each with `resync = false`
      and no `[[exclude]]`.

## Done when

- [ ] Each W4 level's completion matches the oracle to the frame its loop ends, in `sf-conform --
      all` and in the test block.
- [ ] A fault outside W4's objects found on the way is a new task or a FINDINGS row, not a patch
      here.
