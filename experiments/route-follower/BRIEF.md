# Route follower — experiment brief (2026-10-24)

**Not a plan.** No `plans/` folder, no tasks, no `pir`. One experimenting session (or a few) builds
throwaway prototypes, measures them against today's fixed scripts, and hands the owner the numbers
and a recommendation. The owner then decides whether either approach becomes a plan.

Branch `experiment/route-follower`, cut from `pir/smart-planner` at `0a66ec7` (T26 reconciled). Work
and commit here only; push this branch; never touch `pir/*` branches or `main`.

## 1. The question

Every route today is a **fixed script**: the joystick state per frame (`InputScript`, `frame: inputs`).
It replays exactly, which the faithful check needs, but it is blind: when the timing shifts by a frame
(the original lagging in a busy pass, the app's `start_level` start, a leg starting a pass late as in
T31, a different difficulty mode) the script carries on and the frog misses. That is why DESIGN §2.6
keeps "two timings, never crossed" and why routes are re-planned or repaired per timing.

Two alternatives the owner asked to be measured:

- **A. Position-triggered script.** A route as a sequence of steps "when the frog's state matches
  this trigger, hold these inputs (until the next trigger)". Triggers are on game state (frog cell,
  maybe sub-cell X/Y, on-ground, facing, coins left), not on frame numbers. Derived automatically from
  an existing fixed route by playing it once and cutting it where the inputs change.
- **B. Reactive follower.** A route as a path of places to pass (the guide points of DESIGN §2.11:
  cell + coins left, already produced by `sf-plan`'s `trace.rs` / `core::repair::trace_engine`). A
  controller decides the inputs every frame (or every few) from the live state. Two variants worth
  trying, the experimenter may add others:
  - **B1, rule-based:** steer toward the next point; jump/hold by simple rules. Cheap, likely weak on
    platforming.
  - **B2, short look-ahead (model-predictive):** at each decision point, copy the live state into an
    engine `Game`, try a handful of short input sequences (the explorer's bursts, `core::explore`, a
    small budget) toward the next few points, commit the first few frames of the best, repeat. When
    playing the original, the copy comes from the original's memory (`Start::Transplant`, as
    `oracle.rs` already does in its lockstep).

Both are executors: whatever they press is recorded frame by frame, so the recording can still be
judged by the faithful check exactly as today (same inputs to both machines). Reacting and checking
do not conflict; the experiment should confirm that on real runs.

## 2. Baseline (what exists, do not rebuild)

- `sf-plan` (`engine/crates/sf-plan`): explorer, guide paths, repair, shortening, original/app modes.
  Flags: `--app --snap --guide --follow --engine-only --seed --out --budget-min --frames --lag-limit
  --max-mem-gb --threads --no-pace --compass --shorten-share --via --avoid`.
- `sf-bench` (`engine/crates/sf-bench`): `verify` (the §4.1 verdict: `sf_conform::run` + the exit
  milestone), sets A/B/C, `committed_script(set, level)`.
- `trace.rs` / `core::repair::trace_engine`: a route's frog cell and coins left every 8 frames.
- Committed fixed routes for every normal level: `engine/conform/scripts/` (`complete-*`,
  `*-complete`, `enemies-*-complete`, `chain-*`), original-timed.
- Status at cut (tuning, `plans/smart-planner/bench/tuning/RESULTS.md`): set A 20/24 completed
  guided, M2 4/4, set B 13/24 of 23 run, 10/24 set-A routes lose no heart. T27 (unguided), T28
  (compass), T31 (leg starts) open.
- Read first: `plans/smart-planner/DESIGN.md` §2.4–§2.11, §4, §5; `plans/smart-planner/FINDINGS.md`
  top rows; `plans/smart-planner/spike-explore/README.md`.

## 3. What to measure

Run each executor and the baseline (the fixed script, replayed as is) on the same source routes under
the same perturbations. Source routes: the committed route of each level, and where it exists a
`sf-plan` route from tuning (re-plan if needed; `--seed 1`).

**Perturbations** (each is a real way timing shifts in this project):

