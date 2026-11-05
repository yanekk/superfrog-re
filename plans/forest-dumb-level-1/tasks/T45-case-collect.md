# T45 — case-collect

**Phase:** 5 · **Depends on:** T22 · **Weight:** medium

## Goal
Collecting runs agree with the original: objects enter and leave play where the original's do, and score and status bar follow pickups. This is frame-matching work, one group of cases in its own session, because the first game's character builder died in this loop with porting still in its context (owner, 2026-11-05; DESIGN §4.3). A difference is fixed in the engine, never in the comparison; a fix to a routine port carries a rig test that fails without it.
## Design sections this implements
§2.1, §2.2, §4.3.
## Reuses
T06's case runner and coverage report, T32's rig for any fix, `toolkit/docs/porting-routines.md` §3 and §4; T22's scan and collectibles.
## Files
`engine/conform/cases/collect-*.toml`, `engine/conform/scripts/collect-*.txt`, `fd-conform/tests/collect_cases.rs`, `fd-core/src/**` (fixes only), `smoke.toml`.
## Interface
Case and script files only, plus engine fixes.
## Tests
- [ ] Pick up each kind; score and bar equal the original.
- [ ] A collected item stays gone after scrolling away and back.
- [ ] Milestones of a collecting run met by the engine alone.
## Done when
- [ ] Exclusions naming T22 are gone from every case.
- [ ] A test plays through collecting on engine and original, and an `[[area]]` is added.
- [ ] Every T22 `PORTED` routine shows non-zero hits.
- [ ] Every case states its `goal`; unmodelled effects are exclusions naming their task.
- [ ] If the cases need more than one session, the owner is asked to add a task for the rest, split by case.
