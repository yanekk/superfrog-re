# T55 — edge-redraw

**Phase:** 2 · **Depends on:** T09 · **Weight:** medium

Added by T34 with the owner's yes (2026-11-05): `redraw_edge_a..d`, named in T09's goal, run from `main_loop`, outside `game_frame`'s tree, and with what they call are 14 routines, which would put T09 at 24, over DESIGN §4.3's 15. T10, T41, T42 and T43 wait for it (`blocks T10, T41, T42, T43`).

## Goal
Port the foreground's edge redraws, which draw the map's new row or column into the play screen and keep `collision_map` built as the view scrolls (T00: `collision_map` is play state they write): the routines `re/translate.toml` gives T55 (`redraw_edge_a..d` and 10 helpers, 936 instructions). `redraw_edge_c` is `by_hand` (181 instructions of its own flow, over T33's 180). Most arrive as passing literal ports from T40; this task turns them into engine code on typed state and proves each on the rig. T09 ports the edge flags that trigger them.
## Design sections this implements
§2.1, §4.1, §4.3.
## Reuses
T40's ports and table (`docs/11-translation.md`), T33's service for a missing or failed port, T32's rig and T56's generators, T09's camera and edge flags, T00's note on `collision_map` and the play screen, `fd-assets`' tiles.
## Files
`fd-core/src/level/redraw.rs`, `fd-core/src/level/collision.rs` (shared with T08), `fd-core/src/ported.rs` (appends), `fd-core/src/game.rs` (runs the redraws where `main_loop` does), `fd-conform/tests/edge_redraw_routines.rs`, `re/symbols.toml`, `docs/` note, `smoke.toml`.
## Interface
The redraws on typed state: the edge flags from T09 in, the play screen and `collision_map` written. Where the engine runs them in a pass follows `main_loop`'s order, and `docs/` says what a redraw cut short by the next interrupt does (T00: inferred, not seen).
## Tests
- [ ] 10 000 generated states on each ported routine, through the rig, with T56's generator.
- [ ] After a scroll in each direction, the engine's `collision_map` equals the original's (one case from T07).
## Done when
- [ ] Every T55 routine in `re/translate.toml` has a `PORTED` entry and passes the rig.
- [ ] Follows `toolkit/docs/porting-routines.md`: each routine's interruptible points are listed in its `docs/` note before it is ported; each fix carries a test that fails without it. Non-zero hits on the original are shown by T41 to T43.
- [ ] The routines translated by hand, and why, are listed in `docs/11-translation.md`. `h0_681c`, `h0_6852` (object slots) and `h0_6882` (`h0_62b6`'s animated-cell list) are read as part of the redraw; if they belong to T22 or T25, say so in `FINDINGS.md` rather than moving them.

## Shared files
`fd-core/src/ported.rs` entries are appended only; `fd-core/src/level/collision.rs` is shared with T08.
