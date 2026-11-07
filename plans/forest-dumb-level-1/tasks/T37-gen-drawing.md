# T37 — gen-drawing

**Phase:** 1 · **Depends on:** T33, T34 · **Weight:** medium

## Goal
Write and prove the generators for the `drawing` routines in `re/translate.toml`, so the batch translation (T40) judges their ports on states that reach their branches. A pass is only as strong as the states it ran on (brief, "Translation by Muse").
## Design sections this implements
§4.3.
## Reuses
T32's `Gen`, `Setup` and rig, the spike's generators (`games/forest-dumb/spikes/muse-spark/judge/src/main.rs`) as pattern, T00's and T34's notes for what each routine reads; drawing routines blit, so hardware mode.
## Files
`fd-conform/src/gen/drawing.rs`, `fd-conform/src/gen/mod.rs` (registers them), `fd-conform/tests/gen_drawing.rs`, `smoke.toml`.
## Interface
One `Generator` per `drawing` entry of `re/translate.toml`, named as there.
## Tests
- [ ] Each generator runs 2 000 states on the original with no hang, and its `RigReport::paths` is recorded in `docs/11-translation.md`.
- [ ] Each conditional branch in the routine's listing is taken both ways by some state, or is named with why no state can take it.
- [ ] No generator writes outside what its routine reads, plus the stack.
## Done when
- [ ] Every `drawing` entry has a generator, or is named as unreachable from a generated state with why.
- [ ] One routine of the batch passes through T33's service, as a check that the generators fit it.
- [ ] The reviewer checks the branch list against the listing, not the builder's account.

## Shared files
`fd-conform/src/gen/mod.rs` is shared by T35 to T39: each appends its registrations only.
