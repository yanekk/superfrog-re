# T79 — bonus-level-frames

**Split from:** T70. Carries all of T70's "Done when" frame items: entering, completing and
failing the bonus level, each a green case, and `Scene::Bonus` returning `LevelOutcome`.

**Phase:** 2 · **Depends on:** T70, T37, T75, T76, T77, T78; blocks T21 · **Weight:** medium

## Goal

The bonus level played frame-exact, using T75–T78's routines and T36/T37's objects in the
picture, deaths and `LevelOutcome`. Read T70's doc for the original's flow (`world_load_lb`,
`level_load_lb`, `level_init_lb` h0+$8c3a, the start row by visit h0+$8cd6, `bonus_frame`
h0+$6a64 after each death while lives last). The end is marker 55's `level_state` 2 (T77).

## Files

- `engine/crates/sf-core/src/scene.rs`: `Scene::Bonus`. `level/load.rs`: call h0+$7e3e and model
  the visit count (`level_init` leaves both out today).
- `engine/conform/{scripts,cases}/bonus-*`, `engine/crates/sf-conform/tests/bonus_routines.rs`

## Interface

`Scene::Bonus(LevelId)` returning `LevelOutcome` (T37's type).

## Tests

- [ ] Routine conformance for `bonus_frame`.
- [ ] Frame conformance: entering the bonus level, completing it, and failing it (a death).

## Done when

- [ ] Entering, completing and failing the bonus level each a green case.
- [ ] Any table it reads comes from `Tables` with its exe offset documented.
