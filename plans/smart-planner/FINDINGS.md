# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts.** Over 60 rows or 15 KB: merge, drop what code or DESIGN now enforces
(say where), never drop a ✅ row or a greppable term.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-21 | 📌 | T00 Q7: 500 map edges played from real states, enemies live, 46.2% fail on W1-4 (41% land elsewhere, 5% lose a heart), 55.0% on W6-3. 57% of misses (W6-3 44%) are one cell off. 17% of W1-4 edges take coins. |
| 2026-10-21 | 📌 | T00 Q6, speed bands: W1-4 12262 nodes, 279388 edges, 1699 s against cells-only 2414, 49519, 227 s; W6-3 237 s against 28 s, reaching 397 cells against 196. 56% (W6-3 53%) of moving-start edges land elsewhere than stopped. |
| 2026-10-21 | 📌 | T00 Q5: frog parked 2000 frames, 1 of W1-4's 60 enemy marker cells ever fills a slot (W6-3: 2 of 46); walking the map fills 60 (W6-3: 17). Enemies spawn as the screen reaches them; D14 holds. |
| 2026-10-21 | 🐞 | T00 Q4: W6-3 walked, placed, or with 12–200-frame waits stays at 196–218 cells. Random bursts from its nodes find 47 more, route cells 119,5 and 160,7 among them, by steering mid-jump (RU, U, L). The move set lacks air steering. |
| 2026-10-21 | 📌 | T00 Q4: all 44 W1-4 stall coins (sky, cave) are on the map, 38 needing wings (jump, then repeated fire), each replayed. W6-3: 71 of 79 above-air coins and the exit unreachable; the map is wrong there. |
| 2026-10-21 | 📌 | T00 Q3, 10 threads, busy machine (load 10–34): W1-4 cells-only map 738 s with the picture, 227–453 s without; W6-3 38 s and 12 s. Over ~5 min on today's engine. Placed-frog map: 89 s. |
| 2026-10-21 | 🐞 | T00 Q3: `Scene::step` draws `render::level_as_drawn` every level frame whatever `step_unseen`'s flag: 72% of its time (`panel::fetch` 45%). A local patch skipping it left the state identical, 3.2× fewer CPU-seconds a map. `spike/q3-profile.txt`. |
| 2026-10-21 | 📌 | T00 Q2: enemy markers cleared at entry give W1-4 51 more cells, 26% more edges, half the build time, but fail live more (58.8% against 46.2%; W6-3 67.6 against 55.0). Markers 44, 62 are tiles; platforms are passive objects, kept. |
| 2026-10-21 | 📌 | T00 Q1: the frog placed by writing X/Y covers 873 W1-4 cells in 89 s, but 52% of its edges (W6-3 54%) land elsewhere replayed from a real state. The walked map covers 1447. Seed by real moves. `spike/*-q1-seeding.txt`. |
| 2026-10-21 | 🔄 | Amendment review, owner: a wait-then-retry is a different, standing-start move from the stopped node, never the original edge (DESIGN §2.5). Hazards are marker 6 only (`frog_kill`). |
| 2026-10-21 | 🔄 | Owner amended the reviewed plan (D14–D17): danger judged live with wait retries, no parked-frog timetable; speed-band map nodes if T00's build fits; map-unreachable is a cost, search explores when stuck; edges carry coins touched, repeat failures marked unreliable. |
| 2026-10-20 | 📌 | Planning probe, quiet machine: `step_unseen` 1 386 frames/s W1-4, 1 251 W6-3, one thread; `Game::clone` 30–40 µs; oracle ~570. Engine only ~2.3× the oracle; T00 profiles why. A busy machine halves both. |
| 2026-10-20 | 📌 | `cases::render_w1_4` diverged at frame 86 (595 pixels) in a fresh-copy test run beside a busy machine, then passed alone and in a later full run. Treat as load-flaky; rerun before blaming a change. |
| 2026-10-20 | 📌 | Repo `.pir/settings.json` test line runs only the Python suite; this plan's block adds `cargo test --workspace` (rust-engine's line). Owner to fix the stored setting. |
| 2026-10-20 | 📌 | `Game::start_level` and `LevelSetup` exist only on `pir/bug-recorder` (game.rs:1028); `plan_exit --engine` on `pir/bug-recorder-T11` is the set-C precedent. |
