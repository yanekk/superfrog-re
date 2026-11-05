# T09 — camera-scroll

**Phase:** 2 · **Depends on:** T07, T40 · **Weight:** medium

## Goal
Port the camera and scroll state, including the map's edge-redraw flags (`redraw_edge_a..d`), which the foreground loop runs: the `camera` routines `re/translate.toml` gives T09. Most arrive as passing literal ports from T40; this task turns them into engine code and proves each on the rig. Agreement over whole runs is T43's (DESIGN §4.3).
## Design sections this implements
§2.1, §4.3; `docs/07` main loop.
## Reuses
T40's ports and table, T33's service for a missing or failed port, T32's rig and T36's generators, `amiga-clock`.
## Files
`fd-core/src/level/camera.rs`, `fd-core/src/ported.rs` (appends), `fd-core/src/{game.rs,layout.rs}` (calls it from `Game::step`), `fd-conform/tests/camera_routines.rs`, `re/symbols.toml`, `smoke.toml`.
## Interface
```
pub struct Camera { pub x: i32, pub y: i32, pub edges: EdgeFlags }
impl Camera { pub fn step(&mut self, target: (i32, i32), level: &Level) }
```
## Tests
- [ ] Routine conformance on 10 000 states, with T36's generators.
## Done when
- [ ] Every T09 routine in `re/translate.toml` has a `PORTED` entry and passes the rig.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each ported routine has a `PORTED` entry; each fix carries a test that fails without it. Non-zero hits on the original are shown by T43.
- [ ] The routines translated by hand, and why, are listed in `docs/11-translation.md`.

## Shared files
`fd-core/src/ported.rs` is shared by every porting task: entries are appended only, and whoever merges second keeps both sides' entries.
