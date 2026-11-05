# T47 — case-mechanics

**Phase:** 6 · **Depends on:** T25, T45 · **Weight:** medium

## Goal
Runs through each of level 1's mechanics agree with the original. This is frame-matching work, one group of cases in its own session, because the first game's character builder died in this loop with porting still in its context (owner, 2026-11-05; DESIGN §4.3). A difference is fixed in the engine, never in the comparison; a fix to a routine port carries a rig test that fails without it.
## Design sections this implements
§2.1, §2.2, §4.3.
## Reuses
T06's case runner and coverage report, T32's rig for any fix, `toolkit/docs/porting-routines.md` §3 and §4; T25's mechanics.
## Files
`engine/conform/cases/mechanic-*.toml`, `engine/conform/scripts/mechanic-*.txt`, `fd-conform/tests/mechanic_cases.rs`, `fd-core/src/**` (fixes only), `smoke.toml`.
## Interface
Case and script files only, plus engine fixes.
## Tests
- [ ] A scripted run through each mechanic's spot on both sides.
- [ ] Its timing cycle, and its state after scrolling away.
## Done when
- [ ] Exclusions naming T25 are gone; others name T24.
- [ ] A test uses each mechanic; `[[area]]` added.
- [ ] Every T25 `PORTED` routine shows non-zero hits.
- [ ] Every case states its `goal`; unmodelled effects are exclusions naming their task.
- [ ] If the cases need more than one session, the owner is asked to add a task for the rest, split by case.
