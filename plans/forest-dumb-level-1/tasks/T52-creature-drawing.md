# T52 — creature-drawing

**Phase:** 5 · **Depends on:** T10, T22 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T52` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else. It holds this doc, PROGRESS, FINDINGS and the DESIGN sections, so do not read those again.

Added by T34 with the owner's yes (2026-11-05): T24's share was 17 routines, over DESIGN §4.3's 15. T46 waits for it (`blocks T46`).

## Goal
Port the drawing of the five creature slots: restoring the screen behind them and cutting them onto it, the routines `re/translate.toml` gives T52 (`h0_2c7e`, `h0_5e44`, `h0_2d4c`, `h0_5820`, 421 instructions). `h0_5e44` is `by_hand` (200 instructions of its own flow, over T33's 180), so the builder translates it. Picture agreement over runs is T46's.
## Design sections this implements
§2.1, §4.3.
## Reuses
T40's ports and table, T33's service for a missing or failed port, T32's rig and T37's generators, T10's `render/` and blitter model, T22's object slots (`h0+$67da`, `h0+$67ec`), T00's note on bobs (`$9fca`, `$5fca` cuts from `StworkiNew`, C and D the screen), `fd-assets`' `StworkiNew`.
## Files
`fd-core/src/render/objects.rs`, `fd-core/src/ported.rs` (appends), `fd-conform/tests/creature_drawing_routines.rs`, `re/symbols.toml`, `docs/` note, `smoke.toml`.
## Interface
Drawing of the object slots T22 defines onto T10's play screen, called from `Game::step` where `game_frame` calls `h0+$2c7e` and `h0+$2d4c`, including `game_frame`'s swap of the slot sets around `h0+$2c7e`.
## Tests
- [ ] 10 000 generated states on each ported routine, through the rig, with T37's generator.
- [ ] A picture check: creatures on the play screen match the original at one frame of T21's level 1 states.
## Done when
- [ ] Every T52 routine in `re/translate.toml` has a `PORTED` entry and passes the rig.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each fix carries a test that fails without it.
- [ ] `h0_5e44`, translated by hand, and why, is listed in `docs/11-translation.md`.
- [ ] Engine functions written by Muse through `tools/reshape.py --task T52 --all` and a `[T52]` table in `re/reshape.toml`, after the typed state, the rig test, one example unit and stubs by hand (`docs/11-translation.md`, "Reshaping by Muse"). docs/11's "Ported" line names Muse's functions, its spend, and any written by hand with why.

## Shared files
`fd-core/src/render/objects.rs` is shared with T10, T22, T24 and T25; `fd-core/src/ported.rs` entries are appended only.