| # | Condition | How |
|---|---|---|
| P0 | Its own timing | original-timed route on the original from `AtLevel` (baseline must pass) |
| P1 | Engine, no lag | the same route on the engine from `Start::Level` (no original slowdown) |
| P2 | App start | the engine from `Game::start_level` (set C's start, `sf-plan/src/app.rs`) |
| P3 | Late start | the start delayed by 1, 2, 5 and 20 idle frames |
| P4 | Lag bursts | on the engine, extra idle frames injected where the original lagged (frames the faithful check reports as mid-pass), or at fixed intervals |
| P5 | Difficulty | `opt_game_mode` (h0+$9e6) set to 0 (easy) and 2 (hard) at the start, otherwise P0/P1 |
| P6 | Set B leg | the level reached in the whole game (`--snap` leg snapshots under `~/.cache/superfrog-re/bench-legs/`) |

**Metrics per (executor, level, condition):**

- reached the exit (yes/no); frames to exit; hearts lost; lives lost;
- the recorded presses pass `verify` on the original (only for runs on the original);
- how far it got before failing (frames, and coins left) when it fails;
- cost: wall-clock and engine frames spent per frame played (B2's look-ahead is the expensive one),
  and memory;
- determinism: same inputs and seed → same recording, twice;
- size of the route artefact (bytes, steps) and whether a person can read or edit it;
- code size and complexity of the executor (rough lines), and what it would take to make it real.

**Levels:** start with a spread, then widen if the signal is clear: W1-1, W1-4, W2-1, W3-2, W5-2,
W6-1, W6-2, W4-3 (long busy pass), plus any level where a fixed script is known to break across
timings (bug-recorder FINDINGS T06: `complete-w1-1` ends in game over from `start_level`).

## 4. Rules

- The game, the engine's mechanics and the faithful check's tolerances are not changed (smart-planner
  DESIGN §1 Stance, §8). The executors only read state and choose inputs.
- **Do not disturb the live build.** The smart-planner run is mid-way and its counted runs (T16, T17)
  need a quiet machine (DESIGN D2). Before every heavy job: `pgrep -fl 'sf-bench|sf-plan|cargo'`; if a
  benchmark is running, wait or run small. At most one oracle and 10 threads at a time; each process
  under 4 GB (`--max-mem-gb`); every run time-boxed (a `--minutes`/frame cap, as `sf-plan`'s seatbelts).
- Prototypes are `examples/` in `sf-plan` or `sf-bench` (as `spike-explore/` did), not new crates or
  library changes. If one needs a small library hook, keep it on this branch and say so in the report.
- Determinism of the core (no clock or randomness except a passed seed) holds for anything that
  might become real; B2 must be repeatable for a seed.
- Commit as you go (small, `experiment(route-follower): …`), raw numbers in
  `experiments/route-follower/results/` (text/CSV), so a lost session loses nothing.
- Time box: about two working days of session time and machine time together. If a direction is
  clearly dead early (e.g. B1 cannot pass W1-1 under P1), record why and stop it.

## 5. Questions the report must answer

1. Under each perturbation, how often does each executor still reach the exit, against the fixed
   script? (the main table)
2. Does a recording made by A or B on the original pass the faithful check as reliably as a planned
   fixed route?
3. Can one route artefact serve both timings (original and app) and easy/hard, so DESIGN §2.6's
   "never crossed" rule could be relaxed for it? What would still break?
4. What does each cost: per-frame compute (could it run live in the app?), artefact size, how much
   of `sf-plan` it reuses, how much new code a real version needs?
5. How do A and B compare with simply re-planning or repairing with `sf-plan --follow` in the new
   condition (time to a working route)?
6. Which is better for this project's next steps (easy/hard completions, map-editor levels,
   recordings and drills in the app), and what would the first plan for it contain?

## 6. Deliverable

`experiments/route-follower/REPORT.md` on this branch, and a short reply to the owner:

- **For the owner, plain English** (the owner does not read code; see CLAUDE.md "Who you are talking
  to"): what was tried, the main table in words and numbers, a recommendation with its cost, and the
  options as a decision: A, B (which variant), both, or neither (keep fixed scripts).
- For the next session, technical: commands to reproduce every number, the prototypes' files, what
  surprised you, and what a plan would need first.
- What was not measured, and why.

Ask the owner only for a decision that cannot be measured (for example, if a perturbation needs a
judgement about what counts as "the same route"); everything else is measured, not asked.
