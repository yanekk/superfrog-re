# T103 — w4-completions

**Phase:** 2 · **Depends on:** T41, T101, T102; blocks T18 · **Weight:** heavy

Added by T41 (owner, 2026-10-14). T41 ported every W4 object and proved each level's enemies in
its own frame case (`enemies-w4-*`), but not a level played through: no task has scripted a level
completion yet (T38's tally cases force the exit). This task carries T41's "Done when" item "a
scripted completion of each non-boss level of the world matches the oracle". All four W4 levels
are non-boss (docs/09: no per-world boss).

It also carries the types T41's cases did not reach (owner, 2026-10-14), all deep in the W4-3 and
W4-4 mazes: the pacers (markers 2/3, templates 136/137, W4-4 only), the crusher striking up (16,
142), the mace (17, 69/70), the darts of markers 20 and 21 (61, 62, W4-4 only), the floater of
marker 22 (143) and the crawlers (13/14, 139/138; T41 review, owner); and W4-3, which has no T41
case. Each must be met, and hurt the frog or be killed
where it can, in one of this task's playthroughs.

## Goal

A scripted playthrough of each of W4-1, W4-2, W4-3 and W4-4, from the level's start to its exit:
coins collected until `coins_left_for_exit` (h0+$832) is 0 and the exit lock (marker 29) is gone,
the exit (marker 54) reached and the level's end (`level_state` 1, the exit fade, T38's tally)
played, matching the original on every frame with no exclusions and `resync = false`.

## Design sections this implements

DESIGN §2.1, §2.7; docs/09.

## Interface

None new: cases and scripts in `sf-conform`'s existing formats (`sync`, `script`, `frames`,
`resync`), as T41's `enemies-w4-*` cases are.

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
- T41's `enemies-w4-*` cases are the start of each level's route. `examples/route` (T41) is
  `plan` with the objects live, scored by steps through open map cells; a target is the cell the
  frog's feet should be in. It could not climb W4-4's crumbling ledges at 173,59 toward the mace at
  191,55 (it stays at x 2807): ledge climbs need better jump search.

## Tests

- [ ] `complete-w4-1` … `complete-w4-4` green in `sf-conform -- all`, each with `resync = false`
      and no `[[exclude]]`.
- [ ] Between them, every type above met (list the slots each frame spawns and frees, as
      T41 did, in the commit message), and each hurting the frog or killed where it can.

## Done when

- [ ] Each W4 level's completion matches the oracle to the frame its loop ends, in `sf-conform --
      all` and in the test block.
- [ ] A fault outside W4's objects found on the way is a new task or a FINDINGS row, not a patch
      here.
