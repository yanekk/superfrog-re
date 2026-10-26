# Failing levels, to tackle after the smart-planner merge

Written at T34 on the owner's answer (2026-10-26): the M1, M5 and C1 misses stand as recorded, the
old tools are retired, and these seven levels are tackled again once this plan is merged. The
numbers are T17's counted run (sets A, B) and T15's (set C), guided, seed 1, 30 min a level, from
`results.jsonl`; the cause column is the planner's own reason line.

The planner's partial routes and logs from those runs (`bench/routes/new-<set>-<level>.{txt,log}`)
were scratch in the task worktrees and are gone. The failing route is the guide below plus the
planner's run; the rerun command regenerates it, partial route and log included.

Sets: A starts at the level's own start; B from the whole-game leg snapshot (`--snap`); C from the
app's Start Level on the engine with app timing (`--app`). Each run is guided by the level's
committed route in `engine/conform/scripts/` (set C uses set A's).

| Level | Set | Guide (committed route) | Commit | What the run reached |
|---|---|---|---|---|
| W2-3 | A | `w2-3-complete` | f27dde453224 | Coins never met, 9 short; the original confirmed to frame 7870 |
| W2-3 | B | `chain-w2-3` | 1c5aff3beb16 | Coins met, exit 10 cells away |
| W3-2 | B | `chain-w3-2` | ca9822e711ce | Coins met, exit 48 cells away |
| W3-3 | B | `chain-w3-3` | ca9822e711ce | Coins met, exit 10 cells away |
| W3-4 | A | `enemies-w3-4-complete` | f9eeec8f3611 | Coins never met, 82 short; the original confirmed to frame 2528 |
| W3-4 | C | `enemies-w3-4-complete` | eb8505567244 | Coins never met, 3 short |
| W4-4 | B | `chain-w4-4` | 4cf7089084bf | Coins never met, 40 short; the original confirmed to frame 8344 |
| W6-1 | B | `chain-w6-1` | 2c215c483a8e | Coins never met, 2 short; the original confirmed to frame 6807 |
| W6-4 | A | `enemies-w6-4-complete` | d60e543207a8 | Coins met, exit 48 cells away |
| W6-4 | B | `chain-w6-4` | 2c215c483a8e | Coins met, exit 37 cells away |
| W6-4 | C | `enemies-w6-4-complete` | eb8505567244 | Coins met, exit 48 cells away |

Every other set of these levels completed: W2-3 C; W3-2 A, C; W3-3 A, C; W3-4 B; W4-4 A, C;
W6-1 A, C.

## Two kinds of stall

- **Coins met, exit not reached** (W2-3 B, W3-2 B, W3-3 B, W6-4 on all three sets): the
  rest-of-level search never closes the last 10–48 cells. W6-4's is the same stall on every set; a
  44-second owner recording of its climb, spliced into the guide and run with the unmerged
  `sf-plan --guide-ordered` (branch `experiment/ghost-guide`), reached the exit on 2026-10-25.
- **Coins never met** (W2-3 A, W3-4 A and C, W4-4 B, W6-1 B): a repair's reach misses in
  `REACH_ROUNDS` after a parting with the original, then the rest-of-level plan spends the budget
  (FINDINGS 2026-10-23, 2026-10-24).

## Reruns after the merge

W3-4, 2026-10-26, a41f5707f44f, `sf-plan W3-4 --guide enemies-w3-4-complete --threads 5
--budget-min 30` run directly (not through `sf-bench`, so no `results.jsonl` line), two runs at a
time, load 37–45:

| Set | Seed | Result |
|---|---|---|
| A | 1 | Coins met, exit 29 cells away |
| A | 2 | Coins never met, 28 short; 6 repairs, the original confirmed to frame 11874 |
| C | 1 | Coins never met, 3 short; no stall spot, 77 coins left across the map |
| C | 2 | Completed in 17 min: 15 234 frames, 1 heart; `verify_route` in the real app passed. Kept as `routes/app/W3-4.txt` |

Set A again, guided by that set-C route (`--guide routes/app/W3-4.txt`; the guide is traced from
its app start), load 2–20:

| Set | Seed | Result |
|---|---|---|
| A | 1 | Coins met, exit 35 cells away; 2 repairs |
| A | 2 | Completed in 12 min: 13 796 frames, 1 heart, 10 repairs; `sf_bench::verify` set A (§4.1) passed. Kept as `routes/original/A/W3-4.txt` |

W6-4 set C, 2026-10-25, `sf-plan W6-4 --app --seed 3 --guide-ordered --guide rec-guide.txt`. The
guide is `tools/recording_guide.py bugs/bug-W6-4-2026-10-25-1851.zip`: the owner's 44-second
recorded climb after a `tools/handover.py` lead-in, 1 656 waypoints, cut at step 13218 where a life
was lost. No `--via`/`--avoid`. Real-app check 2026-10-26 at c3885591abe6, load 3:

| Set | Seed | Result |
|---|---|---|
| C | 3 | Completed: 15 084 frames, 1 heart; `verify_route W6-4 … 3` in the real app passed (`completed=true route_frames=Some(15084) hearts_lost=1`). Kept as `routes/app/W6-4.txt`; set C is 24/24 |

## Rerun

From `engine/`, one set at a time (a level recorded for the same commit is skipped):

```bash
cargo run --release -p sf-bench -- --set A --planner new --levels W2-3,W3-4,W6-4 --seed 1 --budget-min 30
cargo run --release -p sf-bench -- --set B --planner new --levels W2-3,W3-2,W3-3,W4-4,W6-1,W6-4 --seed 1 --budget-min 30
cargo run --release -p sf-bench -- --set C --planner new --levels W3-4,W6-4 --seed 1 --budget-min 30
```

Each writes `bench/routes/new-<set>-<level>.txt` (the best partial route, in its own timing) and
`.log` (the planner's stderr, its reason line last), and appends a line to `results.jsonl`.
