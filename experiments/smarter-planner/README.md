# Smarter planner — experiment notes (2026-10-21/22)

Not a plan. A side investigation off `pir/smart-planner-T04` (branch `experiment/smarter-planner`),
asked for by the owner because the smart-planner build was performing below expectations. Nothing
here changes the live run; whether to act on it is the owner's decision (plan change).

## What the run had shown

- T04's movement map (one seed state per cell, ~50 fixed macro moves, `core::map`) reaches the
  exit with the coins on 12/24 levels; T20 was added to close the other 12 by hand.
- T00: 46–55% of map edges land elsewhere when replayed live; W6-3 needed new move families.
- T02: old tools 0/6 on the sample (explore died in the 144-frame start freeze; plan_exit ran the
  level clock out).
- No session tried a whole-level archive search, or used committed routes except to diagnose.

## The prototype

`engine/crates/sf-plan/examples/goexp.rs` (~470 lines): whole-level Go-Explore on the engine.

- Archive key: (frog cell / 2, coins-left band — 0 its own band, then 4-wide —, switches pressed,
  wings). Entry keeps the best state (most hearts, then fewest frames) and its inputs.
- Selection weight: novelty (1/√(1+chosen) + ½/√(1+seen)) × exp(−0.8 × bands behind the best) ×,
  in band 0, an exit pull 4·exp(−d/12)+0.2 (d = Manhattan cells to the nearest exit cell).
- Exploration: 80 bursts a round (10 threads × 8), each 1–16 sticky holds (p 0.4 change) of 10
  joystick states, 4/8/16/32 frames. A burst ends on a life or heart lost. Results merged in pick
  order, so a run is a function of (level, seed, threads).
- `--guide <script>`: the script played once on the original from AtLevel; (cell/2 ±1, band) →
  furthest point index along it (every 8 frames). Weight ×3·exp(−(max−p)/20) on the path, ×0.15 off.
- `--follow <script>`: replay inputs from the start, seeding every 8th state (weak: committed routes
  are original-timed and die early on the engine; 4–47 cells seeded).
- `--original 1`: plan on the engine from `Start::Transplant` of the oracle's memory; play the plan
  on the oracle in lockstep with the engine until frog cells differ by >1 or it dies; keep agreed−16
  frames; re-plan from there with the rest of the old plan as `--follow` teacher.
- `--polish S`: keep exploring S seconds after the first exit, keep the shortest (not yet measured).

Helpers: `sf-bench/examples/verify_route.rs` (the benchmark's `verify::verify` verdict on one set-A
route file), `sf-bench/examples/part_ways.rs` (first frame original and engine frogs differ).

## Results (quiet machine, 10 threads, engine-only unless said)

Levels the T04 map cannot finish, 10-min cap:

| Level | Unguided | Notes |
|---|---|---|
| W2-1 | exit 30 s, 2 503 f | map: 24/35 coins |
| W2-3 | timeout (17 cells short of exit); guided timeout | four switches; a 25-min run stayed 17 short from 350 s until the OS reaped it for memory |
| W2-4 | exit 180 s, 9 423 f | map stuck in start chamber |
| W3-2 | exit 340 s, 10 962 f | map 28/99 coins |
| W3-4 | exit 577 s, 11 924 f | needed the exit pull |
| W4-2 | exit 503 s, 7 357 f | map: updraft seeds among enemies |
| W4-4 | exit 589 s, 11 934 f | needed the exit pull |
| W5-2 | exit 62 s, 7 578 f | needed the exit pull |
| W6-1 | exit 211 s, 7 399 f | needed the exit pull |
| W6-2 | timeout (6 cells short); **guided exit 80 s, 9 574 f** | two switches in order |
| W6-4 | timeout (13 cells short); guided timeout | spikes crossed by flight |

Original mode (`--original 1`), judged by `verify_route` (the benchmark's verdict):

| Level | Wall | Repairs | Route vs committed | Hearts lost |
|---|---|---|---|---|
| W2-1 | 85 s | 1 | 3 104 vs 2 524 (+23%) | 0 |
| W5-2 | 482 s | 8 | 7 225 vs 6 094 (+19%) | 3 |

An engine-only route does not survive the original as is: W3-2's parts at frame 3 737 of 10 962
and dies at 5 753 (original lag). `sf_conform::run` still said "ok" on it, because with resync the
engine follows the original's death; only `verify` (exit milestone) catches it.

## Known gaps in the prototype

- Memory: 0.8 MB a `Game`; 15–40 k cells = 5–8 GB RSS. Store inputs plus sparse checkpoints, or
  evict states of cells chosen often with nothing new.
- Hearts: a burst stops on a heart lost, but the teacher replay and the oracle lockstep do not, so
  W5-2 lost 3. Reject plans whose oracle replay loses a heart, or add the heart cost to the key.
- Route length: +19–53% over committed. `--polish`, or a shortcut pass (for route states i<j,
  search from i for j's key with a frame cap) is untried.
- Exit pull is crow-flies; a map distance (T04's `MoveMap`, where it exists) would be the natural
  heuristic, which is a role for the map that does not need it to be complete.
- Thread use: ~27–30 k engine frames/s on 10 threads vs ~11 k/s single-thread `step_unseen` (T19):
  bursts are short and the merge is serial. Bigger rounds would help.
