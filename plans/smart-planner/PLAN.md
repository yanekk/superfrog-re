# Implementation plan

21 tasks in 4 phases (amended 2026-10-22: T13 and T20 dropped, T19, T21, T22 added since the plan was
written). Each has a file in [tasks/](tasks/) with its goal, the files it touches, the
interfaces it defines, and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

---

## Shape of the build

- The riskiest unknown goes first, as a spike. Whether a measured movement map can be built in minutes
  and is right where today's field was wrong decides the whole design; T00 answers it on W1-4 and W6-3
  before anything is built on the map.
- The measuring instrument comes before the thing measured. The benchmark (T01) and an early sample of
  today's tools (T02) exist before the planner, so design choices (D6's conditional technique) rest on
  numbers.
- Everything engine-only is built and tested before the original is involved. Phase 1 is the core on
  `sf-core` alone, tested in milliseconds; Phase 2 adds the oracle and the one command.
- The counted runs come last and run serially on a quiet machine (DESIGN D2), so their timings are
  comparable. That tail is serial by nature, not by accident.
- Amendment 2026-10-22 (DESIGN D19–D24): the map approach was proven partly wrong by its own build
  (T04: 13 of 24 levels reach the exit). The core is now a whole-level explorer (T08), steered by
  guide paths (T09), confirmed on the original by stretch and repair (T10), its routes shortened
  (T22). T04/T05/T06/T07 stay as built; the map and coin choice are compasses T14 may adopt.

```
Phase 0  ▸  T00 T01 T02        prove the ground, build the instrument, sample the baseline
Phase 1  ▸  T03 … T09, T19, T21, T22   planner core, engine only
Phase 2  ▸  T10 T11 T12        original mode, the one command, app mode
Phase 3  ▸  T14 … T18          tuning, counted runs, verdict, retire old tools
```

---

## Phase 0 — Prove the ground

| # | Task | Depends on |
|---|---|---|
| [T00](tasks/T00-movement-map-spike.md) | movement-map-spike | — |
| [T01](tasks/T01-bench-harness.md) | bench-harness | — |
| [T02](tasks/T02-baseline-sample.md) | baseline-sample | T01 |

**T00 gates** DESIGN §2.2's open choices: how the map is seeded (frog placed by poke vs cells reached
by real moves), how enemies and moving platforms are held while measuring, and whether map build time
fits the budget. If the map cannot be built in under ~5 minutes per level on 10 threads, or misjudges
the recorded stall spots, the plan goes back to the owner before T03. It is throwaway and deleted.
Since the 2026-10-21 amendment it also settles whether map nodes carry speed (D15), confirms that
enemies spawn only as the screen reaches them (D14), and measures how often map moves fail in live
play (D17).

**T02 decided** T10's confirmation for the old map design (48-frame chunks); since D24 T10 confirms by
stretch and repair regardless (DESIGN §2.6).

At the end of Phase 0: the map approach is proven or refuted, `sf-bench` exists, and today's tools have
early numbers on the stall levels.

## Phase 1 — Planner core, engine only

| # | Task | Depends on |
|---|---|---|
| [T03](tasks/T03-level-facts.md) | level-facts | T00 |
| [T04](tasks/T04-movement-map.md) | movement-map | T03 |
| [T05](tasks/T05-map-cache.md) | map-cache | T04 |
| [T06](tasks/T06-coin-choice.md) | coin-choice | T03 |
| [T07](tasks/T07-danger-map.md) | danger-map | T03 |
| [T08](tasks/T08-search.md) | search | T04, T06, T07 |
| [T09](tasks/T09-route-repair.md) | route-repair | T08 |
| [T19](tasks/T19-step-unseen-no-picture.md) | step-unseen-no-picture | T00; blocks T04 |
| [T21](tasks/T21-engine-memory-lookup.md) | engine-memory-lookup | T04 |
| [T22](tasks/T22-route-shorten.md) | route-shorten | T08 |

At the end of Phase 1: the core plans a level on the engine from any start state, deterministically.

## Phase 2 — One command

| # | Task | Depends on |
|---|---|---|
| [T10](tasks/T10-oracle-check.md) | oracle-check | T02, T08, T09 |
| [T11](tasks/T11-plan-command.md) | plan-command | T05, T10, T22 |
| [T12](tasks/T12-app-timing.md) | app-timing | T11 |
| [T23](tasks/T23-w1-4-ending.md) | w1-4-ending | T09, T10; blocks T14 |

T12 also needs `pir/bug-recorder` merged into main (DESIGN D3); it checks and stops to ask if not.

At the end of Phase 2: `sf-plan` is the one command, in original and app modes, wired into `sf-bench`.

## Phase 3 — Proving it

