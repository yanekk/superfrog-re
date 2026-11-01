# T22 — collectibles

**Phase:** 5 · **Depends on:** T11, T21 · **Weight:** medium

## Goal
Level 1's collectibles work and update score and status bar, end to end through the engine.
## Design sections this implements
§2.1, §2.2.
## Reuses
T07's dispatch, T21's table, T06's rig. `sf-core`'s pickup code as pattern only.
## Files
`fd-core/src/objects/collectible.rs`, `fd-core/src/render/objects.rs`, `conform/cases/collect-*.toml`, `fd-conform/tests/collect_routines.rs`, `fd-plan/src/game.rs`, `smoke.toml`.
## Interface
```
impl ObjectKind for Collectible { .. }
```
Whatever T21 names; the object map writes at `h0+$2e54` reproduced.
## Tests
- [ ] Routine conformance.
- [ ] Pick up each kind; score and bar equal the original.
- [ ] A collected item stays gone after scrolling away and back.
- [ ] Milestones of a collecting run met by the engine alone.
## Done when
- [ ] Exclusions naming T22 are gone from cases.
- [ ] A test plays through collecting on engine and original, and an `[[area]]` is added.
