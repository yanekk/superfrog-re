# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts.** Over 60 rows or 15 KB: merge, drop what code or DESIGN now enforces
(say where), never drop a ✅ row or a greppable term.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-23 | 🐞 | T14 review: original mode's parting bookkeeping (accept-resume, held frames) was reached only by `#[ignore]`d whole-level runs; now `oracle::kept`, unit-tested. W2-1 original mode passed §4.1 in 49 s, 2 repairs, load ~3. |
| 2026-10-23 | 📌 | T14 review: `shorten::better` keeps fewer hearts before shorter (§2.5, M6); DESIGN §2.12 still reads "keeps the shortest exit". Wording left for the owner. |
| 2026-10-23 | 📌 | T14 split at pir's context limit: kept the planner fixes (commits 4d558bf–21ca306); T24–T28 carry the runs and the open items. T14's uncommitted `bench/tuning/results.jsonl` was discarded; its numbers are in these rows. |
| 2026-10-23 | 🐞 | T14 set A guided failures: coins met, stalled near the exit (W1-4 15 cells, W2-3 5–7, W3-4 13); Heart partings at a new cell each repair, never accepted (W3-2; W4-2 lost 5 hearts); W4-4's early parting re-plans the whole level. |
| 2026-10-23 | 📌 | T14 set A guided, load 8–27, bbcb4dd–2bcd9f4, frames (ref): W1-1 3527 (4474), W1-2 4659 (5183), W1-3 5745 (7729), W2-1 2269 (2524), W2-2 4922 (4739), W3-1 4558 (5492). Also done: W3-3, W4-1, W4-3, W5-1, W5-2, W5-4. Nine failed; W6-2–W6-4 not run. |
| 2026-10-23 | 🔄 | Owner, T14: the lockstep parts past the faithful check's lag bar, looked up from the level's completion case (`lag-limit`, else 100), never passed by `sf-bench` (DESIGN §2.6). W5-3's route exited but failed `verify`: 101 mid-pass frames from 5 310. |
| 2026-10-23 | 📌 | T14: rejoining the old plan (a bridge to its key 96–384 frames on) failed its W1-1 test: bridges in 1–2 rounds, no joined plan exits; dropped. W3-4 rounds to exit: pace 1 380, none 1 097, unguided 1 277. |
| 2026-10-22 | 🐞 | T14: W1-2's scroller freezes the original's frog nine frames from 3 954 while the engine runs on: 18 repairs, 25 min. A frame with the original's frog block unchanged is held; holding every mid-pass frame parted routes in 10 frames. |
| 2026-10-22 | 🔄 | Owner, T14: guide pace on by default (DESIGN §2.11). Engine-only W1-2, W2-1, W1-3, seeds 1–3, frames in all: none 45 850, against the archive's fastest 42 117, against the route's time 37 661 but W1-4 no exit in 30 min. |
| 2026-10-22 | 📌 | T23, load ~24: W1-4 original mode guided by `complete-w1-4` passed §4.1 in 629 s, 2 repairs, share 0.126, 1 heart, 10 062 frames; review rerun 388 s, same route, share 0.098. Engine only, guided: seeds 1, 2 exit in 237, 413 s; seed 3 none in 10 min. |
| 2026-10-22 | 📌 | T23 regressions after the guide change: W6-2 guided engine only 123 s, 8 982 frames (T09: 109–141 s); W5-2 unguided original mode 284 s, 7 repairs, 1 heart (unguided runs build no `GuideIndex`). |
| 2026-10-22 | 🔄 | Owner, T23: a guide point with coins known matches its band and every lower one (§2.11 said its band). W1-4 took coins out of route order; its guide index froze at p712 of 921 and the exit pull found 158,45. |
| 2026-10-22 | 🐞 | T11: `sf-bench` `judge` marks any wall-clock past the budget `over-budget`, the new planner included; `plan_original` runs its last lockstep past the deadline (W1-1 at 1 min overran 33 s). T14/T15 should check 30-min runs for this. |
| 2026-10-22 | 📌 | T11 Done-when, load 27, 10 threads: `sf-bench --planner new --levels W1-1` guided, original mode: completed, 594 s, 4 082 frames (ref 4 474), 2 repairs, 0 hearts. Engine-only defaults unguided: 6.5 min; guided, shortening off: 57 s. |
| 2026-10-22 | 📌 | T10 Done-when, 10 threads, §4.1 passed: W2-1 38 s, 4 repairs, share 0.41, 1 heart; W5-2 unguided 242 s, 5 repairs, share 0.13, 2 hearts. Review: 55 s, 301 s; W2-1 shortened 216 s, 1 repair, 3 651 → 3 583 frames. |
| 2026-10-22 | 📌 | T10 choices: repair stretches keep the shortening share, not its minimum. Heart and cells-apart partings are accepted (third at one cell) with the parting frame. A plan ending before the original's exit runs 50 more frames, last input held. |
| 2026-10-22 | 📌 | T10: `items-upper-w2-1` with enemies live never enters `sync::in_wall_loop`; the wall-loop test places the frog against W2-4's closed gate (x 303, y 912, facing left), as `wall_place` did. |
| 2026-10-22 | 📌 | T22, seed 1, 10 threads, load ~11, unguided, before → after shortening: W2-1 3 374 → 3 051, W5-2 5 701 → 5 405, W3-2 11 580 → 11 394 frames (committed 2 524, 6 994, 6 094). W3-2 stays 63% over M4. |
| 2026-10-22 | 📌 | T22: 2.4–3.3 rounds/s at load 11, 10 threads; `SHORTEN_MIN_ROUNDS` 180 is 60–75 s. Most shortcut hits fail to splice: the coarse key leaves the frog off where the route's rest expects it. |
| 2026-10-22 | 🔄 | Owner, T09: guide regions also match switches pressed when known; trace points are `(Cell, coins, switches)`, `trace_engine`/`trace_original` take `&Facts`. Without them W6-2 guided stalled at a closed gate, seeds 1–3, 5 min each; with them 109–141 s. |
| 2026-10-22 | 🐞 | T09: a leg snapshot's first frame, like `AtLevel`'s, has `level_state` ≠ −2, so an explorer started there reports an exit at frame 1. T11's `--snap` and `AtLevel` starts must play on until the frog lands past frame 150. |
| 2026-10-22 | 📌 | T09 Done-when: set-B W2-1 leg (`whole_game --until W2-2`, `leg-04.snap`), engine only, `follow_options` of `w2-1-complete`: exit in 40 rounds, 22 s at load ~20, route 2 235 frames, 1 heart lost. |
| 2026-10-22 | 📌 | T09, 10 threads, load ~12, seed 1: W6-2 guided by `enemies-w6-2-complete` exits in 138 s, 476 rounds, route 9 182 frames. W1-1 fixture with frame 2000 dropped, followed: 290 k engine frames, 28 rounds; unguided 849 k, 85. |
| 2026-10-22 | 📌 | T08 Done-when, load 9–12, 10 threads, 10-min cap, seed 1, unguided: W6-1 54 s, W5-2 37 s, W3-2 132 s, W3-4 245 s; cells 11.6k, 9.0k, 15.5k, 26.1k; peak RSS 3.1, 3.4, 3.1, 3.3 GB. W1-1 15 s. |
| 2026-10-22 | 🔄 | Owner, T08: kept DESIGN §2.5 heart rule; adopted `experiment/t08-coin-bands` 465d60d: last 8 coins one band each, pull toward nearest object coin left (deviation from §2.4). Before it W6-1 exited 0 of 5 ten-minute runs, stalling 2–4 coins short. |
| 2026-10-22 | 📌 | T08 checkpoint policy (D22): an entry over 64 frames past its checkpoint takes one; states dropped least-recently-used, non-anchor (1024-frame) and unreferenced first. A state counts 9/8 serialized (~620 KB); the archive keeps ¾ of `max_mem_bytes`. |
| 2026-10-22 | 📌 | T08: W1-1 with a 1 GB cap gave the same route as 4 GB, at 2.3× the engine frames (rebuilds), 105 s against 37 s. Counting the archive to the full 4 GiB reached 4.9 GB RSS on W5-2. |
| 2026-10-22 | 📌 | T08: `goexp` seeds each burst with the master generator's own stream (`Rng(sd\|1)`); with that and its end-on-heart-lost rule this explorer reproduced its W6-1 run frame for frame (7 248 frames, 12 743 cells). |
| 2026-10-22 | 🔄 | Owner amended the plan (D19–D24): whole-level explorer core; guide paths allowed everywhere, the counted benchmark included (baseline stays unaided); map and coin choice compasses only; M7, T13, T20 dropped; T22 route-shorten added. |
| 2026-10-22 | 📌 | Experiment `spike-explore/goexp.rs`, engine only, 10 threads, 10-min cap, no map, no hints: exits on W2-1 30 s, W2-4 180, W3-2 340, W3-4 577, W4-2 503, W4-4 589, W5-2 62, W6-1 211. W2-3, W6-2, W6-4 stalled 6–17 cells short. |
| 2026-10-22 | 📌 | Experiment: the band-0 exit pull turned W3-4, W4-4, W5-2, W6-1 from timeouts into exits; a committed route's path as guide made W6-2 exit in 80 s. Raw `--follow` inputs seed only 4–47 cells: original-timed routes die early on the engine. |
| 2026-10-22 | 🐞 | Experiment: W3-2's engine-only route parted from the original at frame 3 737 and died at 5 753, yet `sf_conform::run` said ok (resync follows the death); only `verify`'s exit milestone caught it. Stretch repair passed `verify` on W2-1, W5-2. |
| 2026-10-22 | 🐞 | Experiment: a stored `Game` is ~0.8 MB; 15–40 k-cell archives reached 5–8 GB RSS and the OS killed a 25-minute run. W5-2's original-mode route lost 3 hearts, its lockstep having no heart check. Hence D22 and §2.6's heart rule. |
| 2026-10-22 | 📌 | T21 map s, 10 threads, load 2–15: W1 147 185 156 229; W2 58 193 139 6; W3 79 39 180 109; W4 116 57 174 111; W5 83 192 189 294; W6 67 228 175 205. None over 300. |
| 2026-10-22 | 🐞 | T04 review, for T20: W2-3, W4-4 maps walk no seed with two switches pressed (`per_switch_set` {0,1,2,4} and {0,8}); far-from-gate slots ignore pressed switches, so a second lever is pressed only from world 0; each exit needs all four. |
| 2026-10-21 | 🔄 | Owner, T04: 11 maps over 5 min (quiet, W5-4 566 s) get T21 engine-memory-lookup (T04; blocks T20), speed only; builds still over 5 min after it are recorded and accepted, the move set never shrunk. |
| 2026-10-21 | 🐞 | T04, D13 check (`map_stats`): 11 of 24 maps miss the exit or the coins, no enemy help. Exit unreached: W2-3, W2-4, W3-2, W3-4, W4-2, W4-4, W5-2, W6-1, W6-2, W6-4. W2-1 reaches 24 of 35 coins. Owner asked before T08. |
| 2026-10-21 | 📌 | T04 map gaps found: W2-1's pad lands ~200 frames after its throw (cap now 300); W6 up held floats; W6-3's gate ledge needed a 32-frame jump; seeds beside patrolling enemies lose every move (W4-2 63-66,74). |
| 2026-10-21 | 🐞 | `sf-bench` copies a failed run's reason verbatim, so `bench/sample/RESULTS.md` and `results.jsonl` carry this machine's absolute worktree path (`no route (/Users/…/routes/explore-A-W1-1.txt…)`). Found in T02 review; T01 code, left. |
| 2026-10-21 | 📌 | T02 sample, set A, six levels, all old tools: median confirmation share 6.8 % over the 12 runs that reached the oracle (`plan_exit` 12.3 %, `plan_exit_w6` 2.4 %). Under 40 %: T10 keeps chunked 48-frame confirmation. Machine busy (T00 spike, ~8 cores). |
| 2026-10-21 | 🐞 | T02: old `explore` unaided gives up at frame 0 on all six sample levels in 12–43 s: its 4–48-frame bursts never outlast the ~144-frame level-start freeze, so the archive stays one cell. A 144-frame idle `--prefix` makes it move. |
| 2026-10-21 | 📌 | T02: old `plan_exit --coins 0` timed out on all six. W1-1, W1-4, W2-4, W3-1 run the level clock to 0 near frame 18 000–24 000, then backtrack forever: "every path loses a life on the original". W3-1 sees 0 coins in reach. |
| 2026-10-21 | 📌 | T02: old `plan_exit` on W6-2 oscillates over cells 223–228, 57–62 from frame ~22 500, coins needed stuck at 69. On W6-3 it took every coin, then stalled short of the exit at 61,66 (frame 25 536). |
| 2026-10-21 | 📌 | T02: old `plan_exit_w6` timed out on all six with only 3 744–5 760 route frames planned in 35 min (1.4–2.2 M engine frames), still collecting coins. Slow, no stall, hang or crash. |
| 2026-10-21 | 🐞 | `sf-bench` `best-old` with all three failed keeps the quickest failure (`explore`), so RESULTS.md's baseline shows M3 0.5 min and confirmation 0 %. Its `routes/` folder is not git-ignored; T02 left 1.4 MB of logs uncommitted. |
| 2026-10-21 | 📌 | T07: `play_safe` with all three retries, 40-frame move on W6-3: 0.21 s at load ~20 (expected 0.19). W1-1 pit deaths drop a life with `frog_hearts` unchanged, so `died` must be checked beside hearts. |
| 2026-10-21 | 🐞 | T03: tile coins (marker 55, `coin` h0+$ede0) add to `frog+$7e` only, never `coins_left_for_exit`; DESIGN §2.3 counts them toward the exit. `Facts` keeps them as `CoinKind::Tile`; T06 must meet `coins_needed` with `Object` coins. |
| 2026-10-21 | 📌 | T19, one thread, release, `Start::Level`, 2000 script frames, load 7–8: W1-4 1823 f/s drawn (`step`, the old `step_unseen` cost) to 11159 `step_unseen`; W6-3 1686 to 4736. The loop-ending frame is still drawn (`Game::shown`). |
| 2026-10-21 | 🔄 | Owner, T00: map moves landing within one cell of their target count as success, the search going on from where the frog is; D17's 3-failure ×4 rule stands. T04's move set adds mid-air steering and must reach W6-3's exit. |
| 2026-10-21 | 📌 | T00 Q5: frog parked 2000 frames, 1 of W1-4's 60 enemy marker cells ever fills a slot (W6-3: 2 of 46); walking the map fills 60 (W6-3: 17). Enemies spawn as the screen reaches them; D14 holds. |
| 2026-10-21 | 🐞 | T00 Q4: W6-3 walked, placed, or with 12–200-frame waits stays at 196–218 cells. Random bursts from its nodes find 47 more, route cells 119,5 and 160,7 among them, by steering mid-jump (RU, U, L). The move set lacks air steering. |
| 2026-10-21 | 📌 | T01's check run: `plan_exit --coins 0` on set A W1-1, unaided, stalled backtracking near frame 18 000 with 103 coins in reach; killed at 35 min, `timeout`. The harness recorded it and the table. |
| 2026-10-21 | 📌 | `whole_game --until W6-4` saves no W6-4 leg snapshot (a leg's snap is saved as it starts); `sf-bench` uses `--until WW-1`, into `~/.cache/superfrog-re/bench-legs/`, its `--out` there, never over `game-whole.txt`. |
| 2026-10-21 | 🔄 | Amendment review, owner: a wait-then-retry is a different, standing-start move from the stopped node, never the original edge (DESIGN §2.5). Hazards are marker 6 only (`frog_kill`). |
| 2026-10-21 | 🔄 | Owner amended the reviewed plan (D14–D17): danger judged live with wait retries, no parked-frog timetable; speed-band map nodes if T00's build fits; map-unreachable is a cost, search explores when stuck; edges carry coins touched, repeat failures marked unreliable. |
| 2026-10-20 | 📌 | `cases::render_w1_4` diverged at frame 86 (595 pixels) in a fresh-copy test run beside a busy machine, then passed alone and in a later full run. Treat as load-flaky; rerun before blaming a change. |
| 2026-10-20 | 📌 | Repo `.pir/settings.json` test line runs only the Python suite; this plan's block adds `cargo test --workspace` (rust-engine's line). Owner to fix the stored setting. |
| 2026-10-20 | 📌 | `Game::start_level` and `LevelSetup` exist only on `pir/bug-recorder` (game.rs:1028); `plan_exit --engine` on `pir/bug-recorder-T11` is the set-C precedent. |
