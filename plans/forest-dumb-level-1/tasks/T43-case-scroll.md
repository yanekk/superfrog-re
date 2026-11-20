# T43 — case-scroll

**Phase:** 2 · **Depends on:** T08, T09 · **Weight:** medium

## Goal
The camera and edge-redraw flags agree with the original across runs that scroll both ways. This is frame-matching work, one group of cases in its own session, because the first game's character builder died in this loop with porting still in its context (owner, 2026-11-05; DESIGN §4.3). A difference is fixed in the engine, never in the comparison; a fix to a routine port carries a rig test that fails without it.
## Design sections this implements
§2.1, §2.2, §4.3.
## Reuses
T06's case runner and coverage report, T32's rig for any fix, `toolkit/docs/porting-routines.md` §3 and §4; T09's ports.
## Files
`engine/conform/cases/camera-*.toml`, `engine/conform/scripts/camera-*.txt`, `fd-conform/tests/camera_cases.rs`, `fd-core/src/**` (fixes only), `smoke.toml`.
## Interface
Case and script files only, plus engine fixes.
## Tests
- [ ] Scroll limits at all four level ends.
- [ ] Past column 240's filler is never entered if the original does not.
- [ ] Edge flags agree across a long run.
## Done when
- [ ] Camera variables match on a scripted run that scrolls both ways.
- [ ] Foreground timing of redraws is matched or named as an exclusion.
- [ ] Every T09 `PORTED` routine shows non-zero hits.
- [ ] Every case states its `goal`; unmodelled effects are exclusions naming their task.
- [ ] If the cases need more than one session, the owner is asked to add a task for the rest, split by case.
