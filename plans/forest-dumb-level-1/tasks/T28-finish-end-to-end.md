# T28 — finish-end-to-end

**Phase:** 6 · **Depends on:** T16, T48 · **Weight:** medium

## Goal
A test plays the planned route in the real app, from launch to the end of the level, the plan's final proof; a short version really runs in smoke.
## Design sections this implements
§1, §4.
## Reuses
`sf-e2e/tests/plan_routes.rs`, `sf-bench/src/verify.rs` (`verify_app`, `app_parts`) as pattern, `amiga-e2e`.
## Files
`fd-e2e/tests/{finish_level.rs,finish_short.rs}`, `games/forest-dumb/smoke.toml`, `games/forest-dumb/CLAUDE.md`, `toolkit/PLAYBOOK.md`.
## Interface
Test files only.
## Tests
- [ ] Route as key presses in the real app: state hashes equal headless every frame; reaches `Complete`; fire restarts.
- [ ] Short test: the route's first stretch, a real run, not an `#[ignore]`.
## End to end (the worker drives this)
- suite: `fd-e2e` · sizes: default
- [ ] Launch to "Level 1 complete", no focus taken.
## Done when
- [ ] The full test passes in `tools/full.sh` and takes a recorded time.
- [ ] The short test and `[[area]]` "level 1 finishes" are in smoke and fit the budget.
- [ ] `tools/smoke.sh` passes under the limit.
