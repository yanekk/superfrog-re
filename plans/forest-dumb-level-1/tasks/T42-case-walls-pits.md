# T42 — case-walls-pits

**Phase:** 2 · **Depends on:** T08, T09, T41 · **Weight:** medium

## Goal
The character agrees with the original against walls, pits, slopes and the level edges. This is frame-matching work, one group of cases in its own session, because the first game's character builder died in this loop with porting still in its context (owner, 2026-11-05; DESIGN §4.3). A difference is fixed in the engine, never in the comparison; a fix to a routine port carries a rig test that fails without it.
## Design sections this implements
§2.1, §2.2, §4.3.
## Reuses
T06's case runner and coverage report, T32's rig for any fix, `toolkit/docs/porting-routines.md` §3 and §4; T08's ports.
## Files
`engine/conform/cases/character-edge-*.toml`, `engine/conform/scripts/character-edge-*.txt`, `fd-conform/tests/character_edges.rs`, `fd-core/src/**` (fixes only), `smoke.toml`.
## Interface
Case and script files only, plus engine fixes.
## Tests
- [ ] Runs: wall; pit; slope.
- [ ] Left and top level edges.
## Done when
- [ ] Each case passes, comparing the character's `LAYOUT` variables every pass.
- [ ] Every T08 `PORTED` routine shows non-zero hits in T41's or this task's cases; one with none is named.
- [ ] Every case states its `goal`; unmodelled effects are exclusions naming their task.
- [ ] If the cases need more than one session, the owner is asked to add a task for the rest, split by case.
