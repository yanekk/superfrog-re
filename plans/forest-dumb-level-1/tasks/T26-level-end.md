# T26 — level-end

**Phase:** 6 · **Depends on:** T11, T13 · **Weight:** medium

## Goal
The original's end-of-level sequence, then the "Level 1 complete" message until fire, then restart.
## Design sections this implements
§2.5.
## Reuses
`next_world` (`docs/07`), T00's end reading, the status font if suitable; plain `Scene`.
## Files
`fd-core/src/{scene.rs,level_end.rs}`, `fd-core/src/render/message.rs`, `fd-app/src/main.rs`, `conform/cases/level-end.toml`, `fd-conform/tests/level_end.rs`, `fd-e2e/tests/level_end.rs`, `smoke.toml`.
## Interface
```
pub enum Scene { Level, GameOver { .. }, LevelEnd { .. }, Complete }
impl Game { pub fn finished(&self) -> bool }
```
The `Complete` scene is commented as temporary until world 2 lands.
## Tests
- [ ] Sequence matches the original to the point it starts loading world 2, from a transplanted state near the end.
- [ ] The comparison stops there, by a named exclusion.
- [ ] `Complete` waits for fire, ignoring other input; fire restarts as new.
- [ ] A recording stops at restart.
## End to end (the worker drives this)
- suite: `fd-e2e` · sizes: default, scale 1
- [ ] Script reaches the state → message fits at each size and is unclipped → fire restarts the level.
## Done when
- [ ] Case and end-to-end tests pass, and a milestone for level end exists for T27.
