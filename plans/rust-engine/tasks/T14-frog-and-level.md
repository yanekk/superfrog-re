# T14 — frog-and-level

**Phase:** 2 · **Depends on:** T06, T11, T12, T13 · **Weight:** heavy

## Goal

A playable level with only the frog: loading a level into the engine the way the original does, the
frog's movement, running, jumping, falling and collision against `BODY` flags (including flag 38 fake
walls, docs/07), the camera, and the frog's sound effects. Establishes the level scene and the object
dispatch table that items, enemies and bosses register into, so later tasks extend rather than
restructure it.

## Design sections this implements

DESIGN §2.1, §3.3, docs/07.

## Files

- `engine/crates/sf-core/src/level/{mod.rs,load.rs,collision.rs,camera.rs}` — `mod.rs` owns the object dispatch.
- `engine/crates/sf-core/src/frog/{mod.rs,physics.rs,anim.rs}`
- `engine/crates/sf-core/src/scene.rs` — adds `Scene::Level`.
- `engine/crates/sf-core/tests/frog_routines.rs` — routine conformance.
- `engine/conform/scripts/frog-*.txt`, `engine/conform/cases/frog-*.toml`

## Interface

```rust
pub trait ObjectKind { fn update(&self, obj: &mut Object, lvl: &mut LevelCtx); fn draw(&self, obj: &Object, out: &mut DrawList); }
pub fn register(dispatch: &mut Dispatch, type_id: u16, kind: &'static dyn ObjectKind);   // type ids = the original's
impl Game { pub fn new(data: &GameData, start: Start) -> Game }   // Start::Level(LevelId) enters the level as the original does
```

Object type ids are the original's, so a transplanted object list maps one to one.

## Tests

- [ ] routine conformance for the frog's update and collision routines on 10 000 generated states each.
- [ ] frame conformance, objects frozen out of the comparison only where `LAYOUT` does not yet model
      them (listed in the case file): W1-1 run right to the first gap and jump, 1 500 frames; walk into
      a wall; fall into a pit; walk through a fake wall at `W1-1:94,51`.
- [ ] one scripted run per world's first level, 1 000 frames of movement, frog variables, camera and frame equal.
- [ ] `Start::Level` for each of the 29 levels reaches frame 0 with frog position and camera equal to the oracle.

## Done when

- [ ] The cases above are green; the short ones in the test block.
- [ ] Everything the original does in a level that is not yet modelled is listed in the case files as excluded, with the task that will model it.
- [ ] Frog variables are in `LAYOUT` and named in `symbols.toml`.
