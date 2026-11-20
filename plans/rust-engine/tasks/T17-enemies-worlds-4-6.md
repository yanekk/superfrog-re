# T17 — enemies-worlds-4-6

**Phase:** 2 · **Depends on:** T15 · **Weight:** heavy

> If T07's object-type count shows this is more than one session, it is split with the owner and this
> doc is superseded by the split; say so here with the date.

> **Split 2026-10-13 (owner), and this doc is superseded by it.** T17 was parked until T36–T38
> and closed as the split itself: world 4 is T41, world 5 T39, world 6 (less the witch, T18's) T40,
> each after T38 and blocking T18. Nothing below is built under T17.

## Goal

Every enemy and hazard object in worlds 4–6: movement, animation, collision with the frog (both ways:
hurting the frog, being killed by it), their sound effects, and their activation and deactivation as
the camera approaches and leaves, which is what makes widescreen visual-only (DESIGN §2.7).

## Design sections this implements

DESIGN §2.1, §2.7, docs/09 (object list from T07).

## Files

- `engine/crates/sf-core/src/enemies/w4_6/*.rs`: one file per enemy type, named from `symbols.toml`.
- `engine/crates/sf-core/src/enemies/mod.rs`: registration, shared with the other enemy task; each adds only its own `register` lines.
- `engine/crates/sf-core/tests/enemies_w4_6.rs`, `engine/conform/{scripts,cases}/enemies-w4_6-*`

## Interface

One `ObjectKind` per original type id (T14). Shared helpers (activation range, frog-hit test) go in
`enemies/common.rs`; whichever enemy task lands first creates it and the other extends it.

## Tests

- [ ] routine conformance for each type's update routine on generated states.
- [ ] frame conformance per level of these worlds: a scripted run meeting every enemy type present,
      including being hurt by it and killing it where possible, with no exclusions left for enemies.
- [ ] activation: run past an enemy and back; activation and deactivation frames equal the original.

## Done when

- [ ] Every enemy type in worlds 4–6 that docs/09 lists is ported and in a green case.
- [ ] A scripted completion of each non-boss level in worlds 4–6 matches the oracle (in `sf-conform -- all`).
- [ ] Enemy variables in `LAYOUT`, names in `symbols.toml`.
