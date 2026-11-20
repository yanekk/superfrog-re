# T54 — gen-objects-2

**Phase:** 1 · **Depends on:** T33, T34 · **Weight:** medium

Added by T34 with the owner's yes (2026-11-05): T38's share was 21 routines, over DESIGN §4.3's 15, which binds generator tasks too. T40 waits for it (`blocks T40`).

## Goal
Write and prove the generators for the `objects` routines `re/translate.toml` gives `gen_task = "T54"`: the five creature slots' movement (`h0_2fae`, `h0_2fea` and its helpers `h0_320a`, `h0_3222`, `h0_32f8`, `h0_3322`, `h0_3530`, `h0_3566`, `h0_35a0`, `h0_35dc`) and the slots' sort by y (`h0_3698`), so the batch translation (T40) judges their ports on states that reach their branches. T38 keeps the other 10.
## Design sections this implements
§4.3.
## Reuses
T32's `Gen`, `Setup` and rig, the spike's generators for `h0+$2fae` and `h0+$2dd6` (`games/forest-dumb/spikes/muse-spark/judge/src/main.rs`), T34's notes.
## Files
`fd-conform/src/gen/objects_move.rs`, `fd-conform/src/gen/mod.rs` (registers them), `fd-conform/tests/gen_objects_move.rs`, `smoke.toml`.
## Interface
One `Generator` per T54 entry of `re/translate.toml`, named as there.
## Tests
- [ ] Each generator runs 2 000 states on the original with no hang, and its `RigReport::paths` is recorded in `docs/11-translation.md`.
- [ ] Each conditional branch in the routine's listing is taken both ways by some state, or is named with why no state can take it.
- [ ] No generator writes outside what its routine reads, plus the stack.
## Done when
- [ ] Every T54 entry has a generator, or is named as unreachable from a generated state with why.
- [ ] The reviewer checks the branch list against the listing, not the builder's account.

## Shared files
`fd-conform/src/gen/mod.rs` is shared by T35 to T39, T53, T54 and T56: each appends its registrations only.
