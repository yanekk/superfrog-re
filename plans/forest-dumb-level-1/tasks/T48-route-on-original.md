# T48 — route-on-original

**Phase:** 6 · **Depends on:** T27 · **Weight:** medium

## Goal
T27's route agrees with the original from start to the level-end milestone, with no exclusion but sound and every ported routine shown to run. This is frame-matching work, one group of cases in its own session, because the first game's character builder died in this loop with porting still in its context (owner, 2026-11-05; DESIGN §4.3). A difference is fixed in the engine, never in the comparison; a fix to a routine port carries a rig test that fails without it.
## Design sections this implements
§2.1, §2.2, §4.3.
## Reuses
T06's case runner and coverage report, T32's rig for any fix, `toolkit/docs/porting-routines.md` §3 and §4; T27's route, T15's compare if a recording is needed.
## Files
`engine/conform/cases/route-W1-1.toml`, `fd-conform/tests/full_route.rs`, `fd-core/src/**` (fixes only), `smoke.toml`.
## Interface
Case and script files only, plus engine fixes.
## Tests
- [ ] The route on the original passes the case and reaches the milestone.
- [ ] No exclusions but sound in that case.
- [ ] No `Event::Unported` over the whole route; every value in `re/objects-W1-1.toml` has a registered `ObjectKind`.
- [ ] Every `PORTED` routine shows non-zero hits in some case; a routine with none is named and raised with the owner.
## Done when
- [ ] The case passes in full and is in the full set; a short part of it is smoke if it fits (`smoke-times.tsv`).
- [ ] Every case states its `goal`; unmodelled effects are exclusions naming their task.
- [ ] If the cases need more than one session, the owner is asked to add a task for the rest, split by case.
