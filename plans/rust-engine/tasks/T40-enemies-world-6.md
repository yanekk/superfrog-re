# T40 — enemies-world-6

**Phase:** 2 · **Depends on:** T17, T38; blocks T18 · **Weight:** heavy

Added by T17 (owner, 2026-10-13): T17 was parked until T36–T38 and split by world. World 6 and the
final level have 11 new types; the witch (marker 1, template h0+$221f6, init $259ae, update $25a16)
is T18's, not this task's. W6 has two map sets (FINDINGS, T31): cover both. Routines per marker come
from walking `obj_flag_jumptab` at `obj_world_offset` `$500` (docs/09).

## Goal

Every enemy, hazard and scenery object of world 6 and the final level except the witch: movement,
animation, collision with the frog both ways, their sound effects, and their activation and
deactivation as the camera approaches and leaves (DESIGN §2.7).

## Design sections this implements

DESIGN §2.1, §2.7, docs/09 (object list from T07).

## Files

- `engine/crates/sf-core/src/enemies/w6/*.rs`: one file per type, named from `symbols.toml`.
- `engine/crates/sf-core/src/enemies/mod.rs`: registration; add only this world's `register` lines.
- `engine/crates/sf-core/tests/enemies_w6.rs`, `engine/conform/{scripts,cases}/enemies-w6-*`

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

- [ ] Every type docs/09 lists for the world and the final level, less the witch is ported and in a green case.
- [ ] A scripted completion of each non-boss level of the world matches the oracle (in `sf-conform -- all`).
- [ ] Its variables in `LAYOUT`, names in `symbols.toml`.