| # | Task | Depends on |
|---|---|---|
| [T14](tasks/T14-tuning-run.md) | tuning-run | T11 |
| [T15](tasks/T15-set-c-run.md) | set-c-run | T12, T14 |
| [T16](tasks/T16-baseline-counted.md) | baseline-counted | T15 |
| [T17](tasks/T17-new-counted.md) | new-counted | T16 |
| [T18](tasks/T18-results-and-retire.md) | results-and-retire | T17 |
| [T24](tasks/T24-repair-cost.md) | repair-cost | T14; blocks T15 |
| [T25](tasks/T25-tuning-set-a.md) | tuning-set-a | T14, T24; blocks T15 |
| [T26](tasks/T26-tuning-set-b.md) | tuning-set-b | T14, T25; blocks T15 |
| [T27](tasks/T27-tuning-unaided.md) | tuning-unaided | T14, T26; blocks T15 |
| [T28](tasks/T28-compass-trial.md) | compass-trial | T14, T27; blocks T15 |
| [T29](tasks/T29-engine-sound-ramp.md) | engine-sound-ramp | T25; blocks T15 |
| [T30](tasks/T30-lag-repair.md) | lag-repair | T25; blocks T15 |
| [T31](tasks/T31-leg-start-sync.md) | leg-start-sync | T26; blocks T17 |

T16 depends on T15 only so that it runs on a quiet machine after every other task (DESIGN D2, D4);
T17 follows T16 for the same reason. (T13, M7's map-soundness run, was dropped 2026-10-22, D20.)

---

## Main path: who builds, who wires

| Step | Built by | Wired by |
|---|---|---|
| Agent runs `sf-plan <level>` | T11 (bin `sf-plan`) | T11 (`engine/crates/sf-plan/Cargo.toml` `[[bin]]`) |
| Start from AtLevel / `--snap` | T11 | T11 `main.rs` |
| Start from `start_level` (`--app`) | T12 | T12 `main.rs` |
| Level facts | T03 | T04/T08 call it; T11 `main.rs` constructs |
| Movement map (compass, optional) | T04, `Compass::Map` T08 | T11 `--compass map` via `cache::load_or_build` (T05); adopted or not by T14 |
| Coin choice (compass, optional) | T06 | T14, only if its trial keeps it |
| Live danger check | T07 | not wired (DESIGN §2.5) |
| Explorer | T08 | T10 (original mode), T11 `main.rs` (app/engine) |
| Guide paths `--guide`, repair `--follow` | T09 | T11 `main.rs` flags; T10 for each repair; T11's `sf-bench` adapter for guided runs |
| Shortening | T22 | T08's `run.rs` after the first exit; T11 options |
| Oracle confirmation | T10 | T11 `main.rs` |
| Benchmark runs the new planner | T01 (harness) | T11 (adapter `new`) |
| Old tools retired | T18 | T18 |

## Critical path

```
T08 → T09 → T10 → T11 → T14 → T15 → T16 → T17 → T18
```

(T00–T07, T19, T21 are done.) Off it: T22 (beside T09 and T10, before T11) and T12 (before T15).
T08 declares every module and the `plan_engine`/`RunOptions` signature T09, T10 and T22 build on, so
those three never edit the same file: T09 owns `core/guide.rs`, `core/repair.rs`, `trace.rs`,
`guide_file.rs`, `Cargo.toml`; T22 `core/shorten.rs` and the shortening call in `run.rs`; T10
`oracle.rs` and its `lib.rs` line.

Leaves: T18 only.

## Parallel width

21 tasks, 11 left · longest remaining chain 9 · up to 2 could run at once now (T09 and T22 after
T08). The tail T14–T18 is serial on purpose: counted runs need a quiet machine.

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T00, T04, T08, T14, T16, T17 (the last two in machine hours, up to ~25 h each) |
| **Medium** | T01, T02, T09, T10, T11, T15, T18, T22 |
| **Light** | T03, T05, T06, T07, T12 |

Overrun risks: T08's memory policy (D22: checkpoints and replay must stay deterministic and fast);
W2-3 and W6-4, which the experiment never finished, land in T14; T14 is open-ended by nature
(tuning until the stall levels pass), bounded by D5: what is not clear-cut goes to the owner; T16/T17
are wall-clock bound and run as resumable background slices of at most 2 h each.

## Decisions still open

- The compass (straight, the map, coin choice): T14's trial settles it (D20).
- Whether `pir/bug-recorder` is merged by the time T12 is ready: T12 checks; if not, T12 and T15
  wait and the owner is asked. T16, T17 and T18 wait with them, being chained after T15 for a quiet
  machine (owner, 2026-10-20: keep the order; reorder then if the merge is late).
