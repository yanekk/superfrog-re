# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts** (over 60 rows or 15 KB). Never drop a ✅ row or its date, or a term
somebody would grep for.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user
changed.

| Date | | Finding |
|---|---|---|
| 2026-10-27 | 🐞 | T12: W6-4 has no widescreen route. `sf-plan --widescreen` takes every coin, then stalls 13 cells from the exit at `W6-4:284,62` on seeds 1 and 2, guided by the set-C route or the owner's recording. The 4:3 planner stalled there too. |
| 2026-10-27 | 📌 | T12: W6-1 widescreen seed 1 spent its 30 min 3 coins short; seed 2 finished (`wide-w6-1.txt`). The other 22 normal levels finished on seed 1, guided by `routes/app/`. |
| 2026-10-27 | 📌 | T12: the app's Start Level dialog leaves Spud off, and without it `final-win`'s fire never hits the witch (game over). `wide-ww-1.txt` starts with Spud ticked (`# spud: on`), as `final-win`'s start has it. |
| 2026-10-27 | 📌 | T12: of the 24 set-C routes replayed in widescreen only W2-2 and W3-1 still finish; the rest lose a life. The wide W1-1 route also finishes in 4:3; the wide W5-1 one does not. |
| 2026-10-27 | 📌 | T12: `widescreen_complete -- --ignored` replays 25 routes green in 304 s under load 10–19, W6-4's file missing. WB-1 ends at 7804 in widescreen, `chain-wb-1` at 8094 in 4:3 (D12). |
| 2026-10-27 | 📌 | T11 drill, worker-driven: W1-1, W2-3, W3-1, W4-2, W5-1, W6-2 (bench routes, ghost and plain), WB-1, WW-1, ≥3 000 frames each, both ends, Spud, bonus shots, witch, app switches, record/replay, fullscreen. No black past map, map pop-in, seam tear or crash. |
| 2026-10-27 | 🔄 | T11, owner: switching widescreen while paused keeps the paused 4:3 picture, centred in the wider window, until play resumes (`widescreen_drill.rs`). WW-1's strips keep showing the map, scraps past the castle's right wall included. |
| 2026-10-27 | 📌 | T11: objects freed inside a strip are taken coins' sparkle ending (`+$5c`), child shots or bonus pieces; none by a despawn edge. WB-1's centre buffer and draw list differ under the panel by the left seam; not visible. |
| 2026-10-27 | 📌 | T11: `cargo test --workspace` not run (parallel worker rule, as T02); the drill changed no engine or app code, only added `widescreen_drill.rs`. |
| 2026-10-27 | 🐞 | T10, fixed: `Game::start_level` plays the level code in 4:3, so Start Level or bug replay switched wide entered W1-1 at cx 16 (80 frames black past the map, 65 px jump). `set_widescreen` before the first step restarts it wide. |
| 2026-10-27 | 🔄 | T10, owner: during `--replay` View ▸ Widescreen is disabled; the recording alone switches the mode and the item's tick shows it. |
| 2026-10-27 | 🐞 | T09 review, fixed: a WW-1 sprite's own data fetches SPRxCTL `$000c` (bit 3 set); read as `WIDE_HSTART_BIT` it drew stray pixels at the right strip's edge. The renderer now takes the bit from copper writes only. |
| 2026-10-27 | 📌 | T09 review drill, worker-driven: W1-1 widescreen rendered with the frog at both ends, whole in each strip. A probe of every level's right-edge sprite pixels (600 frames) found only WB-1's frog after the fix. |
| 2026-10-27 | 📌 | T09 measured: widescreen frog sprites show from x −$61 to $190 (HSTART $1F..$210; picture left $38). At a level's end the frog stands at $169/$179; $180..$190 occurs only while the camera catches up (2–6 frames per run). |
| 2026-10-27 | 📌 | T09: HSTART bit 9 rides in SPRxCTL bit 3 (`WIDE_HSTART_BIT`, unused on OCS), set by `place_frog_sprites` in widescreen only, read only for a widened level picture from a copper write. A parked frog ($191, HSTART $211) lands past the picture. |
| 2026-10-27 | 🐞 | T08 review, fixed: level_routines (bob_draw, scroller), level_entry, recorder_setup_start and `bug::Follow::writes` compared or wrote `wide_bobs_0/1`, which a 4:3 game fills too; each now skips `layout::engine_only`. |
| 2026-10-27 | 📌 | T08 review: a bob the drawer culls at the buffer's right edge shows in the strip but not the centre's last 2 columns (W5-1 frame 561). Owner keeps the centre faithful; T11's drill judges it. No `$fc00` bob seen. |
| 2026-10-27 | 📌 | T08: `scroll_column` is not the shown buffer's address (it stayed in buffer 1 while buffer 0 showed); the shown buffer is the play-field lines' bitplane pointer, which the strips now read. |
| 2026-10-27 | 📌 | T08: the centre loses a pass's bob pixels where the scroller redraws map rows (camera moving vertically, restart jumps) and below the lower part's 16-line `pushed` bound; the lists record both so strips match. `redraw_cell` and `$fc00` bobs are not followed. |
| 2026-10-27 | 🐞 | T06 review, sweep: T02's `widescreen_spawn.rs` `a_restart_in_widescreen_wakes_the_strips` checks the left strip only; in T06's like case the wide left-column edge scan alone woke a coin there, so it may pass without the wide start scan. Not fixed. |
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
