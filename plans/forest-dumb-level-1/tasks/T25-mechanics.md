# T25 — mechanics

**Phase:** 6 · **Depends on:** T11, T20, T21, T22 · **Weight:** heavy

## Goal
Level 1's remaining mechanics, whatever T21 lists: expected flames, vanishing blocks, launchers, doors, secret passages.
## Design sections this implements
§2.1; `docs/05`, `docs/06`.
## Reuses
T07's dispatch, T22's scan, T21, `_ELEMENTY`, `_Plomien`, `_ZNIKAJKA`, `_WYBIJACZKA` readers.
## Files
`fd-core/src/objects/{flame,vanish,launcher,*}.rs`, `fd-core/src/render/objects.rs`, `engine/conform/cases/mechanic-*.toml`, `fd-conform/tests/mechanic_routines.rs`, `fd-plan/src/facts.rs`, `smoke.toml`.
## Interface
One `ObjectKind` per mechanic T21 lists; high values `$40`+ per the `$a06` tables.
## Tests
- [ ] Routine conformance per mechanic.
- [ ] A scripted run through each mechanic's spot on both sides.
- [ ] Its timing cycle, and state after scrolling away.
- [ ] Mechanic's effect on the planner's `gates` and `open`.
## Done when
- [ ] T21's list is implemented; exclusions naming T25 are gone, others name T24. T27 proves none remain but sound.
- [ ] A test uses each mechanic; `[[area]]` added.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each ported routine has a `PORTED` entry; each fix carries a test that fails without it; each ported routine shows non-zero hits on the original in at least one case (T06's coverage report).
