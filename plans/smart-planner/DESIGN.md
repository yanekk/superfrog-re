---
setup:
  - python3 -m venv venv && venv/bin/pip install -q --disable-pip-version-check -r requirements.txt
  - [ ! -x tools/setup/oracle.sh ] || tools/setup/oracle.sh
test:
  - NO_COLOR=1 venv/bin/python -m unittest discover -q -s . -p 'test_*.py' || [ $? -eq 5 ]
  - [ ! -f engine/Cargo.toml ] || (cd engine && CARGO_TERM_COLOR=never cargo test -q --workspace)
---

# Smart planner — Design

## 1. Purpose

An agent hands the planner a level, a starting state and the goal (the level's exit) and gets back an
input script that plays the level to its exit on the original, with no hints: no waypoints, no
per-level options, no choice between tools. Today four tools (`plan_exit`, `plan_exit_w6`, `explore`,
`plan_bonus` in `engine/crates/sf-conform/examples/`) each steer by hand-patched heuristics, and the
rust-engine run spent hours on W1-4, W2-4, W6-2 and W6-3 (plans/rust-engine/FINDINGS.md T149, T150,
T168, T207 rows). The root cause is that their distance field knows geometry and not movement: open
air under a coin reads as near. This plan replaces them with one planner steered by a measured
movement map, and proves it against a baseline of today's tools under identical rules. It is the tool
that later plans easy and hard completions (the difficulty-modes plan), so it is proven here by
numbers.

### Success criteria

The benchmark (§4) committed in `plans/smart-planner/bench/` shows, side by side with today's tools:

| # | Measure | Target |
|---|---|---|
| M1 | Set A levels completed unaided within budget, faithful on the original | 24 / 24 |
| M2 | W1-4, W2-4, W6-2, W6-3 completed unaided | 4 / 4, each under 30 min |
| M3 | Planning time per level, set A | median ≤ 10 min, slowest ≤ 30 min |
| M4 | Route length (frames to exit) vs the committed route, per level | none more than 25% longer |
| M5 | Set B legs completed unaided | 24 / 24, same budget |
| M6 | Set A routes losing no heart | ≥ 20 / 24; each short one explained (unavoidable hit or planner gap) |
| M7 | Standable cells a one-hour free explorer reaches that the map calls unreachable | 0 on every level |
| M8 | Same seed, same level, same start → byte-identical route | always (a test) |
| M9 | Tools an agent chooses between for level routing | 1 (`plan_bonus` stays for WB-1) |
| C1 | Set C levels completed on the engine with app timing | 24 / 24 |
| C3 | Set C planning time | median ≤ 2 min, slowest ≤ 30 min |
| C4 | Set C route length vs the committed route | none more than 25% longer |

Recorded, not gated: engine frames simulated per level (machine-independent, so later changes compare
fairly), oracle frames, the share of wall-clock spent checking on the original, and route repair on
set B (given the leg's standalone route).

### Stance

- Unaided is the measure. The planner plans from level, start and goal; anything level-specific it
  knows it measured itself. Hints exist as an escape hatch (§2.10), but every use is logged as a
  planner gap, and the benchmark never uses them, because a hint is a step an agent takes by hand on
  every one of the next 24 levels of easy and hard mode.
- The engine decides what a move does, never a formula. Movement is measured by stepping `sf-core`,
  so the map cannot disagree with the game about what a jump reaches.
- The original has the last word. A route in original mode counts only if the faithful check of
  rust-engine DESIGN §2.1 passes on it, as the completion cases do.
- The planner only reads the engine. No game mechanic, no faithful-bar tolerance changes here.

---

## 2. Behaviour specification

### 2.1 Inputs and output

The planner takes a level id, a start and options that are not level-specific: seed, mode
(original / app), budget, `--follow <script>`, `--out`, `--no-cache`, and the hints of §2.10. Starts:

- `AtLevel(id)`: the level's sync point (set A).
- `--snap <leg.snap>`: a `whole_game` leg snapshot, carrying earlier legs' state (set B).
- `--app`: `Game::start_level(display, &LevelSetup{level, ..Default::default()}, Rng::seeded(seed, seed))`,
  the start Game ▸ Start Level… makes under a script (set C; the API is on `pir/bug-recorder`, §7 D3).

Output is an `InputScript` in today's text format (`frame: inputs`, one line per change, closing
`N: -`), ending in the frame the level's `level_state` leaves −2 plus the same idle tail the committed
`complete-*` scripts carry, so it drops into a case file unchanged. Why: every consumer of routes
(cases, `whole_game`, the bug recorder) already reads that format.

The goal is the exit with the level's coin count met (`COINS_LEFT` = 0 as the exit reads it). Bosses
on x-4 levels are part of the route where the level requires one beaten before its exit opens; the
search treats a required kill as a goal the way today's `--stomp/--hurt` did, found from level facts
(§3.2 `facts`), not given per level.

### 2.2 The movement map

For each standable cell (open cell whose cell below is ground or a platform top, by the markers in
`level/collision.rs`), the map holds edges: a macro move (an input sequence of held joystick states),
its frame count, the cell it ends standing in, and its preconditions (a switch set that must be
pressed, wings held). Move families: walk left/right, jump, running jump of several run-up lengths,
drop off an edge, pad and spring launch (a move that starts on the pad), and with wings, flight
segments. Edges are found by stepping the engine (`Game::step_unseen`) from a real game state, never
computed from a jump formula, because the rust-engine run found the formula versions wrong in each
level family (FINDINGS T103, T115, T150).

Distance is the sum of edge frames, so the cost of a detour is measured in time, which is what the
budget and M4 count.

Switches and gates: a gate cell is solid until its switch pair is pressed (items/fixtures.rs
`switch()`). The map is built per switch set reachable from the start; an edge through a gate carries
the switch as a precondition. Why: W6-2's stall was a room behind two switches in order, which a
static map cannot represent.

How the map is seeded (frog placed by writing its position into a state, or only cells reached from
the start by real moves) and how enemies and moving platforms are held while measuring are decided by
T00 and recorded in §7 by that task's review. Until then the rule is: an edge is valid only if
replaying its inputs on the engine from its start state ends in its end cell, standing, alive (the
test in §4).

### 2.3 Coin choice

Before moving, the planner picks a set of coins whose count meets the exit's requirement and an order
for them, minimising total map distance from start through the coins to the exit. Unreachable coins
(no path in the map) are never chosen. Coins include tile coins from the world's pickup table
(h0+$f382 entries whose handler is h0+$ede0) and coin objects. Why: the exit needs a count, and
nearest-first walked into the hardest coins (W1-4, set-C W1-1 column 92). The choice is re-run when
the route parts from plan (a coin taken by accident, one found unreachable in play), from the state
the frog is in.

### 2.4 The search

The search follows the coin order along map paths by playing each edge's inputs on a live engine
state (enemies live). When an edge's result differs from the map (an enemy, a moving platform, carried
velocity), a frame-level beam search over held joystick states, scored by map distance to the next
target plus the danger cost (§2.5), finds a way back onto the map. Every state it reaches goes into one
archive for the whole level, keyed by (cell, coins left in bands, switches pressed); a later target
starts from the archive's best state for it rather than from scratch. Why: W2-4 rebuilt `explore`'s
archive 31 times, once per coin (FINDINGS T168, ROADMAP).

