Written by hand at T18 (DESIGN D5) and included here by `sf-bench`; edit `bench/verdict.md`, not this
file. The values are the counted run's (T17, seed 1, 30 min a level, a
busy machine, owner 2026-10-25).

How to read the numbers:

- The new planner's counted results are **guided**: each level's committed route is its guide path
  (DESIGN §2.11, D21). The baseline is **unaided**.
- The baseline is only T02's six-level set-A sample (D27); no counted baseline was run, and set B has
  none. It is in effect the best of two old tools: `explore` never started on any sample level (stuck
  at frame 0, "no route"), so `plan_exit` and `plan_exit_w6` carry it.
- A level every old tool failed shows the quickest failure's time (`best_of`), left as recorded (owner,
  2026-10-25). Set C's baseline is `plan_exit --engine` alone, stopped at 15 of 24 levels (D27).
- The unaided set-A results from the tuning run (T14/T27) follow below as recorded, not gated.

| # | Target | Measured (new, guided) | Verdict | Cause, and the owner's decision |
|---|---|---|---|---|
| M1 | 24 / 24 | 21 / 24 | missed | W6-4 meets every coin and stalls 48 cells short of the exit; W2-3, W3-4 fail a repair's reach, then the rest-of-level plan never meets the coins (9, 82 short). Owner, 2026-10-26: recorded as a miss; old tools kept, T34 decides. |
| M2 | 4 / 4, each under 30 min | 4 / 4 (W1-4 26.5, W2-4 15.5, W6-2 6.2, W6-3 5.8 min) | met | |
| M3 | median ≤ 10 min, slowest ≤ 30 min | median 7.9 min, slowest 30.3 min | missed (slowest) | Only failed levels reach 30 min; W3-4 overran by 18 s in its last lockstep past the deadline (T11 finding). Slowest completed level 26.5 min. Owner, 2026-10-26: accepted. |
| M4 | none over +25% | 4 over (W6-1 +56%, W3-2 +45%, W4-1 +40%, W1-4 +28%), 21 compared | missed | W3-2 detours to keep hearts the committed route loses; W4-1 goes back left for its last coins (last-8-coins pull); W1-4 takes coins out of route order; W6-1 not traced (4 hearts lost). Owner, 2026-10-26: accepted. |
| M5 | 24 / 24 | 18 / 24 | missed | Coins met, exit 10–48 cells away: W2-3, W3-2, W3-3, W6-4. Coins never met: W4-4 (40 short), W6-1 (2 short). The same two stalls as M1. Owner, 2026-10-26: recorded as a miss; T34 decides. |
| M6 | ≥ 20 / 24 | 10 / 24 | missed | A repair after a lag parting with the original accepts a lost heart and never tries to win it back (T10, T24); a planner gap, not unavoidable hits. Owner, 2026-10-26: accepted, to be fixed in the next plan (difficulty modes needs hitless routes). |
| M8 | always | a test (T08, T11, T12) | met | |
| M9 | 1 (+ `plan_bonus`) | 5 today: `sf-plan`, `plan_exit`, `plan_exit_w6`, `explore`, `complete` | not yet | The old tools stay until the owner accepts the M1 miss or plays the failing stretches (owner, 2026-10-26); T34 deletes them and rechecks M9. |
| C1 | 24 / 24 | 22 / 24 | missed | W3-4 and W6-4 meet the coins and stall 45 and 48 cells short. Owner, 2026-10-25 (T15): recorded misses; 2026-10-26: T34 decides with M1. |
| C3 | median ≤ 2 min, slowest ≤ 30 min | median 7.3 min, slowest 30.0 min | missed | About 4 M engine frames a level on 3 threads. Owner, 2026-10-25 (T15 review): accepted. |
| C4 | none over +25% | 4 over (W4-1 +51%, W1-4 +37%, W5-3 +29%, W6-1 +27%), 22 compared | missed | Owner, 2026-10-25 (T15 review): accepted. |

Against the baseline: on the six sample levels the old tools completed none, the new planner all six;
on set C `plan_exit` completed 5 of the 15 it ran (median 35 min, timeouts), the new planner 22 of 24.
