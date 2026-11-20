# T53 — gen-character-2

**Phase:** 1 · **Depends on:** T33, T34 · **Weight:** medium

Added by T34 with the owner's yes (2026-11-05): T35's share was 20 routines, over DESIGN §4.3's 15, which binds generator tasks too. T40 waits for it (`blocks T40`).

## Goal
Write and prove the generators for the `character` routines `re/translate.toml` gives `gen_task = "T53"` (`char_step_4be6`, `h0_4c28`, `h0_4c6a`, `h0_4d32`, `h0_4d54`, `read_joystick`), so the batch translation (T40) judges their ports on states that reach their branches. T35 keeps the other 14.
## Design sections this implements
§4.3.
## Reuses
T32's `Gen`, `Setup` and rig, the spike's generators (`games/forest-dumb/spikes/muse-spark/judge/src/main.rs`) as pattern, T00's and T34's notes for what each routine reads.
## Files
`fd-conform/src/gen/character_anim.rs`, `fd-conform/src/gen/mod.rs` (registers them), `fd-conform/tests/gen_character_anim.rs`, `smoke.toml`.
## Interface
One `Generator` per T53 entry of `re/translate.toml`, named as there.
## Tests
- [ ] Each generator runs 2 000 states on the original with no hang, and its `RigReport::paths` is recorded in `docs/11-translation.md`.
- [ ] Each conditional branch in the routine's listing is taken both ways by some state, or is named with why no state can take it.
- [ ] No generator writes outside what its routine reads, plus the stack.
## Done when
- [ ] Every T53 entry has a generator, or is named as unreachable from a generated state with why.
- [ ] The reviewer checks the branch list against the listing, not the builder's account.

## Shared files
`fd-conform/src/gen/mod.rs` is shared by T35 to T39, T53, T54 and T56: each appends its registrations only.
