# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts.** Over 60 rows or 15 KB: merge, drop what code or DESIGN now enforces
(say where), never drop a ✅ row or a greppable term.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-21 | 📌 | T02 sample, set A, six levels, all old tools: median confirmation share 6.8 % over the 12 runs that reached the oracle (`plan_exit` 12.3 %, `plan_exit_w6` 2.4 %). Under 40 %: T10 keeps chunked 48-frame confirmation. Machine busy (T00 spike, ~8 cores). |
| 2026-10-21 | 🐞 | T02: old `explore` unaided gives up at frame 0 on all six sample levels in 12–43 s: its 4–48-frame bursts never outlast the ~144-frame level-start freeze, so the archive stays one cell. A 144-frame idle `--prefix` makes it move. |
| 2026-10-21 | 📌 | T02: old `plan_exit --coins 0` timed out on all six. W1-1, W1-4, W2-4, W3-1 run the level clock to 0 near frame 18 000–24 000, then backtrack forever: "every path loses a life on the original". W3-1 sees 0 coins in reach. |
| 2026-10-21 | 📌 | T02: old `plan_exit` on W6-2 oscillates over cells 223–228, 57–62 from frame ~22 500, coins needed stuck at 69. On W6-3 it took every coin, then stalled short of the exit at 61,66 (frame 25 536). |
| 2026-10-21 | 📌 | T02: old `plan_exit_w6` timed out on all six with only 3 744–5 760 route frames planned in 35 min (1.4–2.2 M engine frames), still collecting coins. Slow, no stall, hang or crash. |
| 2026-10-21 | 🐞 | `sf-bench` `best-old` with all three failed keeps the quickest failure (`explore`), so RESULTS.md's baseline shows M3 0.5 min and confirmation 0 %. Its `routes/` folder is not git-ignored; T02 left 1.4 MB of logs uncommitted. |
| 2026-10-21 | 📌 | T01's check run: `plan_exit --coins 0` on set A W1-1, unaided, stalled backtracking near frame 18 000 with 103 coins in reach; killed at 35 min, `timeout`. The harness recorded it and the table. |
| 2026-10-21 | 📌 | `whole_game --until W6-4` saves no W6-4 leg snapshot (a leg's snap is saved as it starts); `sf-bench` uses `--until WW-1`, into `~/.cache/superfrog-re/bench-legs/`, its `--out` there, never over `game-whole.txt`. |
| 2026-10-21 | 🔄 | Amendment review, owner: a wait-then-retry is a different, standing-start move from the stopped node, never the original edge (DESIGN §2.5). Hazards are marker 6 only (`frog_kill`). |
| 2026-10-21 | 🔄 | Owner amended the reviewed plan (D14–D17): danger judged live with wait retries, no parked-frog timetable; speed-band map nodes if T00's build fits; map-unreachable is a cost, search explores when stuck; edges carry coins touched, repeat failures marked unreliable. |
| 2026-10-20 | 📌 | Planning probe, quiet machine: `step_unseen` 1 386 frames/s W1-4, 1 251 W6-3, one thread; `Game::clone` 30–40 µs; oracle ~570. Engine only ~2.3× the oracle; T00 profiles why. A busy machine halves both. |
| 2026-10-20 | 📌 | `cases::render_w1_4` diverged at frame 86 (595 pixels) in a fresh-copy test run beside a busy machine, then passed alone and in a later full run. Treat as load-flaky; rerun before blaming a change. |
| 2026-10-20 | 📌 | Repo `.pir/settings.json` test line runs only the Python suite; this plan's block adds `cargo test --workspace` (rust-engine's line). Owner to fix the stored setting. |
| 2026-10-20 | 📌 | `Game::start_level` and `LevelSetup` exist only on `pir/bug-recorder` (game.rs:1028); `plan_exit --engine` on `pir/bug-recorder-T11` is the set-C precedent. |
