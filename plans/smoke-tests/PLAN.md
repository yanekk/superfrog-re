# Implementation plan

8 tasks in 3 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches, the
interfaces it defines, and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

---

## Shape of the build

- **The machinery is built and tested on synthetic inputs before any real test is labelled**, so choosing
  labels happens against a check that already works.
- **Measure before choosing.** T03 gives measured times, each file capped at 11 s, so labels do not rest on
  rough past timings and the pass never amounts to the full set.
- **The switch comes last.** Stored lines and directives change only once smoke is shown to fit.

```
Phase 1  ▸  T01 T02 T03 T04   machinery      no real labels yet
Phase 2  ▸  T05 T06 T07       the selection  smoke measured, fits
Phase 3  ▸  T08               the switch     new plans use smoke
```

---

## Phase 1 — The machinery

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-label-check.md) | label-check | — |
| [T02](tasks/T02-smoke-runner.md) | smoke-runner | T01 |
| [T03](tasks/T03-file-timings.md) | file-timings | T01 |
| [T04](tasks/T04-full-command.md) | full-command | — |

At the end, both commands exist and are proven on fakes, and every test file has a measured time or a
"more than 11 s".

## Phase 2 — The selection

| # | Task | Depends on |
|---|---|---|
| [T05](tasks/T05-toolkit-and-forest-labels.md) | toolkit-and-forest-labels | T03 |
| [T06](tasks/T06-superfrog-labels.md) | superfrog-labels | T03 |
| [T07](tasks/T07-smoke-fits.md) | smoke-fits | T02, T05, T06 |

At the end, `tools/smoke.sh` runs for real, green, in 15 minutes or less.

## Phase 3 — The switch

| # | Task | Depends on |
|---|---|---|
| [T08](tasks/T08-switch-and-directives.md) | switch-and-directives | T04, T07 |

At the end, new plans take smoke as their test line and agents are told to keep it growing.

---

## Main path: builder and wirer

| Step | Built by | Wired by |
|---|---|---|
| A build finishes and pir runs the test line | pir, unchanged | this plan's DESIGN block; T08 for new plans (`.pir/settings.json`) |
| The check runs first | T01 | T02 (`tools/smoke.py` calls it) |
| Labelled tests run under the clock | T02, with labels from T05, T06 | T02 |
| The nightly schedule calls full | T04 | the owner, outside this plan |
| A later agent keeps smoke growing | T08 | root, project and template `CLAUDE.md`, T08 |

## Critical path

```
T01 → T03 → T05/T06 → T07 → T08
```

T02 and T04 are off it. Leaves: only T08, the final deliverable.

## Parallel width

8 tasks · longest dependency chain 5 · up to 4 could run at once (T02, T04, T05, T06).

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T06, T07 |
| **Medium** | T01, T02, T05 |
| **Light** | T03, T04, T08 |

T07 is where this may overrun: it holds the real measurements, including a cold compile, and may need the
owner if crucial tests alone do not fit.

## Decisions still open

None blocks. If crucial tests alone exceed 15 minutes, T07 asks the owner what to drop or swap.
