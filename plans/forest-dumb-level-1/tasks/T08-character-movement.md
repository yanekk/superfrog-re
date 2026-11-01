# T08 — character-movement

**Phase:** 2 · **Depends on:** T07 · **Weight:** heavy

## Goal
Port the character's running, jumping, falling and collision against the collision plane, routine by routine.
## Design sections this implements
§2.1, §2.2; `docs/05` collision.
## Reuses
T06's rig; `sf-conform/tests/collision_routines.rs` as pattern; `amiga-input`.
## Files
`fd-core/src/character/{mod.rs,physics.rs,anim.rs}`, `fd-core/src/level/collision.rs`, `fd-conform/tests/character_routines.rs`, `engine/conform/cases/character-*.toml`, `conform/scripts/character-*.txt`, `re/symbols.toml`, `docs/` note, `smoke.toml`.
## Interface
```
pub struct Character { pub x: i32, pub y: i32, pub vx: i16, pub vy: i16, pub frame: u16, /* the original's fields */ }
impl Character { pub fn step(&mut self, i: FrameInput, c: &CollisionPlane) }
pub fn solid(c: &CollisionPlane, x: i32, y: i32) -> bool
```
## Tests
- [ ] 10 000 generated states on each ported routine.
- [ ] Runs: right to first gap and jump; wall; pit; slope; standing jump; jump held versus tapped.
- [ ] Left and top level edges.
- [ ] Input on the frame of a landing.
## Done when
- [ ] Variables match the original on each run on clear stretches.
- [ ] Unmodelled effects are listed as exclusions with their task.
- [ ] Character variables are in `LAYOUT` and `symbols.toml`.
