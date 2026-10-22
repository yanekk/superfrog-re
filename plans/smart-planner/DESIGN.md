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
input script that plays the level to its exit on the original, with no hints: no hand-placed
waypoints, no per-level options, no choice between tools (a guide path, §2.11, is allowed). Today four tools (`plan_exit`, `plan_exit_w6`, `explore`,
`plan_bonus` in `engine/crates/sf-conform/examples/`) each steer by hand-patched heuristics, and the
rust-engine run spent hours on W1-4, W2-4, W6-2 and W6-3 (plans/rust-engine/FINDINGS.md T149, T150,
T168, T207 rows). The root cause is that their distance field knows geometry and not movement: open
air under a coin reads as near. This plan replaces them with one planner, and proves it against a
baseline of today's tools. It is the tool that later plans easy and hard completions (the
difficulty-modes plan), so it is proven here by numbers.

**Amended 2026-10-22 (D19–D24).** The plan first steered by a measured movement map (T04). That map
reached the exit on 13 of 24 levels; an experiment off T04 (`spike-explore/`) finished 8 of the 11
others with a whole-level explorer and no map, and a 9th (W6-2) guided by its committed route. The planner is now that explorer (§2.4); the map is an
optional compass (§2.2); a guide path, such as a level's committed route, may steer it everywhere,
the counted benchmark included (§2.11).

### Success criteria

The benchmark (§4) committed in `plans/smart-planner/bench/` shows, side by side with today's tools:

| # | Measure | Target |
|---|---|---|
| M1 | Set A levels completed within budget, faithful on the original | 24 / 24 |
| M2 | W1-4, W2-4, W6-2, W6-3 completed | 4 / 4, each under 30 min |
| M3 | Planning time per level, set A | median ≤ 10 min, slowest ≤ 30 min |
| M4 | Route length (frames to exit) vs the committed route, per level | none more than 25% longer |
| M5 | Set B legs completed | 24 / 24, same budget |
| M6 | Set A routes losing no heart | ≥ 20 / 24; each short one explained (unavoidable hit or planner gap) |
| M8 | Same seed, same level, same start → byte-identical route | always (a test) |
| M9 | Tools an agent chooses between for level routing | 1 (`plan_bonus` stays for WB-1) |
| C1 | Set C levels completed on the engine with app timing | 24 / 24 |
| C3 | Set C planning time | median ≤ 2 min, slowest ≤ 30 min |
| C4 | Set C route length vs the committed route | none more than 25% longer |

M1–M6 and C1–C4 are measured guided: each level's committed route is the guide path (§2.11, D21).
M7 (the map missing no reachable cell) was dropped on 2026-10-22: the map is a compass and need not
be complete (D20).

Recorded, not gated: engine frames simulated per level (machine-independent, so later changes compare
fairly), oracle frames, the share of wall-clock spent checking on the original, route repair on set B
(given the leg's standalone route), and, from the tuning run only, each set-A level planned unaided
(no guide), so what the planner does on a level with no route is known.

### Stance

- No per-level knowledge but a guide path. The planner plans from level, start, goal and, if given,
  a guide path (§2.11): cells to pass near in order, taken from a committed route, or written by hand
  or by an agent. A guide is not a hint (owner, 2026-10-22, D21): easy and hard mode have a committed
  normal-mode route for every level. Hints (`--via`, `--avoid`) remain an escape hatch (§2.10), every
  use logged as a planner gap, never used by the benchmark.
- The engine decides what a move does, never a formula. Every move the planner keeps was played on
  `sf-core` (or the original), so nothing disagrees with the game about what a jump reaches.
- The original has the last word. A route in original mode counts only if the faithful check of
  rust-engine DESIGN §2.1 passes on it, as the completion cases do.
- The planner only reads the engine. No game mechanic, no faithful-bar tolerance changes here.

---

## 2. Behaviour specification

### 2.1 Inputs and output

The planner takes a level id, a start and options that are not level-specific: seed, mode
(original / app), budget, `--guide <file>` (§2.11), `--follow <script>`, `--out`, `--no-cache`, and
the hints of §2.10. Starts:

- `AtLevel(id)`: the level's sync point (set A).
- `--snap <leg.snap>`: a `whole_game` leg snapshot, carrying earlier legs' state (set B).
- `--app`: `Game::start_level(display, &LevelSetup{level, ..Default::default()}, Rng::seeded(seed, seed))`,
  the start Game ▸ Start Level… makes under a script (set C; the API is on `pir/bug-recorder`, §7 D3).

Output is an `InputScript` in today's text format (`frame: inputs`, one line per change, closing
`N: -`), ending in the frame the level's `level_state` leaves −2 plus the same idle tail the committed
`complete-*` scripts carry, so it drops into a case file unchanged. Why: every consumer of routes
(cases, `whole_game`, the bug recorder) already reads that format.