A path dies on a life lost (a 1-up gained is not a loss, FINDINGS T205 row on `plan.rs`), `DYING`,
an engine panic (caught), or the original hanging in `frog_tiles`'s wall loop (`sync::in_wall_loop`,
original mode).

Determinism: the search is a function of (level, start state, seed, options). Threads expand fixed
partitions and their results are merged in partition order, then stable-sorted. The wall-clock budget
only decides when to give up; it never chooses between routes. Why: M8, and so a benchmark run can be
reproduced on another machine to the byte, given it finishes.

### 2.5 Danger map

Enemy hitboxes are sampled over a window of frames with the frog parked out of reach, giving per cell
the frames an enemy occupies it. The search adds a cost for a move whose frames overlap occupied
cells and prefers waiting at a safe cell for a gap. A heart lost costs more than any detour the budget
allows. Why: M6, and hard mode later, where one touch kills.

### 2.6 Checking on the original

Original mode plans on the engine unpaced and confirms on the oracle, because the original lags in
busy passes and the engine does not (rust-engine FINDINGS T42, T84): the two drift apart. Each chunk
starts from the original's memory transplanted into the engine (`Start::Transplant`, as `plan_exit`
does). T02 measures what share of planning time confirmation costs. If it is large (T02 records the
threshold it used and why), T10 checks long stretches once and re-plans only from where the original
parts from the engine, by route repair (§2.7); otherwise it keeps 48-frame commits. Either way the
final route is accepted only by the faithful check (§4).

App mode (set C) confirms on the engine from the same `start_level` state; there is no original.

### 2.7 Route repair

