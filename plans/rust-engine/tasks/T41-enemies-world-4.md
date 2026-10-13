# T41 — enemies-world-4

**Phase:** 2 · **Depends on:** T38; blocks T18 · **Weight:** heavy

Added by T17 (owner, 2026-10-13): T17 was parked until T36–T38 and split by world. World 4 has 23
new types (15 that hurt, 1 item, the rest scenery and blocks); W3 and W4 use markers 24–28 as
objects, not solid tiles (docs/09). Routines per marker come from walking `obj_flag_jumptab` at
`obj_world_offset` `$300` (docs/09).

## Goal

Every enemy, hazard and scenery object of world 4 (W4-1…W4-4): movement, animation, collision with
the frog both ways, their sound effects, and their activation and deactivation as the camera
approaches and leaves (DESIGN §2.7).

## Design sections this implements

DESIGN §2.1, §2.7, docs/09 (object list from T07).

## Files

- `engine/crates/sf-core/src/enemies/w4/*.rs`: one file per type, named from `symbols.toml`.
- `engine/crates/sf-core/src/enemies/mod.rs`: registration; add only this world's `register` lines.
- `engine/crates/sf-core/tests/enemies_w4.rs`, `engine/conform/{scripts,cases}/enemies-w4-*`

## Interface

One `ObjectKind` per original template, registered into T14's dispatch (`level/objects.rs`, as T15's
items do). Shared helpers (activation range, frog-hit test) go in `enemies/common.rs`, extended, not
duplicated.

## Tests

- [ ] routine conformance for each type's init, update and touch routines on generated states.
- [ ] frame conformance per level of the world: a scripted run meeting every type present,
      including being hurt by it and killing it where possible, with no exclusions left for enemies.
- [ ] activation: run past an enemy and back; activation and deactivation frames equal the original.

## Done when

- [ ] Every type docs/09 lists for the world is ported and in a green case.
- [ ] A scripted completion of each non-boss level of the world matches the oracle (in `sf-conform -- all`).
- [ ] Its variables in `LAYOUT`, names in `symbols.toml`.
