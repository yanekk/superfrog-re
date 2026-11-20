# T29 — level-drill

**Phase:** 6 · **Depends on:** T15, T28 · **Weight:** light

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T29` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else. It holds this doc, PROGRESS, FINDINGS and the DESIGN sections, so do not read those again. A reviewer's pack has no FINDINGS: read it after your cold read.

## Goal
A worker uses the finished game as a person would, judges it against DESIGN and the original, and closes the plan's notes, confirming or breaking the playbook's lessons as game two.
## Design sections this implements
§1, §2.
## Reuses
`amiga-e2e` OS layer, `plans/forest-dumb-level-1/FINDINGS.md`, root CLAUDE.md knowledge rules.
## Files
`fd-e2e/tests/level_drill.rs`, `toolkit/PLAYBOOK.md`, `toolkit/docs/reuse-for-another-game.md`, `games/forest-dumb/CLAUDE.md`, `games/forest-dumb/docs/ROADMAP.md`, `README.md` if it lists games or toolkit features.
## End to end (the worker drives this)
- suite: `fd-e2e`, OS layer · sizes: default, scale 1
- [ ] Through the level with planned route; deaths to game over; each enemy and mechanic; recording during all of it; second run after restart.
- [ ] Unlisted things a person tries: holding keys, opposite keys, focus lost, resize mid-play, pad unplugged.
- [ ] Each screen saved to scratch and judged against the oracle's.
## Done when
- [ ] Decidable defects each have a test; two-answer choices went to the owner.
- [ ] A dated worker-driven row is in FINDINGS.
- [ ] Lessons promoted by rule 2, carried with their `learned on:` lines; the tier name check passes.
- [ ] The worker-only `permissions.deny` list added to `.claude/settings.json` on 2026-11-06 (DESIGN §4.4) is removed, so it never reaches `main`.
