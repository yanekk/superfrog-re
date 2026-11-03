# T24 — enemies

**Phase:** 5 · **Depends on:** T20, T21, T22, T23 · **Weight:** heavy

## Goal
Level 1's enemies: movement, drawing and hurting the character.
## Design sections this implements
§2.1, §2.2.
## Reuses
T07's dispatch, T22's scan, T21's table, T06's rig, `$abc` and `$3a18` readings, `fd-assets`' `StworkiNew`.
## Files
`fd-core/src/objects/creature*.rs`, `fd-core/src/ported.rs` (appends), `fd-core/src/render/objects.rs`, `engine/conform/cases/enemy-*.toml`, `fd-conform/tests/enemy_routines.rs`, `fd-plan/src/game.rs`, `smoke.toml`.
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
- [ ] Exclusions naming T24 are gone; cases pass with only exclusions naming T25 left. T27 proves none remain.
- [ ] Planner's danger avoids them; a replay survives them.
- [ ] A smoke test and `[[area]]`.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each ported routine has a `PORTED` entry; each fix carries a test that fails without it; each ported routine shows non-zero hits on the original in at least one case (T06's coverage report).

## Shared files
`fd-core/src/ported.rs` is shared by every porting task: entries are appended only, and whoever merges second keeps both sides' entries.
