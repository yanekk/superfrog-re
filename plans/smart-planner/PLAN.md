# Implementation plan

19 tasks in 4 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches, the
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

```
Phase 0  ▸  T00 T01 T02        prove the ground, build the instrument, sample the baseline
Phase 1  ▸  T03 … T09          planner core, engine only
Phase 2  ▸  T10 T11 T12        original mode, the one command, app mode
Phase 3  ▸  T13 … T18          soundness, tuning, counted runs, verdict, retire old tools
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

**T02 decides** whether T10 uses long-stretch confirmation with repair (DESIGN §2.6).

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

At the end of Phase 1: the core plans a level on the engine from any start state, deterministically.

## Phase 2 — One command

| # | Task | Depends on |
|---|---|---|
| [T10](tasks/T10-oracle-check.md) | oracle-check | T02, T08, T09 |
| [T11](tasks/T11-plan-command.md) | plan-command | T05, T10 |
| [T12](tasks/T12-app-timing.md) | app-timing | T11 |

T12 also needs `pir/bug-recorder` merged into main (DESIGN D3); it checks and stops to ask if not.

At the end of Phase 2: `sf-plan` is the one command, in original and app modes, wired into `sf-bench`.

## Phase 3 — Proving it

| # | Task | Depends on |
|---|---|---|
| [T13](tasks/T13-map-soundness.md) | map-soundness | T01, T08 |
| [T14](tasks/T14-tuning-run.md) | tuning-run | T11 |
| [T15](tasks/T15-set-c-run.md) | set-c-run | T12, T13, T14 |
| [T16](tasks/T16-baseline-counted.md) | baseline-counted | T15 |
| [T17](tasks/T17-new-counted.md) | new-counted | T13, T16 |
| [T18](tasks/T18-results-and-retire.md) | results-and-retire | T17 |

T15 depends on T13 and T16 on T15 only so that they run on a quiet machine after every other task
(DESIGN D2, D4): T13's explorer fills every core for hours. T17 follows T16 for the same reason.

---

## Main path: who builds, who wires

| Step | Built by | Wired by |
|---|---|---|
| Agent runs `sf-plan <level>` | T11 (bin `sf-plan`) | T11 (`engine/crates/sf-plan/Cargo.toml` `[[bin]]`) |
| Start from AtLevel / `--snap` | T11 | T11 `main.rs` |
| Start from `start_level` (`--app`) | T12 | T12 `main.rs` |
| Level facts | T03 | T04/T08 call it; T11 `main.rs` constructs |
| Movement map | T04 | T11 `main.rs` via `cache::load_or_build` (T05) |
| Live danger check | T07 | T08 `search` (every edge through `danger::play_safe`) |
| Coin choice | T06 | T08 `search::Planner::new` and re-choice |
| Search | T08 | T10 (original mode), T11 `main.rs` (app/engine) |
| Route repair `--follow` | T09 | T11 `main.rs` flag; T10 for stretch confirmation |
| Oracle confirmation | T10 | T11 `main.rs` |
| Benchmark runs the new planner | T01 (harness) | T11 (adapter `new`) |
| Old tools retired | T18 | T18 |

## Critical path

```
T00 → T03 → T04 → T08 → T09 → T10 → T11 → T14 → T15 → T16 → T17 → T18
```

Off it: T01, T02 (finish long before T10), T05, T06, T07, T12 and T13 (before T15).

Leaves: T18 only.

## Parallel width

19 tasks · longest dependency chain 12 · up to 4 could run at once (e.g. T02, T04, T06, T07). The
tail T14–T18 is serial on purpose: counted runs need a quiet machine.

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T00, T04, T08, T14, T16, T17 (the last two in machine hours, up to ~25 h each) |
| **Medium** | T01, T02, T09, T10, T11, T13, T15, T18 |
| **Light** | T03, T05, T06, T07, T12 |

Overrun risks: T04 if moving platforms and pads need many move variants, or speed bands (D15) multiply
the move count; T08, which grew by exploring when stuck, learned and unreliable edges (D16, D17); T14 is open-ended by nature
(tuning until the stall levels pass), bounded by D5: what is not clear-cut goes to the owner; T16/T17
are wall-clock bound and run as resumable background slices of at most 2 h each.

## Decisions still open

- Map seeding, enemy freezing and speed bands (D15): T00 settles them; nothing after T03 starts before.
- Stretch confirmation: T02 settles it before T10.
- Whether `pir/bug-recorder` is merged by the time T12 is ready: T12 checks; if not, T12 and T15
  wait and the owner is asked. T16, T17 and T18 wait with them, being chained after T15 for a quiet
  machine (owner, 2026-10-20: keep the order; reorder then if the merge is late).
