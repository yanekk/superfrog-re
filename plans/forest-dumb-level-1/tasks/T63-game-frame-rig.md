# T63 — game-frame-rig

**Phase:** 6 · **Depends on:** T08, T09, T10, T22, T23, T24, T25, T26, T51, T52, T55; blocks T27 · **Weight:** small

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T63` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else. It holds this doc, PROGRESS, FINDINGS and the DESIGN sections, so do not read those again.

Added by T08 with the owner's yes (2026-11-05): `game_frame` is in T08's share of `re/translate.toml`, but it calls 20 routines owned by T09, T10, T22 to T26, T51, T52 and T55, all after T08, so its engine port cannot pass the rig until they land. T08 put the character's calls into `level::game_frame` in the original's order and did not register `game_frame`.

## Goal
Register `game_frame` (`h0+$1fec`) in `PORTED` and prove `fd_core::level::game_frame` against the original on the rig, once every routine it calls is ported: the order of its calls, the `paused` branch, and its own inline code. Show its coverage hits on the original. The full route (T27) does not run on an unproven frame.
## Design sections this implements
§3.3, §4.1, §4.3; docs/11-translation.md (`game_frame` by hand, blocked).
## Reuses
T39's `game_frame` generator (`fd-conform/src/gen/rest.rs`, base the level sync point's RAM), T32's rig, each porting task's typed state and its rig adapter (`fd-conform/tests/character_routines.rs` and siblings), T31's counter.
## Files
`fd-core/src/level/mod.rs` (`game_frame`), `fd-core/src/ported.rs` (appends), `fd-conform/tests/game_frame_routine.rs`, `docs/11-translation.md`, `smoke.toml`.
## Interface
None new: `level::game_frame` as the porting tasks left it, with whatever adapter turns the sync point's RAM into the engine's typed state and back.
## Tests
- [ ] 10 000 generated states of `game_frame` through the rig, with T39's generator; both ways of its `paused` branch (`h0+$200a`) reached.
- [ ] Every call `game_frame` makes in the listing is made by the engine in the same order (a test reading `re/translate.toml`'s `calls` for `game_frame`).
## Done when
- [ ] `game_frame` has a `PORTED` entry and passes the rig.
- [ ] Its hits on the original are shown in a case's coverage report (DESIGN §4.1).
- [ ] `docs/11-translation.md` records `game_frame` as translated by hand and proven here.
