# T44 — case-picture

**Phase:** 2 · **Depends on:** T10 · **Weight:** medium

## Goal
The engine's picture agrees with the original's within tolerance on clear stretches: playfield, character and status bar. This is frame-matching work, one group of cases in its own session, because the first game's character builder died in this loop with porting still in its context (owner, 2026-11-05; DESIGN §4.3). A difference is fixed in the engine, never in the comparison; a fix to a routine port carries a rig test that fails without it.
## Design sections this implements
§2.1, §2.2, §4.3.
## Reuses
T06's case runner and coverage report, T32's rig for any fix, `toolkit/docs/porting-routines.md` §3 and §4; T10's drawing, T04's `picture_in`.
## Files
`engine/conform/cases/picture-*.toml`, `engine/conform/scripts/picture-*.txt`, `fd-conform/tests/picture_cases.rs`, `fd-core/src/**` (fixes only), `smoke.toml`.
## Interface
Case and script files only, plus engine fixes.
## Tests
- [ ] First frame's picture matches the oracle's within tolerance.
- [ ] Across a scroll in all directions.
- [ ] Status bar with T00's values; digits for each value range it shows.
- [ ] Character at every animation frame in a run.
## Done when
- [ ] Picture cases pass, with the differing pixels' report empty apart from named exclusions.
- [ ] A worker opened frames from both sides; recorded as worker-driven.
- [ ] Every T10 `PORTED` routine shows non-zero hits.
- [ ] Every case states its `goal`; unmodelled effects are exclusions naming their task.
- [ ] If the cases need more than one session, the owner is asked to add a task for the rest, split by case.