The goal is the exit with the level's coin count met (`COINS_LEFT` = 0 as the exit reads it). No normal
level has a boss (the witch on WW-1 is the game's only one, `sf-core/src/bosses/mod.rs`; out of scope,
D1), so the planner has no kill goal. Old routes sometimes used an enemy on purpose (W2-4's hopper:
two stomps make a reward, rust-engine FINDINGS T168). T04 checks on every level that the map reaches
the exit with the count met and no enemy's help; a level where it does not goes to the owner before
T08 (D13).

### 2.2 The movement map, a compass (amended 2026-10-22, D20)

The map T04 built (`core::map`: standable cells reached by real moves from the level start, one seed
state each, ~50 macro moves per seed, edges that replay to their landing, switch worlds near gates;
cached by T05) stays as an optional compass: a distance in edge frames from a cell to the exit, for
the explorer's choice weight (§2.4). It is never a gate. A cell with no map path is not unreachable,
and nothing waits on the map being complete. Why: the map reached the exit on 13 of 24 levels (T04
FINDINGS rows), 46–55% of its edges land elsewhere live (T00 Q7), and each gap closed so far cost a
hand-found move family; the explorer needs none of it (spike-explore/README.md).

The default compass is the straight distance in cells (|Δcol| + |Δrow|), no map. T14 tries the map
compass on the stall levels and keeps it only if it completes more levels or plans them measurably
faster (owner, 2026-10-22); otherwise the map code stays as built and unused.

The map's own rules from before the amendment (speed bands D15, learned and unreliable edges D16 and
D17, M7) bind nothing now: the explorer has no edges to learn or distrust.

### 2.3 Coin choice

T06's coin choice (a set and an order of object coins meeting the count, by map distance) is not on
the explorer's path: the explorer takes coins as it finds them, and its progress is the count left
(§2.4). Like the map it is a compass T14 may try (a pull toward the next chosen coin), kept only if
it measurably helps. The exit counts object coins only (T03: tile coins never lower
`coins_left_for_exit`).

### 2.4 The search: a whole-level explorer (D19)

One archive for the whole level, in the Go-Explore pattern. Its key, a cell, is (frog column / 2, frog
row / 2, coins band, switches pressed, wings held); the coins band is 0 when `COINS_LEFT` (h0+$832) is
0 and 1 + (left − 1) / 4 otherwise. Each entry holds the best state reaching its key (more hearts
first, then fewer frames), the inputs from the start to it, and two counters: times chosen, times
reached.

Each round chooses cells and plays a burst from each:

- The choice is weighted: novelty (1/√(1 + chosen) + ½/√(1 + reached)) × progress (e^(−0.8 × bands
  behind the furthest band reached)) × in band 0 an exit pull (4·e^(−d/12) + 0.2, d the compass
  distance to the nearest exit cell) × the guide weight (§2.11) × 0.25 per heart below the start's.
- A burst is 1–16 holds of one of ten joystick states (none, the four directions, up with left or
  right, fire, fire with left or right) for 4, 8, 16 or 32 frames each, keeping the last hold's state
  with probability 0.6. After each hold the state is offered to the archive.
