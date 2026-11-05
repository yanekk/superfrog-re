# T25 — mechanics

**Phase:** 6 · **Depends on:** T11, T20, T21, T22 · **Weight:** medium

## Goal
Port level 1's remaining mechanics, whatever T21 lists: expected flames, vanishing blocks, launchers, doors, secret passages; the `objects` routines `re/translate.toml` gives T25. Most arrive as passing literal ports from T40. Agreement over runs is T47's (DESIGN §4.3).
## Design sections this implements
§2.1, §4.3; `docs/05`, `docs/06`.
## Reuses
T40's and T49's ports and table, T33's service for a missing or failed port, T32's rig and T38's generators, T07's dispatch, T22's scan, T21, `_ELEMENTY`, `_Plomien`, `_ZNIKAJKA`, `_WYBIJACZKA` readers.
## Files
`fd-core/src/objects/{flame,vanish,launcher,*}.rs`, `fd-core/src/ported.rs` (appends), `fd-core/src/render/objects.rs`, `fd-conform/tests/mechanic_routines.rs`, `fd-plan/src/facts.rs`, `smoke.toml`.
## Interface
One `ObjectKind` per mechanic T21 lists; high values `$40`+ per the `$a06` tables.
## Tests
- [ ] Routine conformance on 10 000 states per mechanic.
- [ ] Mechanic's effect on the planner's `gates` and `open`.
## Done when
- [ ] T21's list is implemented; every T25 routine in `re/translate.toml` has a `PORTED` entry and passes the rig.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each ported routine has a `PORTED` entry; each fix carries a test that fails without it. Non-zero hits on the original are shown by T47.
- [ ] The routines translated by hand, and why, are listed in `docs/11-translation.md`.

## Shared files
`fd-core/src/ported.rs` is shared by every porting task: entries are appended only, and whoever merges second keeps both sides' entries.
