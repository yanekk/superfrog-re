# T07 — level-entry

**Phase:** 2 · **Depends on:** T05, T06 · **Weight:** medium

## Goal
The engine enters level 1 in the same state the original has at the sync point, and establishes the level scene and object dispatch that later tasks extend rather than restructure.
## Design sections this implements
§2.3, §3.3.
## Reuses
`toolkit/docs/porting-routines.md` §4 (the port register); `plans/rust-engine/tasks/T14-frog-and-level.md`'s shape, `level_init` (`docs/07`), `amiga-input`'s `FrameInput`.
## Files
`fd-core/src/{lib.rs,game.rs,scene.rs,level/mod.rs,state.rs,layout.rs}`, `fd-conform/tests/level_entry.rs`, `re/symbols.toml`, `smoke.toml`.
## Interface
```
pub struct GameData { .. }  // from fd-assets and the executable's tables, read at run time
pub enum Start { Level1 }
pub enum Scene { Level, /* T23 GameOver, T26 LevelEnd */ }
impl Game { pub fn new(data: Arc<GameData>, start: Start) -> Game; pub fn step(&mut self, i: FrameInput); pub fn scene(&self) -> &Scene }
pub trait ObjectKind { fn update(&self, o: &mut Object, l: &mut LevelCtx); fn draw(&self, o: &Object, out: &mut DrawList); }
pub fn register(d: &mut Dispatch, value: u8, k: &'static dyn ObjectKind);
```
Object values are the original's, so a transplanted list maps one to one. A value with no registered kind is not skipped: `step` records `Event::Unported(value)`, every case fails on one, and the app writes it to stderr and leaves the object out.
## Tests
- [ ] All `LAYOUT` variables equal the oracle at the sync point.
- [ ] Two `Game::new` give equal state; clones step identically.
- [ ] Missing data file reports, not panics.
- [ ] Purity test.
## Done when
- [ ] `Game` starts and steps idle on a clear stretch with variables equal for 200 frames, or differences excluded by name; its case's goal is `idle = true`.
- [ ] Dispatch is documented in a comment for T22, T24, T25.
- [ ] A test spawns an unregistered value and sees `Event::Unported`, and a case containing it fails.
