# T41 — case-run-jump

**Phase:** 2 · **Depends on:** T08, T09 · **Weight:** medium

## Goal
The character's runs agree with the original, variable by variable, on clear stretches of `W1-1`: running and jumping. This is frame-matching work, one group of cases in its own session, because the first game's character builder died in this loop with porting still in its context (owner, 2026-11-05; DESIGN §4.3). A difference is fixed in the engine, never in the comparison; a fix to a routine port carries a rig test that fails without it.
## Design sections this implements
§2.1, §2.2, §4.3.
## Reuses
T06's case runner and coverage report, T32's rig for any fix, `toolkit/docs/porting-routines.md` §3 and §4; T08's ports.
## Files
`engine/conform/cases/character-run-*.toml`, `engine/conform/scripts/character-run-*.txt`, `fd-conform/tests/character_run.rs`, `fd-core/src/**` (fixes only), `smoke.toml`.
## Interface
Case and script files only, plus engine fixes.
## Tests
- [ ] Runs: right to the first gap and jump; standing jump; jump held versus tapped.
- [ ] Input on the frame of a landing.
## Done when
- [ ] Each case passes, comparing the character's `LAYOUT` variables every pass.
- [ ] Each of T08's `PORTED` routines these runs reach shows non-zero hits.
- [ ] Every case states its `goal`; unmodelled effects are exclusions naming their task.
- [ ] If the cases need more than one session, the owner is asked to add a task for the rest, split by case.
