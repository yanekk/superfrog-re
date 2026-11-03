# T22 — collectibles

**Phase:** 5 · **Depends on:** T11, T20, T21 · **Weight:** medium

## Goal
Build the object scan, which reads the object map and brings objects into play as the screen scrolls, then level 1's collectibles, updating score and status bar through the engine. T24 and T25 register in the same scan.
## Design sections this implements
§2.1, §2.2.
## Reuses
T07's dispatch, T21's table, T06's rig. `sf-core`'s pickup code as pattern only.
## Files
`fd-core/src/objects/{mod.rs,scan.rs,collectible.rs}`, `fd-core/src/ported.rs` (appends), `fd-core/src/game.rs`, `fd-core/src/render/objects.rs`, `engine/conform/cases/collect-*.toml`, `fd-conform/tests/collect_routines.rs`, `fd-plan/src/game.rs`, `smoke.toml`.
## Interface
```
pub fn scan(l: &mut LevelCtx, d: &Dispatch, camera: &Camera)   // the original's activation rule, per T21
impl ObjectKind for Collectible { .. }
```
Whatever T21 names; the object map writes at `h0+$2e54` reproduced.
## Tests
- [ ] Routine conformance, the scan included: objects enter and leave play where the original's do.
- [ ] Pick up each kind; score and bar equal the original.
- [ ] A collected item stays gone after scrolling away and back.
- [ ] Milestones of a collecting run met by the engine alone.
## Done when
- [ ] Exclusions naming T22 are gone from cases.
- [ ] A test plays through collecting on engine and original, and an `[[area]]` is added.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each ported routine has a `PORTED` entry; each fix carries a test that fails without it; each ported routine shows non-zero hits on the original in at least one case (T06's coverage report).

## Shared files
`fd-core/src/ported.rs` is shared by every porting task: entries are appended only, and whoever merges second keeps both sides' entries.
