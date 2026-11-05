# T08 — character-movement

**Phase:** 2 · **Depends on:** T07, T40 · **Weight:** medium

## Goal
Port the character's running, jumping, falling and collision against the collision plane: the `character` routines `re/translate.toml` gives T08 (T34). Most arrive as passing literal ports from T40; this task turns them into engine code on typed state and proves each on the rig. Agreement over whole runs is T41's and T42's (DESIGN §4.3).
## Design sections this implements
§2.1, §4.3; `docs/05` collision.
## Reuses
T40's ports and table (`docs/11-translation.md`), T33's service for a routine whose port is missing or failed (better generator, then retry; after three failed tries the builder translates it), T32's rig and T35's generators, `sf-conform/tests/collision_routines.rs` as pattern, `amiga-input`.
## Files
`fd-core/src/character/{mod.rs,physics.rs,anim.rs}`, `fd-core/src/{game.rs,layout.rs}` (calls it from `Game::step`), `fd-core/src/level/collision.rs`, `fd-core/src/ported.rs` (appends), `fd-conform/tests/character_routines.rs`, `re/symbols.toml`, `docs/` note, `smoke.toml`.
## Interface
```
pub struct Character { pub x: i32, pub y: i32, pub vx: i16, pub vy: i16, pub frame: u16, /* the original's fields */ }
impl Character { pub fn step(&mut self, i: FrameInput, c: &CollisionPlane) }
pub fn solid(c: &CollisionPlane, x: i32, y: i32) -> bool
```
## Tests
- [ ] 10 000 generated states on each ported routine, through the rig, with T35's generator.
- [ ] `Game::step` calls the character in the original's order; one idle case from T07 still passes.
## Done when
- [ ] Every T08 routine in `re/translate.toml` has a `PORTED` entry and passes the rig.
- [ ] Character variables are in `LAYOUT` and `symbols.toml`.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each ported routine has a `PORTED` entry; each fix carries a test that fails without it. Non-zero hits on the original are shown by T41 and T42.
- [ ] The routines translated by hand, and why, are listed in `docs/11-translation.md`.

## Shared files
`fd-core/src/ported.rs` is shared by every porting task: entries are appended only, and whoever merges second keeps both sides' entries.
