# T22 — collectibles

**Phase:** 5 · **Depends on:** T11, T20, T21, T40, T49 · **Weight:** medium

## Goal
Port the object scan, which reads the object map and brings objects into play as the screen scrolls, then level 1's collectibles, updating score and status bar through the engine: the `objects` routines `re/translate.toml` gives T22. Most arrive as passing literal ports from T40. T24 and T25 register in the same scan. Agreement over runs is T45's (DESIGN §4.3).
## Design sections this implements
§2.1, §2.2, §4.3.
## Reuses
T40's and T49's ports and table, T33's service for a missing or failed port, T32's rig and T38's generators, T07's dispatch, T21's table. `sf-core`'s pickup code as pattern only.
## Files
`fd-core/src/objects/{mod.rs,scan.rs,collectible.rs}`, `fd-core/src/ported.rs` (appends), `fd-core/src/game.rs`, `fd-core/src/render/objects.rs`, `fd-conform/tests/collect_routines.rs`, `fd-plan/src/game.rs`, `smoke.toml`.
## Interface
```
pub fn scan(l: &mut LevelCtx, d: &Dispatch, camera: &Camera)   // the original's activation rule, per T21
impl ObjectKind for Collectible { .. }
```
Whatever T21 names; the object map writes at `h0+$2e54` reproduced.
## Tests
- [ ] Routine conformance on 10 000 states, the scan included.
- [ ] The planner's `collectibles_left` and `collectible_live` read the engine's state.
## Done when
- [ ] Every T22 routine in `re/translate.toml` has a `PORTED` entry and passes the rig.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each ported routine has a `PORTED` entry; each fix carries a test that fails without it. Non-zero hits on the original are shown by T45.
- [ ] The routines translated by hand, and why, are listed in `docs/11-translation.md`.

## Shared files
`fd-core/src/ported.rs` is shared by every porting task: entries are appended only, and whoever merges second keeps both sides' entries.
