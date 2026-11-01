# T23 — lives-and-game-over

**Phase:** 5 · **Depends on:** T11, T13 · **Weight:** medium

## Goal
Losing a life, the original's game over screen, and restart with full lives and score zero, skipping high scores.
## Design sections this implements
§2.4.
## Reuses
`game_over_screen` (`docs/07`), T00's readings, `Scene`.
## Files
`fd-core/src/{scene.rs,game_over.rs,character/death.rs}`, `fd-app/src/main.rs`, `conform/cases/death-*.toml`, `fd-conform/tests/death.rs`, `fd-e2e/tests/game_over.rs`, `smoke.toml`.
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
- [ ] A recording across the restart stops cleanly.
## End to end (the worker drives this)
- suite: `fd-e2e` · sizes: default, scale 1
- [ ] Scripted deaths through the real app → game over screen → level restarts, lives full.
## Done when
- [ ] Cases and end-to-end tests pass; death is in `LAYOUT`.
- [ ] The high score skip is named in the app's code and DESIGN §2.4.
