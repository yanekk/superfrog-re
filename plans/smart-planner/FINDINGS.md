# Findings log

**What the build taught.** Read the rows touching the task you pick up.

**Newest first. Forty words a row, counted.** The long version is in the commit message.

**Whoever appends, compacts.** Over 60 rows or 15 KB: merge, drop what code or DESIGN now enforces
(say where), never drop a ✅ row or a greppable term.

Legend: 🐞 defect found · ✅ verified by hand with the user · 📌 worth knowing · 🔄 a decision the user changed.

| Date | | Finding |
|---|---|---|
| 2026-10-25 | 🔄 | Owner, T15 review: set-C C3 (median 7.3 min, target 2; ~4 M engine frames a level, 3 threads) and C4 (W4-1 +51%, W1-4 +37%, W5-3 +29%, W6-1 +27%) accepted as misses. 22 of 24 replay in the real app. |
| 2026-10-25 | 🔄 | Owner, T15: set-C W3-4, W6-4 recorded misses: coins met, 45 and 48 cells short in 30 min, alone on 10 threads too. W6-4's near-exit stall now shows on sets A, B and C, W3-4's on A and C: one fix task across sets? |
| 2026-10-25 | 🔄 | Owner, T15 (D27): set-C `plan_exit --engine --seed 1` baseline stopped at 15 of 24; not run: W4-4, W5-1–W5-4, W6-1–W6-4. Of the 15: 5 done, 10 timed out (35 min). |
| 2026-10-25 | 📌 | T28 compass trial, unguided, seeds 1–3, 30 min (`bench/compass-trial.md`): each compass completed only W5-3 seeds 2–3 and W6-1; engine frames there straight 65.2, map 61.5, walk 40.2 M. Walk kept as `sf-plan` default; DESIGN §2.2 still says straight. |
| 2026-10-24 | 🐞 | T33: W4-1 unaided, seed 1, stretch 3's reach ended mid-fall over hazards 59–60,26 (`Goal::Reach` had no survival check). With it: W4-1 done 4.3 min, 3 979 frames; W1-3 8.2 min. Dead-end rewinds tested in review. |
| 2026-10-24 | 📌 | T27 set A unaided, seed 1, 0a66ec7–fc438f9: 9 of 24 done (W1-1, W1-2, W2-1, W2-4, W3-1, W4-3, W5-1, W5-2, W5-4; 6–25 min). No first exit: W2-3, W5-3, W6-2, W6-4. |
| 2026-10-24 | 🐞 | T27 unaided: W1-4, W3-2, W3-3, W3-4, W4-2, W4-4 first exit at 18–28 min, then budget spent confirming; W1-3, W4-1 dead end (T33). Their "cells archived 1" is the last rest-of-level plan. |
| 2026-10-24 | 🐞 | T27 unaided, not faithful (engine): W2-2 check 4 only, `frog_x` 3 px off from frame 334 (T32's kind); W6-1 `number_started` differs at 6 651. W6-3 route 18 s past budget, unverified. |
| 2026-10-24 | 🔄 | Owner, T27: `sf-bench` passed no `--threads`, so each `sf-plan` took all 10 cores, ~half each under `--jobs 2`. Now `cores / jobs` (`--threads` overrides); earlier rows marked in `RESULTS.md`. |
| 2026-10-24 | 🐞 | T32 (owner's T31 misses): set-B W2-2, W5-1 drifted alone in `passage_step`: the original's digs outlast a frame; the engine freed the frog a frame early, a move ahead. Digs now pass steps (`alone-pass-*`); both complete, 13.4, 14.8 min. |
| 2026-10-24 | 📌 | T31 (owner, T26): W2-2, W5-1, W6-3 leg snapshots end mid-pass (T209), so `verify::lead` syncs one frame on; their committed `chain-*` complete faithfully, the other 21 legs unchanged. Rerun e8f3a3d: W6-3 completed, 12.0 min. |
| 2026-10-24 | 🐞 | T31: `sf-plan --snap`'s settle ends mid-pass on set-B W3-1 and W4-2 (frame 151), so the planner's engine starts in a part-done pass; both completed in T26. W2-2, W5-1, W6-3 settle clean. Not fixed. |
| 2026-10-24 | 📌 | T26 set B guided, seed 1, 2ad88eb: 13 of 24 done (W1-1–W1-3, W2-1, W3-1, W3-4, W4-1–W4-3, W5-2–W5-4, W6-2; 5.5–28.5 min). Misses re-run alone, same result. W5-1 not run (T31). W6-2 passes here, unlike set A. |
| 2026-10-24 | 🔄 | Owner, T26: set-B W1-4, W6-1, W6-4 recorded misses, no first exit in 30 min, twice. W1-4 stalls at 156,45 (T23's dead end); W6-1 one coin never met; W6-4 coins met, 13 cells short, as set A. |
| 2026-10-24 | 🔄 | Owner, T26: set-B W2-3, W2-4, W3-2, W3-3, W4-4 recorded misses: a repair's reach misses in `REACH_ROUNDS`, the rest-of-level exit plan then spends the budget, run alone twice. Set A's W2-3, W3-4, W6-1 pattern (2026-10-23); owner to choose one fix task for both. |
| 2026-10-24 | 📌 | T29 rerun, 1e40096, load ~20, both complete §4.1: W2-4 24.4 min, 9 554 frames, 3 hearts; W6-2 10.5 min, 4 hearts. The owner's W2-4 budget miss (beside W2-3) was the first fix (6de6c5e), superseded by the slot-read point. |
| 2026-10-23 | 📌 | T30, d10dca3: `oracle::kept` lag repairs in a row alternate, `BACK_OFF` then before the run (route rewound). W1-4 done 18.7 min (load ~25), one lag parting; W5-2 done 10.3 min, rewound 954 → 870. Replaces T25 lag rows. |
| 2026-10-23 | 🔄 | Owner, T25: the W2-4, W6-2 sound misses get T29 engine-sound-ramp (T25; blocks T15), added; the W2-3, W3-4, W6-1, W6-4 budget misses are recorded as misses and the run carries on (W6-4 again in T28). |
| 2026-10-23 | 📌 | T25 set A guided, seed 1, run alone (load ~15, its own threads), 2a731dd/ad0dd6f: 18 of 24 done, M3 median 6.4 min; W1-4 9.2, W6-3 6.4 min. `plan-hints.log` has no entry lines; no level-id branch in `sf-plan`. |
| 2026-10-23 | 🔄 | Owner, T25 review: the old lag back-off (`BACK_OFF`) stays, W5-2 recorded a miss (set A 17/24); T30 lag-repair (T25; blocks T15), added, finds one rule finishing W1-4 and W5-2. |
| 2026-10-23 | 🐞 | T29: the T25 W2-4 (7 658), W6-2 (1 718) sound misses were `tile_anim_draw` (h0+$fa7c) freeing a queue slot before the lagging original's frame ended, so the blank's `take_tile` took a coin early. Slots pass steps, the read a point; `tile-queue-*` cases. |
| 2026-10-23 | 🐞 | T25 budget misses: W2-3, W3-4, W6-1 found a first exit (19, 11, 1 min), parted, failed the reach's 300 rounds, then the rest-of-level exit plan never finished. W6-4 found no exit, stalled 13 cells short. |
| 2026-10-23 | 📌 | T25 M4: W3-2 +58% keeps the hearts the committed route loses (a 3500-frame detour); W4-1 +37% goes back left for its last coins (the last-8-coins pull). Share 1.0, not 0.2, shortened each under 2%. Defaults kept. |
| 2026-10-23 | 📌 | T24 pattern 1: Heart partings come from the original's lag (W3-2: enemies drift, the frog matches). Repaired by reach stretches in 13–85 rounds, not accepted: W4-2 no route → 7 491 frames, 2 hearts, 1 182 rounds. No owner trade asked. |
| 2026-10-23 | 📌 | T24, guided, seed 1, 5 threads, load ~30, before → after, §4.1 passed: W1-4 no route → 7 998 frames, 1 heart, 2 044 rounds; W3-2 none → 11 066, 0, 1 245; W4-4 none → 13 197, 3, 3 723 (60-min budget). |
| 2026-10-23 | 🔄 | Owner, T24 review: kept the deviation from DESIGN §2.6 step 3, §2.7: a repair plans to the last whole plan's point 1024 frames on (`Goal::Reach`, `oracle::HORIZON`), the exit near its end or after `REACH_ROUNDS`. Reach stretches unshortened (M4, T25). |
| 2026-10-23 | 🔄 | Owner, T14 review: no guide pace in app mode (DESIGN §2.11). W1-1 `--app`, shorten 0, seeds 1–3, frames/hearts: pace 4452/1, 4158/0, 3404/0; none 2847/0, 3117/0, 2959/1. Pace failed T12's M8 test. |
| 2026-10-23 | 🐞 | T14 review: original mode's parting bookkeeping (accept-resume, held frames) was reached only by `#[ignore]`d whole-level runs; now `oracle::kept`, unit-tested. W2-1 original mode passed §4.1 in 49 s, 2 repairs, load ~3. |
| 2026-10-23 | 📌 | T14 review: `shorten::better` keeps fewer hearts before shorter (§2.5, M6); DESIGN §2.12 still reads "keeps the shortest exit". Wording left for the owner. |
| 2026-10-23 | 🔄 | Owner, T14: the lockstep parts past the faithful check's lag bar, looked up from the level's completion case (`lag-limit`, else 100), never passed by `sf-bench` (DESIGN §2.6). W5-3's route exited but failed `verify`: 101 mid-pass frames from 5 310. |
| 2026-10-22 | 🔄 | Owner, T14: guide pace on by default (DESIGN §2.11). Engine-only W1-2, W2-1, W1-3, seeds 1–3, frames in all: none 45 850, against the archive's fastest 42 117, against the route's time 37 661 but W1-4 no exit in 30 min. |
| 2026-10-22 | 📌 | T23, load ~24: W1-4 original mode guided by `complete-w1-4` passed §4.1 in 629 s (review 388 s), 2 repairs, 10 062 frames; engine only seeds 1, 2 exit 237, 413 s. Regressions: W6-2 guided 123 s, W5-2 unguided 284 s. |
| 2026-10-22 | 🔄 | Owner, T23: a guide point with coins known matches its band and every lower one (§2.11 said its band). W1-4 took coins out of route order; its guide index froze at p712 of 921 and the exit pull found 158,45. |
| 2026-10-22 | 📌 | T12, worker-driven: the real app (`1: start-level`, `seed N N`, key i at step i+1) keeps the engine's state hashes to W1-1's exit; a wrong seed parts at frame 1. `sf-bench --set C` W1-1 guided: completed, 2 936 frames (3.7 min; review, load 25: 9.0 min). |
| 2026-10-22 | 🐞 | T11: `sf-bench` `judge` marks any wall-clock past the budget `over-budget`, the new planner included; `plan_original` runs its last lockstep past the deadline (W1-1 at 1 min overran 33 s). T14/T15 should check 30-min runs for this. |
| 2026-10-22 | 📌 | T11 Done-when, load 27, 10 threads: `sf-bench --planner new --levels W1-1` guided, original mode: completed, 594 s, 4 082 frames (ref 4 474), 2 repairs, 0 hearts. Engine-only defaults unguided: 6.5 min; guided, shortening off: 57 s. |
| 2026-10-22 | 📌 | T10, 10 threads, §4.1 passed: W2-1 38 s, 4 repairs, 1 heart; W5-2 unguided 242 s, 2 hearts. Repairs keep the shortening share; heart and cells-apart partings accepted (third at one cell); short plans run 50 frames more. |
| 2026-10-22 | 🔄 | Owner, T09: guide regions also match switches pressed when known; trace points are `(Cell, coins, switches)`, `trace_engine`/`trace_original` take `&Facts`. Without them W6-2 guided stalled at a closed gate, seeds 1–3, 5 min each; with them 109–141 s. |
| 2026-10-22 | 📌 | T09, seed 1: set-B W2-1 leg (`leg-04.snap`) following `w2-1-complete` exits in 40 rounds, 22 s; W6-2 guided by `enemies-w6-2-complete` in 138 s, 476 rounds. W1-1 followed: 290 k engine frames; unguided 849 k. |
| 2026-10-22 | 📌 | T08 Done-when, load 9–12, 10 threads, 10-min cap, seed 1, unguided: W6-1 54 s, W5-2 37 s, W3-2 132 s, W3-4 245 s; cells 11.6k, 9.0k, 15.5k, 26.1k; peak RSS 3.1, 3.4, 3.1, 3.3 GB. W1-1 15 s. |
| 2026-10-22 | 🔄 | Owner, T08: kept DESIGN §2.5 heart rule; adopted `experiment/t08-coin-bands` 465d60d: last 8 coins one band each, pull toward nearest object coin left (deviation from §2.4). Before it W6-1 exited 0 of 5 ten-minute runs, stalling 2–4 coins short. |
| 2026-10-22 | 📌 | Memory: a stored `Game` is ~0.8 MB; 15–40 k-cell archives reached 5–8 GB RSS (OS kill), hence D22. T08: a 1 GB cap gives W1-1's 4 GB route at 2.3× engine frames; the full 4 GiB reached 4.9 GB RSS on W5-2. |
| 2026-10-22 | 🔄 | Owner amended the plan (D19–D24): whole-level explorer core; guide paths allowed everywhere, the counted benchmark included (baseline stays unaided); map and coin choice compasses only; M7, T13, T20 dropped; T22 route-shorten added. |
| 2026-10-22 | 📌 | Experiment `spike-explore/goexp.rs`, engine only, 10-min cap, no hints: exits on W2-1, W2-4, W3-2, W3-4, W4-2, W4-4, W5-2, W6-1 (30–589 s; band-0 exit pull needed on four); a route guide made W6-2 exit in 80 s. Raw `--follow` seeds 4–47 cells. |
| 2026-10-22 | 🐞 | Experiment: W3-2's engine-only route parted from the original at frame 3 737 and died at 5 753, yet `sf_conform::run` said ok (resync follows the death); only `verify`'s exit milestone caught it. Stretch repair passed `verify` on W2-1, W5-2. |
| 2026-10-22 | 🐞 | T04 review, for T20: W2-3, W4-4 maps walk no seed with two switches pressed (`per_switch_set` {0,1,2,4} and {0,8}); far-from-gate slots ignore pressed switches, so a second lever is pressed only from world 0; each exit needs all four. |
| 2026-10-21 | 🔄 | Owner, T04: 11 maps over 5 min (quiet, W5-4 566 s) get T21 engine-memory-lookup (T04; blocks T20), speed only; builds still over 5 min after it are recorded and accepted, the move set never shrunk. |
| 2026-10-21 | 🐞 | T04, D13 check (`map_stats`): 11 of 24 maps miss the exit or the coins, no enemy help. Exit unreached: W2-3, W2-4, W3-2, W3-4, W4-2, W4-4, W5-2, W6-1, W6-2, W6-4. W2-1 reaches 24 of 35 coins. Owner asked before T08. |
| 2026-10-21 | 🐞 | `sf-bench` copies a failed run's reason verbatim, so `bench/sample/RESULTS.md` and `results.jsonl` carry this machine's absolute worktree path (`no route (/Users/…/routes/explore-A-W1-1.txt…)`). Found in T02 review; T01 code, left. |
| 2026-10-21 | 📌 | T02 sample, set A, six levels, busy machine: old tools all timed out (`plan_exit --coins 0` ran the clock out; `plan_exit_w6` 3 744–5 760 frames in 35 min; `explore` stuck at frame 0, `--prefix` fixes). Median confirmation 6.8 %: T10 keeps 48-frame chunks. |
| 2026-10-21 | 🐞 | `sf-bench` `best-old` with all three failed keeps the quickest failure (`explore`), so RESULTS.md's baseline shows M3 0.5 min and confirmation 0 %. Its `routes/` folder is not git-ignored; T02 left 1.4 MB of logs uncommitted. |
| 2026-10-21 | 📌 | T07: `play_safe` with all three retries, 40-frame move on W6-3: 0.21 s at load ~20 (expected 0.19). W1-1 pit deaths drop a life with `frog_hearts` unchanged, so `died` must be checked beside hearts. |
| 2026-10-21 | 🐞 | T03: tile coins (marker 55, `coin` h0+$ede0) add to `frog+$7e` only, never `coins_left_for_exit`; DESIGN §2.3 counts them toward the exit. `Facts` keeps them as `CoinKind::Tile`; T06 must meet `coins_needed` with `Object` coins. |
| 2026-10-21 | 🔄 | Owner, T00: map moves landing within one cell of their target count as success, the search going on from where the frog is; D17's 3-failure ×4 rule stands. T04's move set adds mid-air steering and must reach W6-3's exit. |
| 2026-10-21 | 🔄 | Amendment review, owner: a wait-then-retry is a different, standing-start move from the stopped node, never the original edge (DESIGN §2.5). Hazards are marker 6 only (`frog_kill`). |
| 2026-10-20 | 📌 | `cases::render_w1_4` diverged at frame 86 (595 pixels) in a fresh-copy test run beside a busy machine, then passed alone and in a later full run. Treat as load-flaky; rerun before blaming a change. |
| 2026-10-20 | 📌 | Repo `.pir/settings.json` test line runs only the Python suite; this plan's block adds `cargo test --workspace` (rust-engine's line). Owner to fix the stored setting. |