`--follow <script>` replays an old route while the frog's cell stays within one cell of the old
route's cell at the same input index; where it parts, the search re-plans to the nearest later cell of
the old route (by map distance) and follows on. Why: T207's chain legs differ from their standalone
routes only in carried state (RNG, wings, hover phase), and re-planning the whole leg discarded a
route that was mostly right.

### 2.8 The map cache

Maps are saved under `~/.cache/superfrog-re/maps/` (beside the oracle build, shared by all
worktrees), keyed by level id and the engine build (`SF_ENGINE_BUILD`), so an engine change
invalidates them. `--no-cache` builds fresh. The benchmark always passes `--no-cache`, because a map
built once and reused would hide its cost from M3.

### 2.9 The unhappy paths

- Budget exhausted: the planner exits non-zero, writes the best partial route to `--out` and a
  one-line reason (coins needed, last cell, stalled target) to stderr. Why: a partial route is what the
  next agent resumes or diagnoses from.
- A coin or switch the map calls unreachable: dropped from the choice with a stderr line; if the exit's
  count cannot then be met, the planner says so and exits non-zero at once rather than searching the
  whole budget.
- The oracle hangs or panics: the oracle runs under its frame cap (rust-engine DESIGN §5.2); the chunk
  is rejected as a dead path.
- A corrupt or stale map cache file: rebuilt, never trusted. The key includes the engine build.
- Two planners at once (the benchmark, a worker): cache writes go to a temp file then rename, so a
  reader sees a whole map or none.
- A benchmark run killed midway: per-level results are appended as each level ends; a rerun skips
  levels with a result line for the same planner, set and commit.

### 2.10 Hints (owner, 2026-10-20)

