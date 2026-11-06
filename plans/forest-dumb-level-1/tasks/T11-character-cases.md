# T11 — character-cases

**Phase:** 2 · **Depends on:** T01, T41, T42, T43, T44 · **Weight:** light

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T11` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else. It holds this doc, PROGRESS, FINDINGS and the DESIGN sections, so do not read those again. A reviewer's pack has no FINDINGS: read it after your cold read.

## Goal
Stage proof: the same inputs on both sides, movement and picture match together on stretches of `W1-1` with no enemies or collectibles; add the first level smoke test and area. The matching itself was done case by case in T41 to T44; this task joins their runs into stretches compared by variables and picture at once.
## Design sections this implements
§2.2, §4.
## Reuses
T06's runner, T41 to T44's cases and scripts.
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
- [ ] A difference that appears only when variables and picture are compared together is fixed here if small; otherwise the owner is asked to add a task for it.
