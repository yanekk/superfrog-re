# Implementation plan

3 tasks in 3 phases, as `plans/amiga-toolkit-prep/DRAFT-2-one-clock.md` splits them (its T1–T3).
Each has a file in [tasks/](tasks/) with its goal, the files it touches, the interfaces it defines,
and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

## Shape of the build

Type first, clock second, the coarse models last. `BeamPos` and the lengths are what every other
move is written in, so they land first and alone, and a red check then points at one kind of
change. The clock and the blit timing move next, proved against today's level unit tests and the
old functions kept as references. The front end and the slot draw go last because their fitted
constants depend on their rounding (draft "Risks"); by then the shared pieces are already known to
be the same pieces, and only the policies are new. T03 carries the full `sf-conform all` and the
whole-game app run, the only whole-game proof in the plan.

```
Phase 1  ▸  T01   amiga-clock with BeamPos and the lengths    M2 constants row
Phase 2  ▸  T02   generic Clock, blit timing, Timeline        M4/M5
Phase 3  ▸  T03   Cx and slot draw policies, full runs        M2 beam row, north-star column
```

## Phase 1 — one beam position, one set of lengths

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-beam-pos-and-constants.md) | beam-pos-and-constants | — |

At the end: `amiga-clock` exists with `BeamPos` and the lengths, `measure.sh` reads ≤ 3 for the
constants row and `PASS amiga-clock`, the copper's `Beam` is `BeamPos`, every check green.

## Phase 2 — the generic clock

| # | Task | Depends on |
|---|---|---|
| [T02](tasks/T02-generic-clock.md) | generic-clock | T01 |

At the end: the level runs on `amiga_clock::Clock<Level>`, all blit timing and `Timeline`'s
stepping live in `amiga-clock`, old save states load, every check green.

## Phase 3 — the coarse models onto the clock

| # | Task | Depends on |
|---|---|---|
| [T03](tasks/T03-front-end-and-slot-policies.md) | front-end-and-slot-policies | T02 |

At the end: `Cx` and the slot draw spend through named policies, `sf-conform all` and
`whole_game.rs` green, the north-star column, findings and the architecture README line written.

## Critical path

```
T01 → T02 → T03
```

Leaves: T03 only, the final deliverable.

## Parallel width

3 tasks · longest dependency chain 3 · up to 1 could run at once. Serial by nature: T02 builds the
clock on T01's type, and T03 puts the front end on T02's clock.

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T02, T03 (T03 mostly for its ~3.5 h of runs) |
| **Medium** | T01 |

T02 overruns if the generic `Clock` changes a save state's bytes or a level unit test, or if
`Timeline`'s split shifts a restart write. T03 overruns if a policy rounds one input differently
and a front-end case goes red (draft "Risks"). The draft estimates 300–600 lines in all.

## Decisions still open

None block. The owner settled the two places where the code had moved on from the draft
(DESIGN §7, 2026-10-27).
