# Findings log

What the build taught, newest first, forty words a row counted. Read the rows touching your task.
Whoever appends compacts first when the file passes 60 rows or 15 KB. Never drop a ✅ row or its date.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user
changed.

| Date | | Finding |
|---|---|---|
| 2026-11-20 | 📌 | KONIEC from the 4990 snapshot (Down 5000, Down 5030, fire 5060, `scripts/case-goals-koniec.txt`): the original quits in step 5073 to `$FF492E`, and the translated program exits with it, every channel equal (`tests/case_goals.rs`). |
| 2026-11-20 | 📌 | `tools/smoke.sh` in a fresh worktree (plan review): setup clean; root, toolkit and forest-dumb ok, forest-dumb done at 1061 s from a cold build; killed at the 1200 s limit in games/superfrog. Warm runs are the 842 s figure. |
| 2026-11-20 | 📌 | `bug compare` judges only hand-engine level 1 recordings (`fd-conform/src/bug.rs` `compare`, retimed to the level sync point); translated recordings, any start, only replay. Left out of scope here. |
| 2026-11-20 | 📌 | Smoke's last pass took 842 s of its 900 (`smoke-times.tsv`, `a4eebadc`); three of the four runs before it failed or hit the limit. Only seconds-long tests go in smoke. |
| 2026-11-20 | 📌 | fd-app's Esc always quits and never reaches the game; the Amiga Esc is reached through a `[keys]` binding (`"key Esc" = ["KeyE"]`, `game_over.rs`). |
| 2026-11-20 | 📌 | `level-end.toml` already holds HELP ending W1-1 under `cheat_flag`, translated and hand; the cheat case proves only that the code sets the flag. |
| 2026-11-20 | 🔄 | Owner chose Start Level… worlds 1 to 6 plus a cheat box, set directly at `load_world`, against the brief's proposal of leaving it unchanged. |
