# T70 — bonus-level

**Phase:** 2 · **Depends on:** T15; blocks T21 · **Weight:** medium

Added by T19 (owner, 2026-10-13): T19 keeps the slot machine. The bonus level (world 7, `LB*`
files) is played once after W5's last level (top_loop+$5bc: `world_load_lb`, `level_load_lb`,
`level_init_lb` h0+$8c3a, then `bonus_frame` h0+$6a64 after each death while lives last) or reached
by password entries 24–27; never from the slot. `level_init_lb` sets `shooter_mode`,
`obj_world_offset` $600, the start row by visit (h0+$8cd6), `shooter_column` 4, and spawns the first
objects (h0+$7e3e). `sync::reach` already reaches it (`WB-1`); T14's `level::load` loads it and T13
renders it (`render-wb-1`).

## Goal

The bonus level played frame-exact: its 15 object types (docs/09 "Types per world", the `$600` row
of `obj_flag_jumptab`) with `touch_bonus_enemy` (h0+$238e2), the shooter-mode camera creep
(`shooter_half_step`, `shooter_column`) and frog, the extra frog parts in `collide_all`, a death in
it (`bonus_frame`), and its end (`level_state` 2).

## Design sections this implements

DESIGN §2.1, §2.3.

## Files

- `engine/crates/sf-core/src/bonus/mod.rs` (object kinds registered into T14's dispatch)
- `engine/crates/sf-core/src/scene.rs` — `Scene::Bonus`.
- `engine/crates/sf-conform/tests/bonus_routines.rs` (not `sf-core/tests/`: the purity test keeps
  the emulator out of the pure crates' tests), `engine/conform/{scripts,cases}/bonus-*`

## Interface

`Scene::Bonus(LevelId)` returning `LevelOutcome` (T15/T37's type).

## Tests

- [ ] routine conformance for each bonus object type's update and touch routines.
- [ ] frame conformance: entering the bonus level, completing it, and failing it (a death).

## Done when

- [ ] Entering, completing and failing the bonus level each a green case.
- [ ] Bonus variables in `LAYOUT`; any table it reads comes from `Tables` with its exe offset documented.
