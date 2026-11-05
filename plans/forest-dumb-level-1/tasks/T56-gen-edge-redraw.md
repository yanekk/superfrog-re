# T56 — gen-edge-redraw

**Phase:** 1 · **Depends on:** T33, T34 · **Weight:** medium

Added by T34 with the owner's yes (2026-11-05), with T55: the edge redraws' 14 routines would put T36 at 24, over DESIGN §4.3's 15. T40 waits for it (`blocks T40`).

## Goal
Write and prove the generators for the `camera` routines `re/translate.toml` gives `gen_task = "T56"` (`redraw_edge_a..d` and their 10 helpers), so the batch translation (T40) judges their ports on states that reach their branches. T36 keeps the other 10.
## Design sections this implements
§4.3.
## Reuses
T32's `Gen`, `Setup` and rig, the spike's generators (`games/forest-dumb/spikes/muse-spark/judge/src/main.rs`) as pattern, T37's blitter states, T00's and T34's notes.
## Files
`fd-conform/src/gen/edge_redraw.rs`, `fd-conform/src/gen/mod.rs` (registers them), `fd-conform/tests/gen_edge_redraw.rs`, `smoke.toml`.
## Interface
One `Generator` per T56 entry of `re/translate.toml`, named as there.
## Tests
- [ ] Each generator runs 2 000 states on the original with no hang, and its `RigReport::paths` is recorded in `docs/11-translation.md`.
- [ ] Each conditional branch in the routine's listing is taken both ways by some state, or is named with why no state can take it.
- [ ] No generator writes outside what its routine reads, plus the stack.
## Done when
- [ ] Every T56 entry has a generator, or is named as unreachable from a generated state with why.
- [ ] The reviewer checks the branch list against the listing, not the builder's account.

## Shared files
`fd-conform/src/gen/mod.rs` is shared by T35 to T39, T53, T54 and T56: each appends its registrations only.
