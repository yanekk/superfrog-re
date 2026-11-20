# T16 — enemies-worlds-1-3

**Phase:** 2 · **Depends on:** T15 · **Weight:** heavy

> **Split 2026-10-13 (owner), and this doc is superseded by it.** T16 was parked until T36–T38
> and closed as the split itself: world 1 is T42, world 2 T43, world 3 T44, each after T38 and
> blocking T18 (T42 also T33, T43 also T39). Nothing below is built under T16.

## Goal

Every enemy and hazard object in world 1 (Magic Woods): movement, animation, collision with the frog (both ways:
hurting the frog, being killed by it), their sound effects, and their activation and deactivation as
the camera approaches and leaves, which is what makes widescreen visual-only (DESIGN §2.7).

## Design sections this implements

DESIGN §2.1, §2.7, docs/09 (object list from T07).

## Files

- `engine/crates/sf-core/src/enemies/w1/*.rs`: one file per enemy type, named from `symbols.toml`.
- `engine/crates/sf-core/src/enemies/mod.rs`: registration, shared with the other enemy tasks (T39–T44); each adds only its own `register` lines.
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