- A burst ends on a life lost (a 1-up gained is not a loss, FINDINGS T205 row on `plan.rs`),
  `DYING`, a caught engine panic, the route reaching `--frames`, or the level ending. The level ending
  with the frog alive, surviving a 40-frame idle tail, is an exit.

The constants are the experiment's (spike-explore/README.md); T14 tunes them, in general, never per
level. The search stops at the first exit, or later for shortening (§2.12). A budget spent with no exit
writes a reason line (§2.9): the furthest band, the least distance to an exit reached with the coins
met, and the cells archived.

Determinism (M8): the round's choices and each burst's seed come from one generator seeded by
`--seed`; every burst of a round starts from the archive as it stood at the round's start, bursts run
in parts on threads and their results are merged in choice order. The bursts per round are fixed by
options, never by the machine, so the route is the same for any thread or part count. Rounds are
counted, never timed; the wall-clock budget only decides when to stop, and a run it stops is not
held to M8.

Memory (D22): a stored `Game` is ~0.8 MB. The experiment's archives of 15 000–40 000 cells took
5–8 GB, and the OS killed a 25-minute run for low memory. The archive therefore keeps a full state for
a bounded set of entries (checkpoints) and rebuilds any other entry's state by replaying its inputs
from its nearest checkpoint. Checkpoints are immutable once taken, so an entry replaced by a better
state never invalidates the entries reached through it. The cap is kept by the core's own count of
the bytes it stores (states and inputs), never by a reading of the OS, so which entries keep states
is a function of seed and options and a rebuilt state equals the stored one: the route does not
depend on the cap (M8). The shell reports peak resident memory (`peak_mem_mb`), which stays under
`--max-mem-gb` (default 4, §5.2). Which entries keep states is T08's choice, recorded in FINDINGS.

### 2.5 Danger

