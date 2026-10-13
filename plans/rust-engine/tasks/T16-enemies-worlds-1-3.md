# T16 — enemies-worlds-1-3

**Phase:** 2 · **Depends on:** T15 · **Weight:** heavy

> **Split by world, owner, 2026-10-13.** This task is now **world 1 only** (12 new types); world 2 is
> T40 and world 3 is T41. It is **parked until T38 is ✅**: its frame cases need the objects in the
> picture and lag (T36), deaths and the restart (T37) and the level's exit (T38), which its
> `Depends on` cell (T15) does not name and cannot gain without editing the row.

## Goal

Every enemy and hazard object in world 1 (Magic Woods): movement, animation, collision with the frog (both ways:
hurting the frog, being killed by it), their sound effects, and their activation and deactivation as
the camera approaches and leaves, which is what makes widescreen visual-only (DESIGN §2.7).

## Design sections this implements

DESIGN §2.1, §2.7, docs/09 (object list from T07).

## Files

- `engine/crates/sf-core/src/enemies/w1/*.rs`: one file per enemy type, named from `symbols.toml`.
- `engine/crates/sf-core/src/enemies/mod.rs`: registration, shared with the other enemy tasks (T17, T40, T41); each adds only its own `register` lines.
- `engine/crates/sf-core/tests/enemies_w1.rs`, `engine/conform/{scripts,cases}/enemies-w1-*`

## Interface

One `ObjectKind` per original type id (T14). Shared helpers (activation range, frog-hit test) go in
`enemies/common.rs`; whichever enemy task lands first creates it and the others extend it.

## Tests

- [ ] routine conformance for each type's update routine on generated states.
- [ ] frame conformance per level of world 1: a scripted run meeting every enemy type present,
      including being hurt by it and killing it where possible, with no exclusions left for enemies.
- [ ] activation: run past an enemy and back; activation and deactivation frames equal the original.

## Done when

- [ ] Every enemy type in world 1 that docs/09 lists is ported and in a green case.
- [ ] A scripted completion of each non-boss level in world 1 matches the oracle (in `sf-conform -- all`).
- [ ] Enemy variables in `LAYOUT`, names in `symbols.toml`.
