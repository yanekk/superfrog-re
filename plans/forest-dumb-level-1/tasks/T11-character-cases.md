# T11 — character-cases

**Phase:** 2 · **Depends on:** T01, T08, T09, T10 · **Weight:** medium

## Goal
Stage proof: the same inputs on both sides, movement and picture match on stretches of `W1-1` with no enemies or collectibles; add the first level smoke test and area.
## Design sections this implements
§2.2, §4.
## Reuses
T06's runner.
## Files
`engine/conform/cases/stretch-*.toml`, `engine/conform/scripts/stretch-*.txt`, `fd-conform/tests/stretches.rs`, `smoke.toml`.
## Interface
Case files only. If T00 and T21-style measurement allow it, a frozen case per `sf-conform/src/freeze.rs` also runs.
## Tests
- [ ] At least five stretches covering run, jump, slope, pit edge and scroll.
- [ ] Each compared by variables and picture.
- [ ] A mutated jump constant fails a stretch.
- [ ] Every stretch has a `goal` (a cell reached by a frame, as the character's two position variables), and each fails when the goal is moved one cell.
- [ ] Every `PORTED` routine of T08, T09 and T10 shows non-zero hits on the original in at least one stretch.
## Done when
- [ ] Stretches pass; one under 60 s is smoke with an `[[area]]`, "level 1's character against the original".
- [ ] Exclusions name their task.
- [ ] Smoke timing checked against `smoke-times.tsv`.