There is no danger map and no wait-and-retry step: enemies and hazards act on the bursts as in play,
and standing still is one of the burst's holds. A state that lost a heart is archived but loses to any
state of its cell with more hearts, and its choice weight drops (§2.4), so a route avoiding the hit
wins where one exists (M6). T07's `danger::play_safe` is not on the explorer's path and is left as
built (D14's live judgement holds; its wait variants are no longer needed).

### 2.6 Checking on the original (amended 2026-10-22)

Original mode plans on the engine and confirms on the original by stretch and repair, the
experiment's loop (spike-explore, `--original`):

1. Transplant the original's memory into a `Game` (`Start::Transplant`) and plan on the engine to the
   exit.
2. Play that plan on the oracle (`sync::step_inserting`) in lockstep with the engine. It parts where
   the frog cells differ by more than one, a life is lost, `DYING` is set, `sync::in_wall_loop`
   holds, or the original loses a heart the engine did not.
3. Keep the plan to 16 frames before the parting, and plan again from the original's state there,
   the rest of the old plan as the guide and the archive seed (§2.7).
4. Done when the original exits. A parting at the same cell three repairs running accepts what the
   original insists on there (a heart lost), so an unavoidable hit cannot loop for ever.

Why stretches, not T02's 48-frame chunks: the explorer makes a whole plan to the exit, the original
agreed with it for 1 700–2 100 frames at a time, and W2-1 and W5-2 then passed `verify` (1 and 8
repairs, 85 s and 482 s). Why the heart check: W5-2's experiment route lost 3 hearts on the original.

An engine route is never accepted on the engine's word: W3-2's engine-only route parted from the
original at frame 3 737 and died at 5 753, yet `sf_conform::run` passed it (with resync the engine
follows the original into the death); only `verify`'s exit milestone caught it. The final route is
accepted only by `verify` (§4.1).

App mode (set C) confirms on the engine from the same `start_level` state; there is no original.

Two timings, never crossed (owner, 2026-10-20). The original lags in busy passes and the engine
never does in normal play (docs/08 "Lag"), so a route belongs to the timing it was planned in. Every
route file's first line is `# timing: original` or `# timing: app`. An original route is never
replayed in the app or from `start_level`, and an app route never on the oracle: bug-recorder found
`complete-w1-1` ends in game over from `start_level` for every seed and offset (its FINDINGS, T06),
and its recordings left the original under lag (T00 Q2). A route that diverges is first checked for a
mixed timing; it is never patched by padding or shifting frames. The fix is re-planning in the right
timing, or route repair (§2.7) from where it parts. A guide path (§2.11) is cells, not inputs, so a
guide from an original route steers an app-mode plan without crossing timings.

### 2.7 Route repair

`--follow <script>` turns an old route into both a guide and an archive seed (given with `--guide`,
the `--guide` path steers and `--follow` only seeds): its frog cells, from
playing it at its own start in its own timing, become the guide path (§2.11), and its inputs are
replayed from the new start, every 8th state seeding the archive until the frog dies. The search then
runs as usual. Why both: replayed inputs reach far at once where the carried state differs little
(T207's chain legs: RNG, wings, hover phase), and the cells keep steering after they part. Original
mode's repair (§2.6) is this, with the rest of the plan as the old route. Replayed inputs are only a
seed: a route is accepted in a timing only after it is played there (§2.6).

### 2.8 The map cache

Maps are saved under `~/.cache/superfrog-re/maps/` (beside the oracle build, shared by all
worktrees), keyed by level id and the engine build (`SF_ENGINE_BUILD`), so an engine change
invalidates them. `--no-cache` builds fresh. The benchmark always passes `--no-cache`, because a map
built once and reused would hide its cost from M3. A run without the map compass (§2.2) builds and
reads no map.

### 2.9 The unhappy paths

- Budget exhausted: the planner exits non-zero, writes the best partial route to `--out` (the
  archived state furthest on: lowest band, then nearest an exit) and a one-line reason (coins needed
  still, least exit distance with the coins met, cells archived, last confirmed frame on the original)
  to stderr. Why: a partial route is what the next agent resumes or diagnoses from. The planner never
  exits early on evidence that a level is impossible (D16 holds).
- Memory: the archive stays under `--max-mem-gb` (§2.4, §5.2). It never grows until the OS kills it:
  the experiment's 25-minute run was killed so.
- The oracle hangs or panics: the oracle runs under its frame cap (rust-engine DESIGN §5.2); the plan
  parts there, as at a death (§2.6).
- A guide file that does not parse, or a guide script whose own replay never leaves the start cell:
  exit 2 with the reason. A guide that is merely wrong (another timing, a changed level) is not an
  error: off-path cells keep a weight (§2.11).
- A corrupt or stale map cache file: rebuilt, never trusted. The key includes the engine build.
- Two planners at once (the benchmark, a worker): cache writes go to a temp file then rename, so a
  reader sees a whole map or none.
- A benchmark run killed midway: per-level results are appended as each level ends; a rerun skips
  levels with a result line for the same planner, set and commit.

### 2.10 Hints (owner, 2026-10-20)

`--via c,r` (repeatable; cells to pass through, in order, before the search's own progress resumes),
`--avoid c0,r0,c1,r1` (repeatable; cells the search treats as deadly) and `--follow <script>` (§2.7,
also usable with a partial route as a starting input). Why: an agent stuck on a level that matters
today needs a way through, and the planner's own fix may take a task.

Every run that uses `--via` or `--avoid` appends one line to `engine/conform/plan-hints.log` (tracked
in git): date, level, start, the hints, and whether the run completed. `--follow` alone is not logged,
because route repair is a planned use. Why: the log lands in the diff of whoever used a hint, so each
workaround is visible and becomes a planner gap to fix rather than a habit. `PLAN_STATS` carries
`hints=N`; `sf-bench` passes no hint options and rejects a result whose `hints` is not 0. A guide
(§2.11) is not a hint and is not logged.

### 2.11 Guide paths (owner, 2026-10-22, D21)

`--guide <file>` gives a path of cells to pass near, in order. Two forms, one shape to the core
(`Vec<GuidePoint { cell, coins_left: Option<u16> }>`):

- A route script (its first line `# timing: …`, or any `InputScript` file): played once at its own
  start in its own timing (on the original for `original` or no timing line, on the engine from the
  `start_level` state for `app`), sampled every 8 frames for the frog's cell, the coins left and
  the switches pressed. The shell plays it; the core takes the points.
- A waypoint file: one `col,row` or `col,row,coins_left` per line, `#` comments. Written by hand or,
  later, by an agent from the annotated level image (`tools/export_annotated.py`); writing one by
  agent is out of scope (§8, owner 2026-10-22).

The core gives each archived cell the furthest point index p whose region it falls in (the point's
coarse cell ±1, its band when the coins are known, any band when not, and its switches pressed when
known, any when not: a waypoint names none). Switches (owner, 2026-10-22, T09): W6-2's route goes
back over its own ground after each switch; without them the guide credited that ground to a frog
that never pressed the switch, and every seed tried stalled at a closed gate. The choice weight is
3·e^(−(p_max − p)/20) on the path (p_max the furthest index any archived cell has), 0.15 off it.
Off-path cells keep a weight because a guide from another start or mode may be wrong in places. Why:
W6-2 went from stalled 6 cells short to an exit in 80 s guided by its committed route's path.

A guide is not a hint (§2.10, D21): it is not logged, and `PLAN_STATS` carries `guided=0|1`. The
counted benchmark guides each level by its committed route (§4.2).

### 2.12 Route length (D23)

The experiment's routes ran up to 57% longer than the committed ones (W3-2 engine-only, 10 962
against 6 994 frames; 19–23% in original mode; M4: at most 25%). After the first exit the search goes
on for a shortening budget and keeps the shortest exit. The budget is counted in rounds, so M8 holds:
default 20% of the rounds used to the first exit, at least a fixed minimum (about 60 s of rounds on
this machine, T22 sets the number); the wall-clock budget still caps it. Then a shortcut pass: for states i < j on the route, a short explorer run
from i targets j's key within (j − i) − 1 frames, and a shorter stretch found replaces the old one,
until a pass finds nothing or the budget ends. A shortened route is kept only if it replays from the
start to the exit with no more hearts lost. In original mode the shortened plan is the one
confirmed (§2.6). T22 builds it; T14 tunes its defaults.

---

## 3. Architecture

### 3.1 The boundary

```
engine/crates/sf-plan/src/core/   — archive, explorer, guide weights, shortening, compasses (map graph,
                                    coin choice), repair policy.
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
  (`((X+0x17)/16, (Y−0x10)/16)`). Replaces the copies in the four old planners.
- `core::archive`: the explorer's cells, entries, counters and checkpointed states (§2.4, D22).
- `core::explore`: rounds, choice weights, bursts, exits; `Compass` (straight by default, `MoveMap`
  optional).
- `core::guide`: guide points and their weights (§2.11).
- `core::shorten`: the shortening budget and shortcut pass (§2.12).
- `core::map`, `core::graph`: `MoveMap`, built by stepping the engine (T04); a compass only (§2.2).
- `core::coins`: coin choice over a `MoveMap` (T06); a compass only (§2.3).
- `core::danger`: the live heart check with wait variants (T07); not on the explorer's path (§2.5).
- `core::repair`: `--follow` policy (old route → guide and seed, §2.7); the engine trace of a route.
- `run`: the explorer's rounds on threads, the wall-clock budget, shortening's call (T08, T22).
- `trace`, `guide_file`: a guide script played at its own start on the original, a waypoint file
  parsed (§2.11, T09).
- `oracle`: stretch-and-repair confirmation on `sf_oracle::Amiga` through `sf_conform::sync`
  (§2.6, T10).
- `cache`: map files (bincode, `serde` derives already in the workspace).
- `main` (bin `sf-plan`): the one command.
- crate `sf-bench` (bin `sf-bench`): §4.2. A separate crate that runs every planner, old and new, as a
  child process, so measuring never links the thing measured.

### 3.3 Data flow

start state → `facts` → (guide file → `oracle`/engine replay → guide points) → `explore` (archive,
compass, guide; engine) → `shorten` → in original mode `oracle` stretch and repair, back to `explore`
from each parting → `InputScript` → `--out`.

### 3.4 Storage

Maps in the cache dir (§2.8). Benchmark results in `plans/smart-planner/bench/results.jsonl`, one JSON
object per level run, appended and flushed after each level; the table `plans/smart-planner/bench/RESULTS.md`
is generated from it, never hand-edited. A crash loses at most the level in flight.

---

## 4. Testing

**Unit and engine tests (test block).** `facts` against known level counts; `coins` on synthetic
graphs; every edge of the W1-1 map replays on the engine to its end node, landed (a sample of edges on other
levels); the archive's rebuilt states equal the stored ones byte for byte (D22); M8: plan W1-1 twice
with one seed, byte-equal, engine only, in the core (T08) and through the command (`--engine-only`
T11, `--app` T12); the purity test. The M8 tests stay in the block even over
§5's ~30 s rule, at most ~2 min each, because they are the target's only guard.

**The benchmark (not in the test block).** Too slow: a counted set-A run is up to 12 h. It is run as
its own command (§4.2) and its output table is committed.

### 4.1 What counts as a completed route

Set A: a case `{ sync = { level = id }, script, frames, resync = true }` passes `sf_conform::run` (the
faithful check as `complete-*` cases pass it) and its milestones include the level's exit. Set B: the
same with `sync = { script = "game-whole", frame = <leg start>, df1 = true }`, the leg's start frame
taken from the leg snapshot's frame number (`sync::load_snapshot_file`). Set C: the script replays on the engine from the same
`start_level` state to `level_state` ≠ −2 with no life lost, and also in the real app: `sf-e2e`'s
script layer starts the level by the `start-level` event and plays the route, and the app reaches the
level's exit with no life lost (owner, 2026-10-20). Why: the engine check alone cannot see a
difference in how the app starts or feeds a script (a frame offset, the seed), which is how
bug-recorder's routes failed.

### 4.2 The benchmark

`cargo run --release -p sf-bench -- --set A|B|C --planner new|plan_exit|plan_exit_w6|explore|best-old [--levels W1-4,..] [--jobs N]`.
Rules: level, start, goal and, for the new planner, the level's committed route as its guide
(`--guide`, the script `verify::committed_script(set, level)` names, played at the same start the
planner is given: the `AtLevel` sync point for sets A and C, the leg snapshot for set B; owner
2026-10-22, D21); seed 1; no other level-specific options; budget 30 min wall-clock per level
including confirmation, the guide's own replay included; maps built fresh where the compass uses
them. The results line and the table carry `guided`, so a guided new result is never read as an
unaided one. The tuning run (T14) also plans set A unaided, recorded and not gated. For the old tools "unaided" means each run with
`--coins 0` style defaults and no `--via/--avoid/--ground/--jump/--launch/--fly/--skip/--to/--stomp/--hurt`; the
baseline records the best of the three old tools per level, each given the full budget in turn,
because an agent today would try them all. Records per level: completed, wall-clock, engine frames,
oracle frames, confirmation share, route frames, hearts lost, failure reason.

Counted runs (T15–T17) run one level at a time with no other build or test on the machine
(`--jobs 1`), because four oracles at once each run at ~95 frames/s against ~314 alone
(rust-engine FINDINGS 2026-10-11). Set C may use `--jobs` up to the core count, memory allowing:
each planner may hold `--max-mem-gb` (§5.2), so jobs × that stays under the machine's 16 GB.

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
T00 measured it (busy machine, 10 threads): W1-4's walked map is 1 447 cells, 49 519 edges, 4.8 M frames,
453–738 s; W6-3's 196 cells, 38 s, both mostly spent drawing a picture nothing reads (T19, D18).

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
| `sf-plan --max-mem-gb N` | 4 | The archive keeps checkpointed states under N GB resident (§2.4, D22); the experiment's unbounded archive reached 8 GB and was killed by the OS |
| `sf-bench` per-level timeout | budget + 5 min | Kills a planner child that ignores its budget |
| Oracle frame cap | rust-engine DESIGN §5.2 | Every oracle run is bounded |

Every oracle and planner process the benchmark starts is a child it kills on exit, so a killed
benchmark leaves no emulator running.

### 5.3 Outside actions

None (owner, 2026-10-20). Everything runs on this machine and in this repo: builds, the engine and
oracle, background benchmark slices (up to ~25 h of machine time per counted run), the map cache under
`~/.cache/superfrog-re/maps/`, commits to the plan's branch, and T18's deletion of the old tools
(gated by D5, undone by git). Nothing is deployed, published, sent, paid for or logged into. Why the
table is empty rather than absent: any action reaching beyond the machine that turns up later has no
row, so it is `ask`.

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
  absent. Copying the API here was refused: two copies collide at merge. T15–T18 wait with it, being
  chained after T15 for a quiet machine; the owner keeps that order (2026-10-20) and may reorder if
  the merge is late.
- D4 (owner): an early baseline sample (T02) guides the design; the counted baseline (T16) runs at the
  end, just before the counted new run, while the old tools still exist.
- D5 (owner): a missed target at the end is brought to the owner with its cause; clear-cut planner bugs
  are fixed and those levels re-run within the task. The old tools are deleted once M1 holds or the
  owner accepts the miss.
- D6 (owner): techniques in: planning in map moves, check-less-repair-diffs (conditional on T02),
  danger map (made live by D14), map cache. Out: learning from committed routes (it is a hint), a trained model.
  Amended 2026-10-22 (D19, D21): the explorer replaces planning in map moves; following a committed
  route's path is allowed everywhere. A trained model stays out.
- D7 (owner): a throwaway spike (T00) proves the map on W1-4 and W6-3 before anything is built on it.
- D11 (owner, 2026-10-20): hints (`--via`, `--avoid`, `--follow` as starting input) are allowed for
  agents but never in the benchmark, and every `--via`/`--avoid` use is logged to
  `engine/conform/plan-hints.log` as a planner gap. Alternatives refused: no hints (leaves an agent
  stuck with no way through), hints only through `--follow` (clumsy for "go via this tile").
- D12 (owner, 2026-10-20): `examples/complete` (T42's engine-only, nearest-coin level planner) is
  retired at T18 with the other three but left out of the baseline. Why: it is a fifth level-routing
  tool (M9), its engine-only routes drift on the original (rust-engine T84), and a fourth baseline
  tool would cost about a day of quiet-machine time.
- D13 (owner, 2026-10-20): the boss goal is dropped (no normal level has one). Whether a level needs
  an enemy (a stomp reward, a knock) is checked early, by T04's per-level reachability test, and
  brought to the owner before T08. Refused: assuming none and finding out at T14 (late), building
  enemy use into the search now (maybe unneeded).
- D14 (owner, 2026-10-21): no pre-sampled danger map; danger is judged live on the search's own copy of
  the state, with wait-then-retry variants (§2.5), and only fixed hazards go on the map. Why: enemies
  spawn as their edge scrolls into view (docs/09), so a parked-frog sample misses them and their phase.
  T00 confirms the spawn behaviour on the engine before T07 is built. A retry after a wait is a
  different, standing-start move (owner, 2026-10-21 amendment review): waiting bleeds speed (D15).
- D15 (owner, 2026-10-21; binds the map only since D20): map nodes carry a speed band and facing if T00 measures the build at ~5 min
  or less per level with them; otherwise cells only, with momentum left to the frame-level search.
  Edges end landed, never padded to a stop. Why: momentum (§2.2) and M4.
- D16 (owner, 2026-10-21; since D19 only its rule stands that no evidence of a level being impossible ends a run early): "unreachable on the map" is a cost, never a stop: far-coin cost in the
  choice, exploring when stuck in the search (step-counted, so M8 holds), learned edges for the run,
  and no early exit on map evidence. Why: a map gap must not become a false "impossible".
- D17 (owner, 2026-10-21; map only since D20, the explorer has no edges): edges record the coins they touch on the way, and an edge that fails live
  replay 3 times in a run is marked unreliable (cost ×4) for that run. T00 measures the live-failure
  rate. Why: coins taken mid-jump, and a world (object slots, spawns, RNG) that differs by route.
- D18 (T00 spike, owner 2026-10-21; recorded by T00's review; binds the map only since D20): seed the map by real moves, never by
  writing the frog's X/Y (placed: 52% of W1-4's and 54% of W6-3's edges land elsewhere from a real
  state); build with enemies live (frozen-world edges fail live 58.8% against 46.2% on W1-4, 67.6
  against 55.0 on W6-3); no speed bands (W1-4 1 699 s against 227 s cells only, over D15's ~5 min).
  Today's engine builds W1-4 in 453–738 s, over ~5 min: the owner added T19 (`step_unseen` skips
  `level_as_drawn`, ~3.2× fewer CPU-seconds), refusing a coarser move set or a lazily built map. W1-4's
  44 stall coins are all reachable; W6-3's map is wrong without mid-air steering, so T04 adds it and
  must reach W6-3's exit. Live replay fails 46–55%, about half one cell off: a move landing within one
  cell of its target counts as success, the search going on from where the frog is, and D17's
  3-failure ×4 rule stands. D14 holds: parked 2 000 frames, 1 of W1-4's 60 enemy cells spawns.
- D19 (owner, 2026-10-22): the planner is a whole-level explorer (§2.4), not map following. Why: T04's
  map reached the exit on 13 of 24 levels and T20 was to patch the other 11 by hand; an experiment off
  T04 (`spike-explore/`, ~470 lines, engine only, 10 threads, 10 min, no map, no hints) finished 8 of
  those 11, and a 9th (W6-2) with a guide; in original mode W2-1 and W5-2 passed `verify`. Open in it:
  W2-3 and W6-4 stalled 17 and 13 cells short of the exit, memory, hearts on the original, route length.
  T08–T10 are rewritten for it, T13 (M7) and T20 dropped, T22 added.
- D20 (owner, 2026-10-22): the movement map is kept as an optional compass (§2.2), tried in T14 and kept
  only if it measurably helps; M7 and T13 are dropped. Refused: dropping the map outright (loses a
  possible sense of direction already built), keeping M7 (brings back T20's hand patching).
- D21 (owner, 2026-10-22): a guide path (§2.11) is allowed everywhere, the counted benchmark included,
  which guides each level by its committed route; unaided set-A results come from the tuning run,
  recorded, not counted. Refused: guides as a separate counted mode, two counted runs (~25 h more), and
  unaided counted runs. Cost accepted: the counted numbers prove nothing about a level with no route;
  the old tools' baseline stays unaided, and the table says which results were guided. The planner
  reads hand or agent waypoint files; an agent writing them is not built here (§8).
- D22: the archive bounds its memory by checkpointing states and rebuilding the rest by replay, under
  `--max-mem-gb` (§2.4). Why: 0.8 MB a state, 5–8 GB in the experiment, and the OS killed a 25-minute
  run.
- D23 (owner, 2026-10-22): a route-shortening task (T22, §2.12) is added, because the experiment's
  routes ran up to 57% over the committed ones (19–23% in original mode) against M4's 25%. Refused: leaving length to T14.
- D24 (owner, 2026-10-22): original mode confirms by stretch and repair (§2.6), not T02's 48-frame chunks. T02 chose chunks
  for the old tools on a small confirmation share; the explorer plans whole stretches, and its repair
  loop is what passed `verify` on W2-1 and W5-2.
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
- A goal-free explorer for finding bugs: the planner's explorer may be reused later; no bug-finding here.
- An agent writing guide paths from a level's picture (map-editor levels): the planner reads a
  waypoint file (§2.11), but writing one by agent belongs to the map-editor plan (owner, 2026-10-22).
- Learning from committed routes or a trained model.
- Any change to a game mechanic or the faithful bar.
- The bonus level, the witch and the slot machine (D1).
- The map editor and the bug recorder.
