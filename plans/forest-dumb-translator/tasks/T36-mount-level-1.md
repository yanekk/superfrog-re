# T36 — mount-level-1

**Phase:** 6 · **Depends on:** T30, T31, T35 · **Weight:** heavy

## Goal

Mount level 1's hand engine into the translated program through the seam (owner, 2026-11-11), so level 1 runs the readable engine and the translated program carries everything else, and level 1's cases stay green with it mounted.

## Design sections this implements

DESIGN §2.9, §2.10.

## Files

- `crates/fd-core/src/ported.rs` (`mounted: true` for every routine on level 1's path, or the frame mount), adapters in `crates/fd-translated/src/seam.rs`, `fd-conform` option `--mount level-1`.

## Tests

- [ ] Full: every mounted routine passes `fd-judge --seam`.
- [ ] Full: every level 1 case T30 and T31 made pass also passes with `--engine translated --mount level-1`.
- [ ] Full: `power-on-level-1` passes with the mount.

## Done when

- [ ] All three hold; the count of mounted routines is in FINDINGS.
- [ ] A routine that cannot be mounted (needs state `LAYOUT` lacks) is either given its variables in `LAYOUT` or left translated, with the reason in FINDINGS.
