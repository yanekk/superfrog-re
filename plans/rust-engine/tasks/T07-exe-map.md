# T07 — exe-map

**Phase:** 1 · **Depends on:** T02, T04, T05 · **Weight:** heavy

## Goal

Map the original's five executables well enough that every later task starts from a shared chart
rather than rediscovering it: what each executable is for and when it is loaded, the main loop and the
vertical-blank interrupt, input reading, the random-number generator and its seeding, the object
system (list layout, type dispatch, per-type update/draw routines), the level loader and which chunks
make a level, the scene flow (intro, title, level, slot, bonus, boss, end), disk-swap prompts, and the
RAM variables that hold game state. This is a map, not a full understanding: name things, locate
them, measure the few properties the design depends on, and stop.

## Design sections this implements

DESIGN §2.1 (lag, RNG), §2.3, §2.4 (stragglers), §2.10 (which files make a level), §3.3 (layout).

## Files

- `docs/08-executable-map.md` — roles, load order, memory map, main loop, interrupt, scene flow.
- `docs/09-object-system.md` — object record layout, type list per world with routine addresses,
  boss and slot-machine entry points.
- `re/symbols.toml` — every routine and variable named here.
- `engine/crates/sf-oracle/tests/exe_map_claims.rs` — the measurable claims below as tests.
- `docs/03-atn-codec.md` — straggler verification result.

## Measurements it must make (each a test against the oracle)

- [ ] **Lag:** over the scripted runs available (title idle, W1-1 first 3 000 frames with movement, a
      busy scene found by reading the object list), does the main loop ever complete in more than one
      frame? Record per scene. If yes, stop and raise it with the owner (PLAN.md open decision).
- [ ] **RNG:** routine, state variable and seed source. If the seed reads beam position, a CIA timer
      or anything CPU-timing dependent, raise it with the owner.
- [ ] **Interrupt work:** what runs in the vertical-blank interrupt (music? input? object updates?)
      versus the main loop.
- [ ] **Stragglers:** after the game loads `d1_c16`, `d2_c39` and `d4_c16`, their decompressed bytes in
      memory equal `sf-disk::atn` output (or the reason they can't be compared, e.g. never loaded).
- [ ] **Level composition:** which chunks and memory blocks the game loads for one level besides T7MP
      (object placement, sprite bank), confirmed by memory dumps at level start.

## Done when

- [ ] docs/08 and docs/09 exist, every routine and variable they name is in `symbols.toml`, and the
      measurement tests pass in the test block.
- [ ] The object-type count per world is reported in PROGRESS notes, with a recommendation whether
      T16–T18 need splitting; any split is agreed with the owner and amended into PLAN.md.
- [ ] The lag and RNG answers are recorded in FINDINGS and, if either needs a decision, it was put to the owner.

## Needs a person

Only if the lag or RNG measurement shows timing dependence:

```
Read the measurement in FINDINGS (which scenes lag, or what seeds the RNG).
```

Expect: a short choice between ways of defining frame-exact for that case, set out by the worker.
Tell me: which definition to adopt.
