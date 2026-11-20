# T74 — switched-on-redraw-race

**Phase:** 6 · **Depends on:** T47, T73; blocks T27, T48 · **Weight:** medium

**Start (owner, 2026-11-06, DESIGN §4.4):** run `python3 -I games/forest-dumb/tools/task_pack.py T74` (a reviewer adds `--review`) and read the pack it names whole, in one step, before anything else.

Added by T47 with the owner's yes (2026-11-09). Since T47 no case freezes springs or the animated cells (`h0_6882`, `h0_24ce`), so the original's passes are longer. Measured on `mechanic-vanish` (frames 6593 to 6595): pass `$125` starts while `main_loop` is in `redraw_edge_d` for the last pass's right latch, ends in the next step, and when the redraw ends `main_loop` clears `frame_flag`, swallowing pass `$125`'s; its down latch waits a pass, so `edge_latch_down` is `$ffff` at pass `$126`'s end where the engine has cleared it. On `mechanic-spring` (pass 495) the down row is then scanned a pass late on the original, its coins join other animated-cell lists, and from frame 6845 they spin out of step: the play area's picture differs on 77 frames. On `edge-latch-route` `edge_latch_right` differs on 2 passes, and T73's measured rule (rows of 9 bobs or more are cut by the next pass) no longer holds there: the original cuts a row of no bobs at pass 328 and no row of 9 bobs remains.

## Goal
Make `mechanic-spring` and `mechanic-vanish` pass in full when the original's long pass drops the strip-redraw signal, without modelling clock ticks (DESIGN §2.1, sub-frame timing is out). Possible ways, each proven frame by frame: the comparison passing the original's measured cut to the engine for that pass (as T67 pairs input), or accepting only the versions the original could show, as T71 did for the status bar. Restate or replace T73's nine-bob rule with the animated cells running, re-enable its check, and decide whether the engine's T73 rule (`ROW_BOBS_CUT`) stays, since it was measured in a frozen world.

## Design sections this implements
§2.1, §2.2, §4.1.

## Reuses
T73's cut-row measurement (`fd-conform/tests/edge_latch.rs`), T71's measured cut (`interrupted_pc`, `redraw_cut`, `Engine::cut_redraw`), T67's pass pairing, T47's cases and the probe described above (watch `main_loop`'s `clr.w` of each latch, `h0+$2d0`, `$2f8`, `$320`, `$348`, and of `frame_flag`, `h0+$352`, per step).

## Files
`engine/conform/cases/*.toml` (exclusions naming T74 removed), `fd-conform/src/**` (the runner, if the cut is handed over), `fd-conform/tests/edge_latch.rs`, `fd-conform/tests/mechanic_cases.rs`, `fd-core/src/**` (fixes only), docs/09, docs/10, `smoke.toml`.

## Interface
Whatever the chosen way needs on `fd_conform::Engine`; nothing the app or the planner calls.

## Tests
- [ ] `mechanic-spring` compares the play area's picture with no exclusion and passes.
- [ ] `mechanic-vanish`, `mechanic-spring`, `camera-bottom-right`, `death-water` and `edge-latch-route` compare every edge latch with no exclusion and pass.
- [ ] T73's rule check (`a_row_of_nine_bobs_is_cut_by_the_next_pass_and_the_engine_cuts_the_same_rows`) restated for the animated cells running and no longer `#[ignore]`d.
- [ ] Whatever is accepted per frame fails on a frame the original could not show (a test that fails without the check).

## Done when
- [ ] No exclusion or `#[ignore]` names T74.
- [ ] docs/09 states, measured and dated, when a long pass swallows `main_loop`'s `frame_flag`, and what the engine or the runner does about it.
- [ ] The engine's T73 rule is kept, changed or removed, with the reason in docs/09.
