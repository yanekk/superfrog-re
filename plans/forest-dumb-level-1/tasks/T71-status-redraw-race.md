# T71 — status-redraw-race

**Phase:** 5 · **Depends on:** T70; blocks T27 · **Weight:** small

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T71` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T70 with the owner's yes (2026-11-06). `main_loop`'s status bar redraw (`h0_6a96`, after pass `32k`) is interrupted by the next pass. In `death-spikes` the spikes take a life in pass 1441 (frame 7741); the original's hearts and lives digits show `$22` from frame 7742, so its redraw read `lives` after pass 1441 ran. The engine runs the whole redraw before the next pass (`Game::foreground`) and shows `$23` until pass 1472's redraw. About one life lost in 32 falls in that pass.

## Goal
The status bar's numbers and icons show what the original's interrupted redraw shows, frame by frame: which of `h0_6a96`'s reads happen after the next pass has run, measured, and the engine's redraw split there.

## Design sections this implements
§2.4, §4.1.

## Reuses
T10's `render::status` (`Status::update`, the digit and icon routines), T44's picture pairing (the status bar redrawn after pass `32k` shows in its frame), T70's `death-spikes` case and its trace of lives against the icons' bytes (`h4+$7d06`, frames 6526 to 7876).

## Files
`fd-core/src/game.rs` (`foreground`, `picture`), `fd-core/src/render/status.rs`, `engine/conform/cases/death-spikes.toml` (its `T71` exclusion removed), `fd-conform/tests/death.rs`, `fd-conform/tests/picture_cases.rs`.

## Interface
None new.

## Tests
- [ ] Measure, on the original, the beam line at which `h0_6a96` reads each of `countdown`, `lives`, `items_left` and `score`, against the line pass `32k + 1` starts (level-3 at line 233), idle and on `death-spikes`.
- [ ] `death-spikes` passes with its `rows 226..250 cols 24..96` exclusion (T71) removed, pictures and variables on every frame.
- [ ] The cases that already show the status bar (`picture_cases.rs`, `stretches.rs`) still pass with nothing tolerated.

## Done when
- [ ] Cases pass; no exclusion names T71.
- [ ] docs/09 or docs/06 states when the redraw reads each number, measured and dated.
