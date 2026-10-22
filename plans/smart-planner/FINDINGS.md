# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts.** Over 60 rows or 15 KB: merge, drop what code or DESIGN now enforces
(say where), never drop a ✅ row or a greppable term.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-22 | 📌 | T22, seed 1, 10 threads, load ~11, unguided, before → after shortening: W2-1 3 374 → 3 051, W5-2 5 701 → 5 405, W3-2 11 580 → 11 394 frames (committed 2 524, 6 994, 6 094). W3-2 stays 63% over M4. |
| 2026-10-22 | 📌 | T22: 2.4–3.3 rounds/s at load 11, 10 threads; `SHORTEN_MIN_ROUNDS` 180 is 60–75 s. Most shortcut hits fail to splice: the coarse key leaves the frog off where the route's rest expects it. |
| 2026-10-22 | 📌 | T08 review: DESIGN §2.4 still states the band as 1 + (left − 1) / 4; the owner's T08 rule (last 8 coins one band each, `archive::band`) is what the code and T09's guide regions use. |
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
| 2026-10-22 | 📌 | T21 map frames (k), same build: W1 6420 9135 8321 10652; W2 2501 9883 6613 262; W3 3004 1572 7472 5393; W4 5824 2060 6871 4780; W5 3254 7814 8379 12781; W6 2187 9269 7690 9185. |
| 2026-10-22 | 📌 | T05 review, for T20/T21: a new core module `map.rs`, `facts.rs`, `graph.rs` or `moves.rs` uses must join `cache::CORE_FILES`, or cached maps go stale; `tests/cache.rs` fails naming it. `cargo fmt -p sf-plan` rewrites seven files already unformatted. |
| 2026-10-22 | 🐞 | T04 review, for T20: W2-3, W4-4 maps walk no seed with two switches pressed (`per_switch_set` {0,1,2,4} and {0,8}); far-from-gate slots ignore pressed switches, so a second lever is pressed only from world 0; each exit needs all four. |
| 2026-10-21 | 🔄 | Owner, T04: 11 maps over 5 min (quiet, W5-4 566 s) get T21 engine-memory-lookup (T04; blocks T20), speed only; builds still over 5 min after it are recorded and accepted, the move set never shrunk. |
| 2026-10-21 | 📌 | T04 profile, one-thread map build: `layout::index_of` (a binary search per `State` byte access) is ~35% of busy samples, `State::byte`/`Bus::set_b` most of the rest; the map code is negligible. 1 thread 6.2 k frames/s, 10 threads 25 k. |
| 2026-10-21 | 🔄 | Owner, T04: the 11 levels the map cannot finish go to a new task, T20 map-gaps (T04; blocks T08), fixed before the search is built rather than left for it. |
| 2026-10-21 | 🐞 | T04, D13 check (`map_stats`): 11 of 24 maps miss the exit or the coins, no enemy help. Exit unreached: W2-3, W2-4, W3-2, W3-4, W4-2, W4-4, W5-2, W6-1, W6-2, W6-4. W2-1 reaches 24 of 35 coins. Owner asked before T08. |
| 2026-10-21 | 📌 | T04 build s, 10 threads, load ~25: W1 301 355 299 441; W2 111 375 268 11; W3 151 75 346 210; W4 223 110 331 215; W5 158 371 363 568; W6 127 438 374 394. 0.3–12.8 M frames. |
| 2026-10-21 | 📌 | T04 map gaps found: W2-1's pad lands ~200 frames after its throw (cap now 300); W6 up held floats; W6-3's gate ledge needed a 32-frame jump; seeds beside patrolling enemies lose every move (W4-2 63-66,74). |
| 2026-10-21 | 📌 | T04 switch sets: walking each set fully cost W2-2 36.8 M frames and W2-3 (four switches) over an hour; now only nodes within 10 cells of a gate are walked per set, the press written in from a real one. |
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
| 2026-10-21 | 🔄 | Owner, T00: added T19 step-unseen-no-picture (T00; blocks T04): `step_unseen` stops drawing `level_as_drawn`, about 3× faster map builds, state unchanged. Chosen over a coarser move set or a lazily built map. |
| 2026-10-21 | 📌 | T00 Q7: 500 map edges played from real states, enemies live, 46.2% fail on W1-4 (41% land elsewhere, 5% lose a heart), 55.0% on W6-3. 57% of misses (W6-3 44%) are one cell off. 17% of W1-4 edges take coins. |
| 2026-10-21 | 📌 | T00 Q6, speed bands: W1-4 12262 nodes, 279388 edges, 1699 s against cells-only 2414, 49519, 227 s; W6-3 237 s against 28 s, reaching 397 cells against 196. 56% (W6-3 53%) of moving-start edges land elsewhere than stopped. |
| 2026-10-21 | 📌 | T00 Q5: frog parked 2000 frames, 1 of W1-4's 60 enemy marker cells ever fills a slot (W6-3: 2 of 46); walking the map fills 60 (W6-3: 17). Enemies spawn as the screen reaches them; D14 holds. |
| 2026-10-21 | 🐞 | T00 Q4: W6-3 walked, placed, or with 12–200-frame waits stays at 196–218 cells. Random bursts from its nodes find 47 more, route cells 119,5 and 160,7 among them, by steering mid-jump (RU, U, L). The move set lacks air steering. |
| 2026-10-21 | 📌 | T00 Q4: all 44 W1-4 stall coins (sky, cave) are on the map, 38 needing wings (jump, then repeated fire), each replayed. W6-3: 71 of 79 above-air coins and the exit unreachable; the map is wrong there. |
| 2026-10-21 | 📌 | T00 Q3, 10 threads, busy machine (load 10–34): W1-4 cells-only map 738 s with the picture, 227–453 s without; W6-3 38 s and 12 s. Over ~5 min on today's engine. Placed-frog map: 89 s. |
| 2026-10-21 | 🐞 | T00 Q3: `Scene::step` draws `render::level_as_drawn` every level frame whatever `step_unseen`'s flag: 72% of its time (`panel::fetch` 45%). A local patch skipping it left the state identical, 3.2× fewer CPU-seconds a map. `spike/q3-profile.txt`. |
| 2026-10-21 | 📌 | T00 Q2: enemy markers cleared at entry give W1-4 51 more cells, 26% more edges, half the build time, but fail live more (58.8% against 46.2%; W6-3 67.6 against 55.0). Markers 44, 62 are tiles; platforms are passive objects, kept. |
| 2026-10-21 | 📌 | T00 Q1: the frog placed by writing X/Y covers 873 W1-4 cells in 89 s, but 52% of its edges (W6-3 54%) land elsewhere replayed from a real state. The walked map covers 1447. Seed by real moves. `spike/*-q1-seeding.txt`. |
| 2026-10-21 | 📌 | T01's check run: `plan_exit --coins 0` on set A W1-1, unaided, stalled backtracking near frame 18 000 with 103 coins in reach; killed at 35 min, `timeout`. The harness recorded it and the table. |
| 2026-10-21 | 📌 | `whole_game --until W6-4` saves no W6-4 leg snapshot (a leg's snap is saved as it starts); `sf-bench` uses `--until WW-1`, into `~/.cache/superfrog-re/bench-legs/`, its `--out` there, never over `game-whole.txt`. |
| 2026-10-21 | 🔄 | Amendment review, owner: a wait-then-retry is a different, standing-start move from the stopped node, never the original edge (DESIGN §2.5). Hazards are marker 6 only (`frog_kill`). |
| 2026-10-21 | 🔄 | Owner amended the reviewed plan (D14–D17): danger judged live with wait retries, no parked-frog timetable; speed-band map nodes if T00's build fits; map-unreachable is a cost, search explores when stuck; edges carry coins touched, repeat failures marked unreliable. |
| 2026-10-20 | 📌 | Planning probe, quiet machine: `step_unseen` 1 386 frames/s W1-4, 1 251 W6-3, one thread; `Game::clone` 30–40 µs; oracle ~570. Engine only ~2.3× the oracle; T00 profiles why. A busy machine halves both. |
| 2026-10-20 | 📌 | `cases::render_w1_4` diverged at frame 86 (595 pixels) in a fresh-copy test run beside a busy machine, then passed alone and in a later full run. Treat as load-flaky; rerun before blaming a change. |
| 2026-10-20 | 📌 | Repo `.pir/settings.json` test line runs only the Python suite; this plan's block adds `cargo test --workspace` (rust-engine's line). Owner to fix the stored setting. |
| 2026-10-20 | 📌 | `Game::start_level` and `LevelSetup` exist only on `pir/bug-recorder` (game.rs:1028); `plan_exit --engine` on `pir/bug-recorder-T11` is the set-C precedent. |
