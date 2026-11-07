# T73 — edge-latch-gap

**Phase:** 5 · **Depends on:** T24; blocks T46, T27 · **Weight:** small

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T73` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T24 with the owner's yes (2026-11-06). With T24's creatures the planner finds a new short route (`fd-plan/tests/route.rs`: `W1-1:173,58`, 7 600 frames, seed 1, 100 rounds). Played on the frozen original (`fd_plan::confirm`), `edge_latch_right` differs at pass 1211 (oracle frame 7513): 0 in the engine, `$ffff` in the original, every other compared variable equal. It differs on two passes in all. Replayed with T24's creature calls switched off, the route differs the same way, so the gap is in the camera's latches or `main_loop`'s redraw timing, not the creatures. `confirm::EDGE_LATCH_RIGHT` excludes the variable, naming this task, in `the_routes_stretch_is_confirmed_on_the_original_where_modelled` only.

## Goal
Find why the original still holds the right-edge latch at pass 1211 of that route when the engine's foreground has cleared it, and make the engine match: measured on the original (which pass set the latch, when `main_loop`'s redraw half ran and cleared it), then fixed in `level::main_loop_redraws`, `Game::foreground` or the camera, whichever is wrong.

## Design sections this implements
§2.1, §4.1.

## Reuses
T09's camera and its latches, T55's edge redraws and `main_loop_redraws`, T55's finding that a two-edge frame's redraw is cut by the next pass, T67's pass pairing, T71's way of measuring what `main_loop` reads after the next pass, the route in `fd-plan/tests/route.rs` (or its first 1 300 steps as a case script).

## Files
`engine/conform/cases/edge-latch-*.toml`, `engine/conform/scripts/edge-latch-*.txt`, `fd-conform/tests/` (a case test), `fd-core/src/**` (fixes only), `fd-plan/src/confirm.rs` and `fd-plan/tests/route.rs` (the exclusion removed), docs/09, `smoke.toml`.

## Interface
None new.

## Tests
- [ ] A case from the level sync point over the route's first 1 300 steps (frozen, as `confirm`), `edge_latch_right` compared on every pass, passing with no exclusion; it fails without the fix.
- [ ] `route.rs`'s full-route confirmation runs with no `T73` exclusion and still parts only where T46's gap is (or nowhere, once T46 has landed).

## Done when
- [ ] No exclusion names T73; `confirm::EDGE_LATCH_RIGHT` is gone.
- [ ] docs/09 states, measured and dated, when the original clears a right-edge latch the pass after it was set and when it does not.
- [ ] The fix carries a test that fails without it (`toolkit/docs/porting-routines.md` §3).
