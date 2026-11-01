# T24 — enemies

**Phase:** 5 · **Depends on:** T21, T23 · **Weight:** heavy

## Goal
Level 1's enemies: movement, drawing and hurting the character.
## Design sections this implements
§2.1, §2.2.
## Reuses
T07's dispatch, T21's table, T06's rig, `$abc` and `$3a18` readings, `fd-assets`' `StworkiNew`.
## Files
`fd-core/src/objects/creature*.rs`, `fd-core/src/render/objects.rs`, `conform/cases/enemy-*.toml`, `fd-conform/tests/enemy_routines.rs`, `fd-plan/src/game.rs`, `smoke.toml`.
## Interface
```
impl ObjectKind for Creature { .. }   // one per kind T21 lists, value 1 to 6
```
Random-number use follows T00; seed in state.
## Tests
- [ ] Routine conformance for each kind.
- [ ] Each kind's path on the original over a run.
- [ ] Spawn and despawn at the screen edges, and after scrolling back.
- [ ] Hit reduces energy and matches timing, then T23's path.
- [ ] Kill behaviour if the game has one.
## Done when
- [ ] Exclusions naming T24 are gone, with whole-run cases passing.
- [ ] Planner's danger avoids them; a replay survives them.
- [ ] A smoke test and `[[area]]`.
