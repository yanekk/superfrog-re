# T00 — wide-spawn-spike

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal

Measure, before T03 is built, how many objects a widescreen game keeps alive at once and whether waking
objects at the wide edge crashes anything. Throwaway code on the task branch's scratch copy: crudely
widen the spawn scans and despawn thresholds by DESIGN §2.2 (no mode switch, no `Screen` type), give
the slot table room (any hack), replay level-finishing inputs on the engine, and log. The numbers are
the deliverable; the code is deleted before the task ends.

## Design sections this implements

DESIGN §2.2, §2.3, §2.4 (sizing), §2.11.

## Files

- scratch only (`plans/real-widescreen/spike/` for the hack as a patch file and the logs, or a scratch
  folder outside the repo); nothing under `engine/` is committed.
- `plans/real-widescreen/spike/README.md`: what was run, and the table below.

## Interface

None kept. The README holds, per level (W1-1 … W6-4, bonus, final): frames replayed, the input source,
peak live slots in 4:3 and in the widened hack, peak bob draw-list and hitbox entries, the frame of the
peak, `find_free` failures or evictions, and any panic with its frame.

Input sources, engine only (no oracle): `engine/conform/scripts/` finishing scripts (`complete-*`,
`code-*`/`chain-*`, `game-whole.txt` for the whole game). A script drifts from its route once enemies
differ; that is fine, the run still visits the level.

## Tests

- [ ] none committed (spike). The README states how each number was obtained, so T03 can trust it.

## Done when

- [ ] `spike/README.md` has the table for every level and the bonus and final levels, and the
  recommended `WIDE_SLOTS` (peak + margin, with the margin's reason).
- [ ] Panics or softlocks found in the hack are listed with level and frame, each as a FINDINGS row.
- [ ] No engine code committed: no commit of this task touches `engine/` (`git log --stat` of the
  task's own commits; other tasks' merged code may be on the branch).
