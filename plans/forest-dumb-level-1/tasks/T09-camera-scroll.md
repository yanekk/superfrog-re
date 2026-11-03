# T09 — camera-scroll

**Phase:** 2 · **Depends on:** T07 · **Weight:** medium

## Goal
Port the camera and scroll state, including the map's edge-redraw flags (`redraw_edge_a..d`), which the foreground loop runs, so the screen follows the character as in the original.
## Design sections this implements
§2.1; `docs/07` main loop.
## Reuses
T06; `amiga-clock`.
## Files
`fd-core/src/level/camera.rs`, `fd-core/src/ported.rs` (appends), `fd-core/src/{game.rs,layout.rs}` (calls it from `Game::step`), `fd-conform/tests/camera_routines.rs`, `engine/conform/cases/camera-*.toml`, `re/symbols.toml`, `smoke.toml`.
## Interface
```
pub struct Camera { pub x: i32, pub y: i32, pub edges: EdgeFlags }
impl Camera { pub fn step(&mut self, target: (i32, i32), level: &Level) }
```
## Tests
- [ ] Routine conformance on 10 000 states.
- [ ] Scroll limits at all four level ends.
- [ ] Past column 240's filler is never entered if the original does not.
- [ ] Edge flags agree across a long run.
## Done when
- [ ] Camera variables match the original on a scripted run that scrolls both ways.
- [ ] Foreground timing of redraws is matched or named as an exclusion.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each ported routine has a `PORTED` entry; each fix carries a test that fails without it; each ported routine shows non-zero hits on the original in at least one case (T06's coverage report).

## Shared files
`fd-core/src/ported.rs` is shared by every porting task: entries are appended only, and whoever merges second keeps both sides' entries.
