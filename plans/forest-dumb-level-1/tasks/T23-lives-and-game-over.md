# T23 — lives-and-game-over

**Phase:** 5 · **Depends on:** T11, T13, T14, T20 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T23` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else. It holds this doc, PROGRESS, FINDINGS and the DESIGN sections, so do not read those again.

## Goal
Losing a life, the original's game over screen, and restart with full lives and score zero, skipping high scores.
## Design sections this implements
§2.4.
## Reuses
`game_over_screen` (`docs/07`), T00's readings, `Scene`; T40's ports and T33's service for the routines `re/translate.toml` gives T23, proven on T32's rig.
## Files
`fd-core/src/{scene.rs,game_over.rs,character/death.rs}`, `fd-app/src/main.rs`, `fd-plan/src/game.rs` (`lives`, `dying`, `health`), `engine/conform/cases/death-*.toml`, `fd-conform/tests/death.rs`, `fd-e2e/tests/game_over.rs`, `smoke.toml`.
## Interface
```
pub enum Scene { Level, GameOver { .. }, /* T26 */ }
impl Game { pub fn lives(&self) -> i16; pub fn energy(&self) -> u16 }
```
## Tests
- [ ] Falling into a pit loses a life as the original; respawn point and state equal it.
- [ ] Last life enters the game over screen; picture matches the original.
- [ ] Past the original's game over, the comparison ends at the point high scores start.
- [ ] Restart: lives full, score zero, level as at start.
- [ ] A recording runs on through game over and the restart until Stop, and replays to the same result.
## End to end (the worker drives this)
- suite: `fd-e2e` · sizes: default, scale 1
- [ ] Scripted deaths through the real app → game over screen → level restarts, lives full.
## Done when
- [ ] Cases and end-to-end tests pass; death is in `LAYOUT`.
- [ ] The high score skip is named in the app's code and DESIGN §2.4.
- [ ] If matching its cases against the original needs more than one session, the owner is asked to add a task for the rest, split by case (DESIGN §4.3).
- [ ] Engine functions written by Muse through `tools/reshape.py --task T23 --all` and a `[T23]` table in `re/reshape.toml`, after the typed state, the rig test, one example unit and stubs by hand (`docs/11-translation.md`, "Reshaping by Muse"). docs/11's "Ported" line names Muse's functions, its spend, and any written by hand with why.
