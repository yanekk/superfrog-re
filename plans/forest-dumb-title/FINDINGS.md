# Findings log

What the build taught, newest first, forty words a row counted. Read the rows touching your task.
Whoever appends compacts first when the file passes 60 rows or 15 KB. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user
changed.

| Date | | Finding |
|---|---|---|
| 2026-11-20 | 📌 | Every new case file must be listed in `mount_level_1.rs` (`PASS_MOUNTED` or `DIFFER`) or `every_case_is_in_one_list` fails in smoke; T04 to T06 each grow the same array, a merge to resolve (T03). |
| 2026-11-20 | 📌 | **An `all` goal true at the start is met on frame one**: `title_cursor` 0, `mt_songpos` 0, `world_index` 0 for world 1. `after = F` (T03, owner's choice) counts it only after oracle frame F. |
| 2026-11-20 | 📌 | `cargo fmt -p fd-conform` rewrites about 17 files outside any task (the crate is not fmt-clean); found in T02 review, left alone. |
| 2026-11-20 | 📌 | KONIEC from the 4990 snapshot (Down 5000, Down 5030, fire 5060, `scripts/case-goals-koniec.txt`): the original quits in step 5073 to `$FF492E`, and the translated program exits with it, every channel equal (`tests/case_goals.rs`). |
| 2026-11-20 | 📌 | **Codes reach worlds 1, 3 and 5 only** (0556, 2424, 7531) plus 2400, world 1 with `cheat_flag`; worlds 2, 4 and 6 have none, so `title-code-wN` exists for N 1, 3, 5 (T01, docs/16). |
| 2026-11-20 | 🐞 | Esc on the title quits to AmigaDOS in the step it is pressed (`title_codes.rs`); docs/07 read it as dead. `title-esc` judges a quit, as `title-koniec` does. |
| 2026-11-20 | 📌 | `level2_handler` runs on keys at the title: `last_key_byte` follows them (T01). docs/15's "none taken" for level 2 there pressed no key. KODY and Esc need the machine to take it. |
| 2026-11-20 | 📌 | Esc in play gives up straight to game over; `game-over-*.txt` use it. High scores take a typed name, kept in memory only (the save routine is unreferenced), so `game-over-name` applies; no owner question. |
| 2026-11-20 | 📌 | A `LAYOUT` row needs a `State` field (fd-core `TitleVars`, carried unchanged). High-score names are three long rows each, `hiscore_name_N+$0/+$4/+$8`; a non-creature `+$` row reads as a plain variable (T01 review). |
| 2026-11-20 | 📌 | `cargo clippy -p fd-conform` fails before this plan: `while_immutable_condition` (deny) at `fd-conform/src/bug.rs` `while steps > 0`. Smoke runs no clippy, so nothing catches it. |
| 2026-11-20 | 📌 | `tools/smoke.sh` in a fresh worktree (plan review): setup clean; root, toolkit and forest-dumb ok, forest-dumb done at 1061 s from a cold build; killed at the 1200 s limit in games/superfrog. Warm runs are the 842 s figure. |
| 2026-11-20 | 📌 | `bug compare` judges only hand-engine level 1 recordings (`fd-conform/src/bug.rs` `compare`, retimed to the level sync point); translated recordings, any start, only replay. Left out of scope here. |
| 2026-11-20 | 📌 | Smoke's last pass took 842 s of its 900 (`smoke-times.tsv`, `a4eebadc`); three of the four runs before it failed or hit the limit. Only seconds-long tests go in smoke. |
| 2026-11-20 | 📌 | fd-app's Esc always quits and never reaches the game; the Amiga Esc is reached through a `[keys]` binding (`"key Esc" = ["KeyE"]`, `game_over.rs`). |
| 2026-11-20 | 📌 | `level-end.toml` already holds HELP ending W1-1 under `cheat_flag`, translated and hand; the cheat case proves only that the code sets the flag. |
| 2026-11-20 | 🔄 | Owner chose Start Level… worlds 1 to 6 plus a cheat box, set directly at `load_world`, against the brief's proposal of leaving it unchanged. |
