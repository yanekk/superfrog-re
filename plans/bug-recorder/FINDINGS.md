# Findings log

**What the build taught.** Read the rows touching the task you pick up. Newest first, forty words a
row, counted. Whoever appends, compacts (over 60 rows or 15 KB). Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-20 | 📌 | T03: `SyncPoint::Setup` equals `start_level` at frame 0 on all 26 levels and easy W1-1/W4-3 but `playfield_clean`. WW-1 also needed `music_fade` and `map_blink` from the engine (W6-4's tally and load). First run ~30 s a level; cached ~7 s. |
| 2026-10-20 | 🔄 | Owner (T02): Start Level offers 26 levels (`sf_core::recording::levels()`), not 29: W6-2alt..W6-4alt are never loaded when a level is started by its code. T08's dropdown takes `levels()`; `LevelSetup::validate` refuses the alt maps. |
| 2026-10-20 | 🔄 | Owner (T02): screen name `world end` added (end-of-world picture); awards, hall of talent and attract demo read `title`; a fade's blank frame reads as the place before it (`Game::location` keeps the last). |
| 2026-10-20 | 📌 | T02: `Game::start_level` runs about 2 700 frames from power-on, 4 s a start in the dev build (opt-level 2); WW-1 adds W6-4's forced exit and tally. T06/T08 may want it off the main thread. |
| 2026-10-20 | 🐞 | T01 review: `Recording::parse` preallocated the header's `frames`, so `frames 4000000000` asked for 128 GB. Fixed: `MAX_FRAMES` (a day, 4 320 000) refused by `parse` and `Recording::validate`; T06's recorder must stop or refuse past it. |
| 2026-10-20 | 📌 | T00 review: W4-3 `complete-w4-3` retimed frame for frame, faithful runner with the engine paced: GREEN, 388 late, 1 resynced. Paced legs are expected to hold where unpaced legs fail; T05's test is the per-leg measurement. |
| 2026-10-20 | 🔄 | Owner (T00): the dialog drops Hard (the original's options screen offers only easy/normal; `options_input` caps `opt_game_mode` at 1) and Lives defaults to 3, the game's own new-game value (`opt_lives` 0 at power-on), not 5. |
| 2026-10-20 | 📌 | T00 Q1: start (b) chosen. Power-on, options screen, code typed, unseen; at frame 0 only title/load leftovers differ, which T03 writes into the original. 2 000 faithful frames green on W1-1, W1-2, W4-3, WB-1 (`bonus-fly`), easy and normal. |
| 2026-10-20 | 📌 | T00 Q1: start (a) (`Start::Level` + pokes) differs at frame 0 in 14–21 gameplay variables (`p41_channels` music place, `fades`, `display_timer`, front-end state) and fails at frame 1 everywhere. |
| 2026-10-20 | 📌 | T00 Q1: (b) through a transition: W1-1 exit forced, `tally-points`, W1-2's load and 500 frames, green for 5/1, 3/0 lives/mode; W4-3 likewise. Hard written on the options screen also equal and green (moot). |
| 2026-10-20 | 📌 | T00 Q2: retime frame for frame, not by pass (the frog reads the joystick every vertical blank). Hold the original through an instant engine load except `timed_screen` frames; re-anchor at each level frame 0 and slot entry. |
| 2026-10-20 | 📌 | T00 Q2: lag-free stretches stay on the recording every frame: W1-1 exit→tally→W1-2 (anchor 573=573) and tally→slot (759-frame load held)→W1-2 (1955↔1196). Faithful green. |
| 2026-10-20 | 🐞 | T00 Q2: under lag the original leaves the recording. W1-1 `frog-run-right`: heart lost at 615 one side, re-converged at restart (12 frames apart). W4-3 `complete-w4-3`: score 350, frog 400; recorded death ~650 absent on the original. |
| 2026-10-20 | 📌 | T00 Q2 fallback: legs re-synced from the original, engine unpaced. W1-1: 8 passes 0/119 legs differ, 32 1/30. W4-3: one leg 8 px at ~2257 for every N≥1 (a late pass), 64 2/28. N=8, paced within legs. |
| 2026-10-20 | 📌 | T00 Q3: cell `((frog_x+0x17)/16, (frog_y-0x10)/16)` is open with ground under it on 1 120/1 176 grounded W1-1 frames (`(x+8)/16,(y-8)/16`: 623); eyeballed at three spots against `levels_annotated`: the frog's lower half. |
| 2026-10-20 | 🐞 | T00, no task: engine candidates seen in passing. Easy W1-1 `frog-run-right` frame 519: engine sound on channel 2, original none. WB-1 `frog-run-right` frame 687 `frog_x` differs from `AtLevel(WB-1)` too. |
| 2026-10-20 | 📌 | `Start::Level` has no seed: `rng_state` comes from the boot image, and `level_entry.rs` `NOT_MODELLED` lists it with the front end's fades and music place. Why D2 is measured by T00. |
| 2026-10-20 | 📌 | `opt_game_mode` and `opt_lives` are read by `frog_new_game` before the level loads; poking them after `Start::Level` is too late (`frog/mod.rs`). |
| 2026-10-20 | 📌 | Full workspace suite: 6 247 s cold in a fresh worktree; `whole_game.rs` "the window fell behind" failed on a loaded machine. Block scoped to this plan's parts (D11). `.pir/settings.json` runs Python only. |
| 2026-10-20 | 🔄 | Owner: Lives is a 3/5/7 dropdown like the options screen, replacing 0–99; Energy a 1–4 dropdown. |