`--via c,r` (repeatable; cells to pass through, in order, before the coin choice's own targets resume),
`--avoid c0,r0,c1,r1` (repeatable; cells the search treats as deadly) and `--follow <script>` (§2.7,
also usable with a partial route as a starting input). Why: an agent stuck on a level that matters
today needs a way through, and the planner's own fix may take a task.

Every run that uses `--via` or `--avoid` appends one line to `engine/conform/plan-hints.log` (tracked
in git): date, level, start, the hints, and whether the run completed. `--follow` alone is not logged,
because route repair is a planned use. Why: the log lands in the diff of whoever used a hint, so each
workaround is visible and becomes a planner gap to fix rather than a habit. `PLAN_STATS` carries
`hints=N`; `sf-bench` passes no hint options and rejects a result whose `hints` is not 0.

---

## 3. Architecture

### 3.1 The boundary

```
engine/crates/sf-plan/src/core/   — map graph, coin choice, danger cost, search policy, repair policy.
                                    Takes Game states and level facts as arguments; may step sf-core.
                                    No clock, file, environment, oracle, network or random seeding
                                    other than the seed passed in.
engine/crates/sf-plan/src/        — everything else: CLI, oracle confirmation, cache files, threads'
  (shell)                           wall-clock budget, benchmark.
```

`sf-core` is pure (rust-engine DESIGN §3.1), so the core may step the engine and stay deterministic.
Enforced by `engine/crates/sf-plan/tests/purity.rs`, which scans `src/core/` for `std::time`,
`std::fs`, `std::env`, `std::process`, `std::net`, `sf_oracle`, `sf_conform` and `Instant`. If that
test fails, move the code to the shell side; never relax the test. Why: everything in the core is
testable in milliseconds and reproducible to the byte, and M8 depends on it.

Threads: the core exposes work as partitions (`expand(part) -> Vec<Child>`); the shell runs them on
`std::thread::scope` and passes results back in partition order. The core never spawns.

### 3.2 Modules

- `core::facts`: level facts from a `Game` state and `sf_conform::game_data()`'s T7MP: standable
  cells, coin cells (objects and tile coins), switches and gates, exit cells, coins needed, frog cell
  (`((X+0x17)/16, (Y−0x10)/16)`), required boss. Replaces the copies in the four old planners.
- `core::map`: `MoveMap` (cells, edges, preconditions), built by stepping the engine.
- `core::coins`: coin choice over a `MoveMap`.
- `core::danger`: occupancy sampling and the cost lookup.
- `core::search`: macro following, beam fallback, archive, scoring.
- `core::repair`: `--follow` policy.
- `oracle`: chunked or stretch confirmation on `sf_oracle::Amiga` through `sf_conform::sync`.
- `cache`: map files (bincode, `serde` derives already in the workspace).
- `main` (bin `sf-plan`): the one command.
- crate `sf-bench` (bin `sf-bench`): §4.2. A separate crate that runs every planner, old and new, as a
  child process, so measuring never links the thing measured.

### 3.3 Data flow

start state → `facts` → `map` (cache or build) → `danger` → `coins` → `search` (engine; in original
mode chunked through `oracle`) → `InputScript` → `--out`.

### 3.4 Storage

Maps in the cache dir (§2.8). Benchmark results in `plans/smart-planner/bench/results.jsonl`, one JSON
object per level run, appended and flushed after each level; the table `plans/smart-planner/bench/RESULTS.md`
is generated from it, never hand-edited. A crash loses at most the level in flight.

---

## 4. Testing

**Unit and engine tests (test block).** `facts` against known level counts; `coins` on synthetic
graphs; every edge of the W1-1 map replays on the engine to its end cell (a sample of edges on other
levels); M8: plan W1-1 in app mode twice with one seed, byte-equal (engine only, a minute at most);
the purity test.

**The benchmark (not in the test block).** Too slow: a counted set-A run is up to 12 h. It is run as
its own command (§4.2) and its output table is committed.

### 4.1 What counts as a completed route

Set A: a case `{ sync = { level = id }, script, frames, resync = true }` passes `sf_conform::run` (the
faithful check as `complete-*` cases pass it) and its milestones include the level's exit. Set B: the
same with `sync = { script = "game-whole", frame = <leg start>, df1 = true }`, the leg's start frame
taken from the leg snapshot's frame number (`sync::load_snapshot_file`). Set C: the script replays on the engine from the same
`start_level` state to `level_state` ≠ −2 with no life lost.

### 4.2 The benchmark

`cargo run --release -p sf-bench -- --set A|B|C --planner new|plan_exit|plan_exit_w6|explore [--levels W1-4,..] [--jobs N]`.
Rules (unaided): level, start, goal; seed 1; no level-specific options; budget 30 min wall-clock per
level including confirmation; maps built fresh. For the old tools "unaided" means each run with
`--coins 0` style defaults and no `--via/--avoid/--ground/--jump/--launch/--fly/--skip/--to`; the
baseline records the best of the three old tools per level, each given the full budget in turn,
because an agent today would try them all. Records per level: completed, wall-clock, engine frames,
oracle frames, confirmation share, route frames, hearts lost, failure reason.

Counted runs (T15–T17) run one level at a time with no other build or test on the machine
(`--jobs 1`), because four oracles at once each run at ~95 frames/s against ~314 alone
(rust-engine FINDINGS 2026-10-11). Set C may use `--jobs` up to the core count. M7 (T13) is engine
only and runs levels in parallel.

---

## 5. Environment

| | |
|---|---|
| OS | macOS 26 on Apple M4, 10 cores, 16 GB |
| Language / runtime | Rust 1.99.0 (cargo 1.99.0), Python 3 venv for `tools/` |
| Oracle | vAmiga headless, built by `tools/setup/oracle.sh` into `~/.cache/superfrog-re/vamiga/` |
| Disks / ROMs | four zips in the main checkout root (`SUPERFROG_DISKS`), Kickstart in `~/amiga-roms/` |
| Deliberately absent | no rayon, no ML framework, no GPU use |

**The test command** is the block's `test` lines, the rust-engine plan's lines. The repo's stored
`.pir/settings.json` test line runs only the Python suite; this plan adds Rust code, so it uses both
lines. `CARGO_TERM_COLOR=never` and `NO_COLOR=1` keep it colourless (the shell sets
`COLORTERM=truecolor`, which forces nothing, but cargo colours on a TTY); `-q` keeps a pass to
summary lines. A person debugging runs `cargo test -p sf-plan -- --nocapture`.

**Measured 2026-10-20**, quiet machine, from each level's `AtLevel` state: `Game::step_unseen` 1 386
frames/s on W1-4 and 1 251 on W6-3, one thread; `Game::clone` 30–40 µs; the oracle 565–571 frames/s
from a sync point. A busy machine (a cargo build beside it) halved the engine and cut the oracle to a
third. A map of ~3 000 standable cells × ~10 moves × ~40 frames is ~1.2 M frames, ~90 s on 10 threads;
T00 measures it properly.

**The full test block takes ~50 min** on this machine (Python 28 s; `cargo test --workspace
--no-fail-fast` 3 172 s on 2026-10-20). New `sf-plan`/`sf-bench` tests in the block must be cheap:
anything over ~30 s is `#[ignore]` with its time noted, and a worker iterating runs
`cargo test -p sf-plan` rather than the whole block. The cargo tests need the venv from `setup` (the
Python reference tools), so a worktree without it fails four sf-assets/sf-disk tests with "run the
test block's setup".

**Dependencies.** No new crates. `std::thread::scope` for parallelism (as today's planners), `serde` +
`bincode` (already in sf-core) for the cache, `toml` (in sf-conform) for case files. `serde_json` is
not in the lock file, so `results.jsonl` lines are written and parsed by hand (flat objects of numbers
and short strings).

### 5.1 What the test command cannot reach

| Cannot be tested automatically | Why |
|---|---|
| The counted benchmark timings | Hours of machine time on a quiet machine; run by T15–T17 as their own command, results committed |

Nothing here needs a person: every measure is machine-decided.

### 5.2 Seatbelts

| Mechanism | Default | Effect |
|---|---|---|
| `sf-plan --budget-min N` | 30 | Planner gives up at N minutes wall-clock |
| `sf-plan --frames N` | 30 000 | No route longer than N frames; the oracle's own frame cap applies to every confirmation |
| `sf-bench` per-level timeout | budget + 5 min | Kills a planner child that ignores its budget |
| Oracle frame cap | rust-engine DESIGN §5.2 | Every oracle run is bounded |

Every oracle and planner process the benchmark starts is a child it kills on exit, so a killed
benchmark leaves no emulator running.

---

## 6. Recovery

Nothing here can lock anyone out. A bad cache is deleted with `rm -rf ~/.cache/superfrog-re/maps`.
The old tools stay in the tree until T18, so any level can still be planned the old way until then.

---

## 7. Decisions and rationale

- D1 (owner, 2026-10-20): one planner for the 24 normal levels. WB-1 (bonus shooter) keeps
  `plan_bonus` and its committed route, WW-1 (the witch) and the slot machine keep theirs, because
  none of them is map routing. Set B is the 24 normal-level legs.
- D2 (owner): the counted baseline and the counted new run go one level at a time on a quiet machine,
  timed in wall-clock; a flat 30 min per level for every level, W4-4's 24 000-frame route included.
  Why: four oracles at once each run at a third of their speed, and parallel build workers distort
  timings.
- D3 (owner): set C stays in this plan and waits for `pir/bug-recorder` (which adds
  `Game::start_level` and `LevelSetup`) to be merged into main. T12 checks for it and stops to ask if
  absent. Copying the API here was refused: two copies collide at merge.
- D4 (owner): an early baseline sample (T02) guides the design; the counted baseline (T16) runs at the
  end, just before the counted new run, while the old tools still exist.
- D5 (owner): a missed target at the end is brought to the owner with its cause; clear-cut planner bugs
  are fixed and those levels re-run within the task. The old tools are deleted once M1 holds or the
  owner accepts the miss.
- D6 (owner): techniques in: planning in map moves, check-less-repair-diffs (conditional on T02),
  danger map, map cache. Out: learning from committed routes (it is a hint), a trained model.
- D7 (owner): a throwaway spike (T00) proves the map on W1-4 and W6-3 before anything is built on it.
- D11 (owner, 2026-10-20): hints (`--via`, `--avoid`, `--follow` as starting input) are allowed for
  agents but never in the benchmark, and every `--via`/`--avoid` use is logged to
  `engine/conform/plan-hints.log` as a planner gap. Alternatives refused: no hints (leaves an agent
  stuck with no way through), hints only through `--follow` (clumsy for "go via this tile").
- D8: the planner is a new crate `sf-plan`, not more examples in sf-conform, because examples cannot be
  unit-tested or shared, which is how four copies of the level facts came to exist.
- D9: the benchmark verifies with `sf_conform::run` on a generated case, the check the completion
  cases already use, so "counts" means the same as in the rust-engine plan. Set B uses the existing
  `AtFrame { game-whole, frame, df1 }` sync point.
- D10: old tools' baseline is the best of three per level, each with the full budget, since an agent
  today would try all three.

---

## 8. Explicitly out of scope

- Easy and hard completions: need the difficulty-modes plan's conformance first (owner's order).
- Re-planning committed routes: they pass today; this plan only shows equivalent ones can be produced.
- A goal-free explorer for finding bugs: the M7 explorer may be reused later; no bug-finding here.
- Learning from committed routes or a trained model.
- Any change to a game mechanic or the faithful bar.
- The bonus level, the witch and the slot machine (D1).
- The map editor and the bug recorder.
