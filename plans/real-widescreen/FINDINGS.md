# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts** (over 60 rows or 15 KB). Never drop a ✅ row or its date, or a term
somebody would grep for.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user
changed.

| Date | | Finding |
|---|---|---|
| 2026-10-27 | 🐞 | T06: `recorder_follow` `a_recording_the_original_leaves_is_put_back_on_it` (once `recorder_compare` `a_short_run_compares_same`) panics "vAmiga refused a memory write: … not chip or slow RAM" on d0f5147 too. Suspect T03's wide range in LAYOUT written to the oracle (`sf-conform/src/lib.rs` 1409). Block line. |
| 2026-10-27 | 🐞 | T06, fixed: switched off, the last extra freed on the screen kept its drawing: the draw list dropped the extras' free list once `WIDE_LIVE` hit 0. `build_bob_draw_list` now keeps the wide list while a free extra bob has drawings (`extra_left_drawn`). |
| 2026-10-26 | 📌 | T03: with T02–T05's rules W6-2's ghost run peaks at 92 live of 100 (`sf-conform/tests/widescreen_capacity.rs`). Extras are compared as 0 in 4:3 conformance, not excluded: a 4:3 write there fails a case. |
| 2026-10-26 | 📌 | T03: switched off with no extra live, the draw list is the original's, so an extra free bob's last drawing is never erased. T06 should test a switch-off right after an extra is freed on screen. |
| 2026-10-26 | 🐞 | T00 review, fixed: the spike's ghost run started its 4:3 game with the hack on, so W6-3's ghost stopped at frame 1362. Re-run: W6-3 peaks 73 live over 8400 frames; `WIDE_SLOTS` 80 stands. |
| 2026-10-26 | 📌 | T00: no panic or softlock in any level, bonus or final with spawns widened (`spike/README.md`). Given room, 95 live at once (W6-2), at most 57 inside the wide view (W4-2): stale objects stay until evicted. `WIDE_SLOTS` 80 recommended. |
| 2026-10-26 | 📌 | T00: the wide draw list peaks at 96 entries against `bob_draw_list`'s 40 places (`$a8` bytes); hitboxes peak at 74 of 127. A wide replay of a 4:3 route drifts and mostly dies: enemies differ. |
| 2026-10-26 | 📌 | T04 review: WB-1's start camera moves from x 17 to 82 in widescreen (start_point's clamp), so the bonus level starts 65 px, about 2.6 s of creep, further on; with D12 it ends about 5.6 s sooner. |
| 2026-10-26 | 📌 | T04: WW-1's camera stays at x 949 in both modes, inside widescreen's clamp (81..4375), so widescreen does not move it and the witch's path is the original's. Her fight starts in a 1 000-step widescreen run. |
| 2026-10-26 | 📌 | T04: a frog sprite parked at $195 needs HSTART $215, past 9 bits (wraps to $015, left of the widened window). T09 must keep a parked frog hidden in the strips. |
| 2026-10-26 | 🐞 | T01 review: `load::enter` (`Game::new` at a level) builds its LevelCtx before `set_widescreen` can be called, so a directly started level runs `start_point` and `scan_screen` narrow. Fixed in T04: `set_widescreen` enters an unplayed level again in the new mode. |
| 2026-10-26 | 📌 | T02 review probe: W5-3 widescreen, camera moving left, the row scan woke an object at cx+432 freed next frame by the right despawn (cx+433); §2.2's whole-cell rounding puts the row's end 15 px past it. 4:3 run: none. |
| 2026-10-26 | 📌 | T02: task says run `cargo test --workspace` once; the parallel worker rule forbids whole-suite runs, so it was skipped. The 4:3 conformance cases outside the block were not run for T02; regression set and enemies_w1/w3 green. |
| 2026-10-26 | 📌 | T02: widened row and start scans reach 6 cells past the map's right end (into the next row) with the camera at the 4:3 limit; T04's narrower wide clamp keeps them within the 4-cell row padding. |
| 2026-10-26 | 📌 | T01 review: the restart's `level_init` and game over's vertical blank get `c.screen` through `Front.screen` and `Cx.screen`, not `Machine`. Nothing reads it yet, so T02 and T04 tests should include a widescreen restart to lock it. |
| 2026-10-22 | 📌 | Plan review: bonus markers 11 (chains fly off) and 55 (level end) act when `scan_right_column` reaches them, so widescreen's wider column ends the bonus level about 3 s sooner. Owner accepted, DESIGN D12. |
| 2026-10-22 | 📌 | Plan review: under machine load 13–18, `recorder_app` `every_ending_action_discarded…` hit its 150 s `--timeout` in the block's sf-e2e line; alone it took 85 s, and the line re-ran green. A timing flake, not a defect. |
| 2026-10-22 | 🔄 | Owner: widescreen changes the game (wake things at the wide edge), overturning rust-engine DESIGN §2.7 "visual only" for widescreen alone. 4:3 stays the original. DESIGN D1. |
| 2026-10-22 | 📌 | Planner survey: slots are referenced by absolute address (slot+$4c, +$50, hitbox +8, chain +$64, bonus `SHOTS`); code sits on both sides of `obj_slots` and `bob_records`. Six literal `(0..20)` loops plus `PASS_WRITES`. DESIGN §2.4. |
| 2026-10-22 | 📌 | Planner: today's widescreen paints 65 columns each side from the map only; `compose` overwrites the bobs the buffer holds 8–24 px past the window. Black past the map: 49 px at the left end, about 39 at the right. |
