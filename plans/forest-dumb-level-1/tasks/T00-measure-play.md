# T00 — measure-play

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal
Replace the readings the plan rests on with measurements on the original, so nothing is ported on top of a guess. Its scripts and notes stay; any probe code that has no later use is deleted.
## Design sections this implements
§2.2, §2.3; `docs/07-executable-map.md`.
## Reuses
`amiga-oracle` (`cpu_writes`, `step`, `snapshot`), T00 of `forest-dumb-start`'s boot script, `tools/re_listing.py`, the Ghidra MCP server.
## Files
`games/forest-dumb/engine/conform/scripts/boot-to-level1.txt`, `games/forest-dumb/docs/09-play-measured.md`, `games/forest-dumb/re/symbols.toml`, `games/forest-dumb/engine/crates/fd-conform/tests/play_measured.rs`, `games/forest-dumb/smoke.toml`.
## Interface
The note answers, each with the evidence and date: first level frame from fire, fire, on the `SCENA 1` path; `game_frame` runs per displayed frame in play and under load; input mid-run takes effect on which frame; where DIW is set (copper or CPU) and the window in play; which of hardware sprites, bobs and dual playfield the level uses; random-number source and the input to it; character position, lives, energy and camera variables' addresses; what "level end" and game over set.
## Tests
- [ ] A mid-run input on frame N changes the character on the measured frame, and not sooner.
- [ ] The script reaches the level twice with identical chip RAM at the sync frame.
- [ ] `irq_count` and `game_frame` counts match the note's rate over 3 000 frames of play.
## Done when
- [ ] Every question above has an answer with evidence, or is named as open with why.
- [ ] `symbols.toml` holds each measured name; a name changed from `docs/07` is corrected there.
- [ ] `PLAN.md`'s gates can be read off the note; a result that changes a later task is raised, not patched in.
