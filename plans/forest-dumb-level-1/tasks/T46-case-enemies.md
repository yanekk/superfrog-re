# T46 — case-enemies

**Phase:** 5 · **Depends on:** T24, T45 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T46` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else. It holds this doc, PROGRESS, FINDINGS and the DESIGN sections, so do not read those again.

## Goal
Runs with level 1's enemies agree with the original: paths, spawning, hits on the character. This is frame-matching work, one group of cases in its own session, because the first game's character builder died in this loop with porting still in its context (owner, 2026-11-05; DESIGN §4.3). A difference is fixed in the engine, never in the comparison; a fix to a routine port carries a rig test that fails without it.
## Design sections this implements
§2.1, §2.2, §4.3.
## Reuses
T06's case runner and coverage report, T32's rig for any fix, `toolkit/docs/porting-routines.md` §3 and §4; T24's creatures, T23's lives.
## Files
`engine/conform/cases/enemy-*.toml`, `engine/conform/scripts/enemy-*.txt`, `fd-conform/tests/enemy_cases.rs`, `fd-core/src/**` (fixes only), `smoke.toml`.
## Interface
Case and script files only, plus engine fixes.
## Tests
- [ ] Each kind's path on the original over a run.
- [ ] Spawn and despawn at the screen edges, and after scrolling back.
- [ ] A hit reduces energy with the original's timing, then T23's path.
- [ ] Kill behaviour if the game has one.
## Done when
- [ ] Exclusions naming T24 are gone; only exclusions naming T25 may remain.
- [ ] A smoke test and `[[area]]`.
- [ ] Every T24 `PORTED` routine shows non-zero hits.
- [ ] Every case states its `goal`; unmodelled effects are exclusions naming their task.
- [ ] If the cases need more than one session, the owner is asked to add a task for the rest, split by case.
