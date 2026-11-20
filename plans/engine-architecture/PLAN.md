# Implementation plan

18 tasks in 3 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches,
what its chapter must contain, and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

---

## Shape of the build

- The skeleton comes first. T01 fixes the chapter list, which code each chapter owns, the shared
  section layout and the glossary of mechanisms, and lands the drift check green with every
  chapter a stub. After that every chapter task edits one file only, so they can all run at once.
- The mechanism chapters (T02–T07) and the subsystem chapters (T08–T14) run in parallel rather
  than mechanisms first. A subsystem chapter names the mechanisms it uses by glossary term and
  links to their chapter; it does not need that chapter's content, and the glossary keeps the
  words consistent.
- The overview comes last, because its tables and totals are computed from every chapter's front
  matter and its rewrite order needs every mechanism chapter's dependencies.

```
Phase 1  ▸  T01                 skeleton, glossary, drift check
Phase 2  ▸  T02 … T17           one chapter each (T07 writes two), all in parallel
Phase 3  ▸  T18                 overview, totals, rewrite order, check made strict
```

There is no spike: nothing here rests on an unmeasured technical assumption. The test command
was measured on a fresh worktree (DESIGN §5).

---

## Phase 1 — Skeleton

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-skeleton-and-check.md) | skeleton-and-check | — |

At the end: `docs/architecture/` exists with a stub per chapter whose front matter owns every
tracked engine file, the glossary and template are written, and the drift check is green.

## Phase 2 — Chapters

| # | Task | Depends on |
|---|---|---|
| [T02](tasks/T02-memory-model.md) | memory-model | T01 |
| [T03](tasks/T03-copied-in-data.md) | copied-in-data | T01 |
| [T04](tasks/T04-cpu-time-and-beam.md) | cpu-time-and-beam | T01 |
| [T05](tasks/T05-frame-interrupts-input.md) | frame-interrupts-input | T01 |
| [T06](tasks/T06-display-hardware.md) | display-hardware | T01 |
| [T07](tasks/T07-sound.md) | sound | T01 |
| [T08](tasks/T08-game-flow.md) | game-flow | T01 |
| [T09](tasks/T09-front-end.md) | front-end | T01 |
| [T10](tasks/T10-story-and-ending.md) | story-and-ending | T01 |
| [T11](tasks/T11-level.md) | level | T01 |
| [T12](tasks/T12-frog-and-items.md) | frog-and-items | T01 |
| [T13](tasks/T13-enemies-and-bosses.md) | enemies-and-bosses | T01 |
| [T14](tasks/T14-slot-and-bonus.md) | slot-and-bonus | T01 |
| [T15](tasks/T15-disks-and-assets.md) | disks-and-assets | T01 |
| [T16](tasks/T16-app.md) | app | T01 |
| [T17](tasks/T17-verification.md) | verification | T01 |

At the end: every chapter is written, classified and rated; no `tbd` remains in any front matter.

## Phase 3 — Overview

| # | Task | Depends on |
|---|---|---|
| [T18](tasks/T18-overview-and-summary.md) | overview-and-summary | T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T12, T13, T14, T15, T16, T17 |

At the end: the overview is complete, the repository README links to it, and the drift check
refuses `tbd`.

---

## Critical path

```
T01 → (the slowest of T09, T10, T11, T13, T04) → T18
```

Every phase 2 task is off the critical path except whichever finishes last.

Leaves: T18 only. Every chapter task feeds it.

## Parallel width

18 tasks · longest dependency chain 3 · up to 16 could run at once.

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T04 (cycle charging is spread over many files), T09 (front end, ~7k lines), T10 (story, ~6k lines with its own bus simulation), T11 (level, ~6k lines), T13 (enemies, ~5.8k lines) |
| **Medium** | T01, T02, T03, T05, T06, T07, T08, T14, T17, T18 |
| **Light** | T12, T15, T16 |

T03 and T04 are the likeliest to overrun: copied-in data and cycle charging are spread across
files other chapters own, so both must search by behaviour (`Rom`, `h0+$`, `spend`, `CODE_TIMES`)
rather than read a folder.

## Decisions still open

None block. T18 settles any `owns` moves the chapter tasks log in `FINDINGS.md` (DESIGN §2.7).
