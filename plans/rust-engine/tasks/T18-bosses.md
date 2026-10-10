# T18 — bosses

**Phase:** 2 · **Depends on:** T16, T17 · **Weight:** heavy

## Goal

Every world's boss level: the boss objects, their attack patterns and phases, hit counting, the
arena behaviour (camera lock, any special scrolling), and the win and lose transitions. Bosses reuse
the enemy helpers from T16/T17, which is why this task comes after both.

## Design sections this implements

DESIGN §2.1, docs/09.

## Files

- `engine/crates/sf-core/src/bosses/{mod.rs,w1.rs … w6.rs}` (named from `symbols.toml`)
- `engine/crates/sf-core/tests/bosses.rs`, `engine/conform/{scripts,cases}/boss-*`

## Interface

One `ObjectKind` per boss type id, registered into T14's dispatch; boss outcomes reuse `LevelOutcome`.

## Tests

- [ ] routine conformance for each boss's update and phase-change routines.
- [ ] frame conformance per boss: a scripted win, a scripted loss (frog dies), and a run that idles
      for 3 000 frames to exercise every attack pattern.

## Done when

- [ ] Every boss is beaten in a green scripted case (in `sf-conform -- all`), one short case per boss in the test block.
- [ ] Boss variables in `LAYOUT`, names in `symbols.toml`.
- [ ] No case file excludes anything a boss level does.
