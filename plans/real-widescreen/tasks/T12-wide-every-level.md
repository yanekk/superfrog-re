# T12 — wide-every-level

**Phase:** 3 · **Depends on:** T11 · **Weight:** heavy

## Goal

Prove the widescreen game can be finished: the smart planner plays every level (W1-1 … W6-4), the bonus
level and the final level to their ends in widescreen, and each route is kept as a test that replays it
on the engine to the exit.

## Design sections this implements

DESIGN §1 success criteria, §5 (smart planner), D5.

## Precondition

`sf-plan` is on the base branch: `git merge-base --is-ancestor pir/smart-planner main` succeeds, or
`engine/crates/sf-plan` exists on main. If not, ask the owner and wait (DESIGN D5); do nothing else
first. Merge the base branch into the feature branch only through pir's own procedure.

## Files

- `engine/crates/sf-plan/src/main.rs` (a `--widescreen` flag: the engine plays with
  `set_widescreen(true)`; implies `--engine-only`, since there is no original to confirm against)
- `engine/conform/scripts/wide-*.txt` (one route per level)
- `engine/crates/sf-conform/tests/widescreen_complete.rs` (new; engine only)

## Interface

```
sf-plan <level> --widescreen [--out engine/conform/scripts/wide-<level>.txt] [sf-plan's other flags]
```

`widescreen_complete.rs`: for each `wide-*.txt`, start the level as `sf-plan` did, `set_widescreen(true)`,
replay, and assert the level is finished (`level_state` set, or the bonus/final level's own end) within
the script's frames. Runs headless; mark the whole-set test `#[ignore]` if it exceeds the block's budget
and keep one quick level un-ignored.

## Tests

- [ ] one route per level, the bonus level and the final level, each replaying to its end.
- [ ] `--widescreen` refuses `--follow`/oracle confirmation with a clear message.

## Done when

- [ ] All 26 routes committed and `widescreen_complete.rs` green (ignored ones run once, recorded).
- [ ] A level the planner cannot finish within its budget is a FINDINGS row with the reason and the
  owner told; a level made impossible by widescreen is a defect, fixed with a test or raised.
