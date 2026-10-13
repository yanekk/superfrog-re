# T40 — enemies-world-2

**Phase:** 2 · **Depends on:** T38; blocks T18 · **Weight:** heavy

Added by T16 (owner, 2026-10-13: "park T16 until T36–T38 and split them by world"). T16 keeps world
1; this task is world 2's share of the old "worlds 1–3" task, 17 new types per docs/09. It waits on
T38 because its frame cases need the objects in the picture and lag (T36), deaths and the restart
(T37) and the level's exit (T38).

## Goal

Every enemy and hazard object new in world 2: movement, animation, collision with the frog (both
ways: hurting the frog, being killed by it), their sound effects, and their activation and
deactivation as the camera approaches and leaves, which is what makes widescreen visual-only
(DESIGN §2.7). Types shared with world 1 are T16's; reuse them, do not port them twice.

## Design sections this implements

DESIGN §2.1, §2.7, docs/09 (object list from T07; per-marker templates by walking
`obj_flag_jumptab` at `obj_world_offset` `$100`).

## Files

- `engine/crates/sf-core/src/enemies/w2/*.rs`: one file per enemy type, named from `symbols.toml`.
- `engine/crates/sf-core/src/enemies/mod.rs`: registration, shared with T16, T17 and T41; add only
  this world's `register` lines.
- `engine/crates/sf-core/tests/enemies_w2.rs`, `engine/conform/{scripts,cases}/enemies-w2-*`

## Interface

One `ObjectKind` per original type id (T14). Shared helpers (activation range, frog-hit test) go in
`enemies/common.rs`; whichever enemy task lands first creates it and the others extend it.

## Tests

- [ ] routine conformance for each type's update and touch routines on generated states.
- [ ] frame conformance per level of world 2: a scripted run meeting every enemy type present,
      including being hurt by it and killing it where possible, with no exclusions left for enemies.
- [ ] activation: run past an enemy and back; activation and deactivation frames equal the original.

## Done when

- [ ] Every enemy type new in world 2 that docs/09 lists is ported and in a green case.
- [ ] A scripted completion of each level in world 2 matches the oracle (in `sf-conform -- all`).
- [ ] Enemy variables in `LAYOUT`, names in `symbols.toml`.
