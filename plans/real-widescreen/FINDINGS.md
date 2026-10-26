# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts** (over 60 rows or 15 KB). Never drop a ✅ row or its date, or a term
somebody would grep for.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user
changed.

| Date | | Finding |
|---|---|---|
| 2026-10-26 | 📌 | T04: WW-1's camera stays at x 949 in both modes, inside widescreen's clamp (81..4375), so widescreen does not move it and the witch's path is the original's. Her fight starts in a 1 000-step widescreen run. |
| 2026-10-26 | 📌 | T04: a frog sprite parked at $195 needs HSTART $215, past 9 bits (wraps to $015, left of the widened window). T09 must keep a parked frog hidden in the strips. |
| 2026-10-26 | 🐞 | T01 review: `load::enter` (`Game::new` at a level) builds its LevelCtx before `set_widescreen` can be called, so a directly started level runs `start_point` and `scan_screen` narrow. Fixed in T04: `set_widescreen` enters an unplayed level again in the new mode. |
| 2026-10-26 | 📌 | T01 review: the restart's `level_init` and game over's vertical blank get `c.screen` through `Front.screen` and `Cx.screen`, not `Machine`. Nothing reads it yet, so T02 and T04 tests should include a widescreen restart to lock it. |
| 2026-10-22 | 📌 | Plan review: bonus markers 11 (chains fly off) and 55 (level end) act when `scan_right_column` reaches them, so widescreen's wider column ends the bonus level about 3 s sooner. Owner accepted, DESIGN D12. |
| 2026-10-22 | 📌 | Plan review: under machine load 13–18, `recorder_app` `every_ending_action_discarded…` hit its 150 s `--timeout` in the block's sf-e2e line; alone it took 85 s, and the line re-ran green. A timing flake, not a defect. |
| 2026-10-22 | 🔄 | Owner: widescreen changes the game (wake things at the wide edge), overturning rust-engine DESIGN §2.7 "visual only" for widescreen alone. 4:3 stays the original. DESIGN D1. |
| 2026-10-22 | 📌 | Planner survey: slots are referenced by absolute address (slot+$4c, +$50, hitbox +8, chain +$64, bonus `SHOTS`); code sits on both sides of `obj_slots` and `bob_records`. Six literal `(0..20)` loops plus `PASS_WRITES`. DESIGN §2.4. |
| 2026-10-22 | 📌 | Planner: today's widescreen paints 65 columns each side from the map only; `compose` overwrites the bobs the buffer holds 8–24 px past the window. Black past the map: 49 px at the left end, about 39 at the right. |
