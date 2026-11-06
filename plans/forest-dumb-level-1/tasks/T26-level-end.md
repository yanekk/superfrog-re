# T26 — level-end

**Phase:** 6 · **Depends on:** T11, T13, T14, T23 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T26` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else. It holds this doc, PROGRESS, FINDINGS and the DESIGN sections, so do not read those again.

## Goal
The original's end-of-level sequence, then the "Level 1 complete" message until fire, then restart.
## Design sections this implements
§2.5.
## Reuses
`next_world` (`docs/07`), T00's end reading, the status font if suitable; plain `Scene`; T40's ports and T33's service for the routines `re/translate.toml` gives T26, proven on T32's rig.
## Files
`fd-core/src/{scene.rs,level_end.rs}`, `fd-core/src/render/message.rs`, `fd-app/src/main.rs`, `fd-plan/src/game.rs` (`playing`), `engine/conform/cases/level-end.toml`, `fd-conform/tests/level_end.rs`, `fd-e2e/tests/level_end.rs`, `smoke.toml`.
## Interface
```
pub enum Scene { Level, GameOver { .. }, LevelEnd { .. }, Complete }
impl Game { pub fn finished(&self) -> bool }
```
The `Complete` scene is commented as temporary until world 2 lands.
## Tests
- [ ] Sequence matches the original to the point it starts loading world 2, from a transplanted state near the end.
- [ ] The comparison stops there, by a named exclusion.
- [ ] `Complete` waits for fire, ignoring other input; fire restarts with full lives and score zero, state equal to `Game::new`.
- [ ] A recording runs on through `Complete` and the restart until Stop, and replays to the same result.
## End to end (the worker drives this)
- suite: `fd-e2e` · sizes: default, scale 1
- [ ] Script reaches the state → message fits at each size and is unclipped → fire restarts the level.
## Done when
- [ ] Case and end-to-end tests pass, and a milestone for level end exists for T27.
- [ ] If matching its cases against the original needs more than one session, the owner is asked to add a task for the rest, split by case (DESIGN §4.3).
- [ ] Engine functions written by Muse through `tools/reshape.py --task T26 --all` and a `[T26]` table in `re/reshape.toml`, after the typed state, the rig test, one example unit and stubs by hand (`docs/11-translation.md`, "Reshaping by Muse"). docs/11's "Ported" line names Muse's functions, its spend, and any written by hand with why.
