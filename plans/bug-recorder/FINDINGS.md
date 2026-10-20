# Findings log

**What the build taught.** Read the rows touching the task you pick up. Newest first, forty words a
row, counted. Whoever appends, compacts (over 60 rows or 15 KB). Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-20 | 📌 | T11 review: the five `recorder_drill` OS-layer tests were not rerun (`IOConsoleLocked` true); their pass rests on the builder's run. Chain test, script-layer recorder tests and a recorder mutation were run. |
| 2026-10-20 | 📌 | T11 drill, worker-driven: Start Level, W1-1 played to its exit, tally, slot machine, W1-2, STOP with a note; `--replay` and `bug replay` equal. Every ending action, ⌘Q, ⌘2, close, each answer. Nothing clipped at 1×, 3×, widescreen. |
| 2026-10-20 | 📌 | T11: a 20-minute recording (60 000 frames) saved as a 116 KB zip; `sf-conform bug replay` took 40 s, screenshot equal; `--replay` in the app runs in real time (20 min) to "replay finished at frame 60000". |
| 2026-10-20 | 🔄 | Owner (T11): STOP while the player had paused leaves the game paused after Save or Discard (DESIGN §2.4's "resumes" undoes STOP's own pause). Pinned by `recorder_drill` OS test. |
| 2026-10-20 | 🐞 | T11, fixed in the OS harness: the menu bar read while a Start Level start blocks the main thread (`AXError -25204`) panicked; a close-button press that opens the question reports -25204 though delivered. `ax::menu_bar` retries; `os_close_window` accepts it. |
| 2026-10-20 | 📌 | T11: `recorder_start_level` OS tests lose a typed key or Return about 1 run in 4 in a batch, 6/6 alone; other sessions' app windows on screen (another worktree's e2e) take focus. Not fixed. |
| 2026-10-20 | 📌 | T11: `plan_exit --engine` plans on the engine from Start Level's start; W1-1's floating-ledge coins trap the beam (`--skip 0,0,150,83` from 2784). In the slot machine fire every 80 frames spends the coins; held up+fire enters it. |
| 2026-10-20 | 📌 | T11: a note pasted with CRLF line ends gets two spaces per break (the field converts each character before `SaveForm` sees it). Left as is. |
| 2026-10-20 | 📌 | T09 review: OS layer (`SF_E2E_OS=1`) finds no AX window while the console is locked (`IOConsoleLocked` true): "no element … within 20 s", base `controls.rs` too. Owner skipped the rerun; T11's drill re-checks the bar. |
| 2026-10-20 | 📌 | T09 drill, worker-driven: bar captured at 1×, 2×, 3×, 2× widescreen and fullscreen 1920×1080; STOP, ⌘., scale, widescreen and resize while recording driven. Nothing clipped or overlapping; picture unchanged. |
| 2026-10-20 | 📌 | T09: winit's `set_min_inner_size` resizes at once: it grows a window below the new minimum, and puts a window entering fullscreen back to its windowed size. Set the minimum only when the bar comes or goes. |
| 2026-10-20 | 🐞 | No task: `menus.rs` `os_layer_scale_and_fullscreen…` and `window.rs` `fullscreen_on_and_off…` fail on the base branch too (3/3 runs, 1920×1080 display): the window never goes fullscreen within their waits. |
| 2026-10-20 | 🔄 | Owner (T09): the bar's time is the recording's length (frames at the PAL rate), so it stands still while paused or a question is open. |
| 2026-10-20 | 📌 | T10 review: owner chose to close T10 with the 8 `recorder_save_window` OS tests unrun (screen locked); T11 must run them. Native close-button Quit, sheet keys and rfd Save…/Discard mapping are read-only. |
| 2026-10-20 | 📌 | T10: screen locked (`CGSSessionScreenIsLocked` true) hides every app window from the accessibility API; `recorder_save_window` OS tests (and `controls`) find no buttons. Not run; T11 reruns them on an unlocked screen. |
| 2026-10-20 | 📌 | T08: OS-layer `recorder_start_level` passed 4/4, then quit-discards and busy-app retry changed. Review rerun failed 4/4 on a locked screen (`IOConsoleLocked`); owner deferred the rerun to T11's drill. |
| 2026-10-20 | 📌 | T08 OS layer: the modal Start Level window sits at window layer 8, so `os_windows` misses it (tests look for its buttons); `AXError -25204` while a started level runs unseen; a locked screen or another worker's OS tests fail every run. |
| 2026-10-20 | 🐞 | T05 review: `--strict bug compare` (and `--strict run` of any `Setup` case) differs at frame 0 on `playfield_clean`, T03's known start residue, so strict names no engine bug. Open; T12's procedure relies on `--strict`. |
| 2026-10-20 | 📌 | T05: on today's engine W1-1 exit (forced) → tally → slot `slot-gamble` → W1-2 + 500 compares same, 33 legs; chain same. W4-3 `complete-w4-3` leaves the recording at frame 399 (`frog_hearts`), all 352 legs same. |
| 2026-10-20 | 📌 | T05: the runner re-syncs the engine after every lag (`resync = true`, RE §2.1), so a difference made inside or just before a lag goes unseen (W1-1 lags 155–275, 297–369). `bug compare` prints the re-sync count; T12's procedure should mention it. |
| 2026-10-20 | 📌 | T07 review: `--replay` in a home with widescreen on captures 464×261 frames, so its last frame never equals the 4:3 `screenshot.png`; the e2e runs a fresh 4:3 home. Left as is. |
| 2026-10-20 | 📌 | T07 review: fake unverified against real: `ScriptedUi::notice` for a bad `--replay` zip; the native notice is modal and waits for OK, and unscripted without `--capture` there is no timeout. |
| 2026-10-20 | 📌 | T07: sf-app's `recorder::read_zip` duplicates `sf_conform::bug::read_zip` (no PNG check); a shared one could live in `sf_core::recording`. `--replay` keeps the single-instance lock, so an agent replaying beside the owner's app needs `--home`. |
| 2026-10-20 | 📌 | T06 review: fake unverified against real: `ScriptedUi::copy` logs `clipboard:`; the native NSPasteboard `copy` is unreachable until T10's save window, so T10's OS test must read the clipboard back. |
| 2026-10-20 | 📌 | T06: no played route reaches W1-1's exit from `Game::start_level` unpaced: `complete-w1-1` (the original's pace) ends in game over at step 3201 for any seed and offsets −4..+4. T05 and T11 need one; T06's test forces the exit (`N: exit`). |
| 2026-10-20 | 📌 | T06: a dialog-started game keeps `from_dialog` through a new game begun from the title after its game over, so it saves no progress until Reset, Continue, Import Disks or Play Level File. Load State now takes the slot's own mark (T06 review). |
| 2026-10-20 | 📌 | T06: `menus.rs`'s OS-layer test lists stale File and View items (Export Level, Play Level File, CRT Filter, Widescreen missing); T06 updated only the Game menu's. |
| 2026-10-20 | 📌 | T04: `bug replay` draws the 4:3 view. T06 review: in widescreen the app now saves `render(View::default())`, since `narrowed` differs in the status rows; a widescreen STOP on a mid-pass frame can still differ. |
| 2026-10-20 | 📌 | T03 review: a `Setup` case records `Start::Transplant` as `Recorded.start` (lib.rs), so `channels.rs`'s channel check replays it from the original's memory, not `start_level`. Open; no `Start` variant carries a setup. T05 should know. |
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
