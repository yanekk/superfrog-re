# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts** once this file is over 60 rows or 15 KB. Never drop a ✅ row or its
date, or a term somebody would grep for.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing ·
🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-28 | 📌 | T03: `sf-conform all` 216 cases green, run in two parts (a 2 h limit cut it after 213; the three `whole-*` cases run with `run`); `whole_game.rs` 7 green in 82 min; `story --ignored` 9 green; `channels` verdict byte-identical to base. |
| 2026-10-28 | 📌 | T03: `amiga_clock::policy` holds `whole_lines`, `exact_on_line` (`Cx::busy`), `wait_lines` (`Cx::blit_wait`), `spend_stolen` and `free_between` (slot draw `spend`, `done`, wrapped at 313), each swept against the old code kept in its tests. |
| 2026-10-28 | 📌 | T02 review: `amiga_clock::Timeline::run` yields at the fixed `FRAME_LINES - 1` and `word_times` wraps at `FRAME_LINES`, not at a `Timing`'s `frame_lines`; a game with another frame length needs them parameterised. As the task's interface specified. |
| 2026-10-28 | 📌 | T02: `alone` 5, `beam` 2, `frog_cases` 36, `story --ignored` 9 green; `channels` whole-game `Verdict` byte-identical to base `1249729` (still the `T216` label assert). Full test block not run (pir runs it at the end). |
| 2026-10-28 | 🐞 | T01 review: `channels` whole-game `Verdict` is byte-identical on base `1249729` and T01's head (`runs` empty); both fail the `T216` label assert. Pre-existing, open, not this plan's. |
| 2026-10-28 | 🐞 | `channels` `the_whole_games_channels_sound_as_the_original_does` fails on its known-label assert: it wants `T216`, `game-whole.toml`'s `task` says `none (accepted, awaiting the owner's confirmation)` since `dce1cdc`. Not T01's; `runs` empty. |
| 2026-10-28 | 📌 | T01: sf-conform `alone` 5, `beam` 2, `frog_cases` 36 green on base `1249729` and after T01; `story --ignored` 9 green after. Full test block not rerun (pir runs it at the end); planning's green on `874c78f` stands. |
| 2026-10-27 | 📌 | Planning: the stored setup and test lines, run in a fresh worktree on `874c78f`, are green: setup clean, Python ok, `cargo test -q --workspace` 145 suites ok in 1 h 45 min, near the 120-minute `testTimeoutMinutes`. |
| 2026-10-27 | 📌 | Planning: `story/sim.rs`'s HRM word table differs from `level::beam::slots_per_word` for channels 0x6 and 0xE (B and C without D: 3 against 4). Not one table; both stay (DESIGN §2.4). |
| 2026-10-27 | 📌 | Planning: `slot/draw.rs` `spend` and its line sum wrap lines at the front end's 313 (`crate::front::FRAME_LINES`), while the slot's `CUT_LINE` uses its own 312. Each keeps its length (DESIGN §2.2). |
| 2026-10-27 | 📌 | Planning: frame/line lengths `measure.sh` does not count: sf-paula `CLOCKS_PER_LINE`, `LINES_PER_FRAME`; story `LONG_FRAME`, intro `LINES`; `blit.rs` `l % 313`. T01 derives them from `amiga-clock` (DESIGN §2.2). |
| 2026-10-27 | 📌 | Planning: `level/timeline.rs` reads `front::Cx`, `Yield` and `game::Job`, and `word_times` takes `&copper::Fetch`; neither moves as it is. T02 splits them (DESIGN §2.4, §2.6). |
