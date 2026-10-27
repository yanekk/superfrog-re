# Implementation plan

2 tasks in 2 phases, as `plans/amiga-toolkit-prep/DRAFT-4-one-chip-model.md` splits them (its T1
and T2). Each has a file in [tasks/](tasks/) with its goal, the files it touches, the interfaces it
defines, and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

## Shape of the build

Merge first, move second. Merging inside sf-core keeps each change small enough that a red check
points at one merge; moving afterwards is then a change of paths only, checked by the same tests.
The one unknown, whether the level's per-modulo pointer mask differs from `Blit::run`'s start-only
mask (DESIGN §2.3), is T01's first commit rather than a separate spike, because only T01's own merge
depends on the answer.

```
Phase 1  ▸  T01   one copy of each, inside sf-core     rows 1–4
Phase 2  ▸  T02   the amiga-chips crate                row 5
```

## Phase 1 — one copy of each

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-one-copy-each.md) | one-copy-each | — |

At the end: `measure.sh` reads 1 / 3 / 1 / 2 for the blitter, BLTSIZE, `is_paula` and chip-memory
rows, everything still in sf-core, every check green.

## Phase 2 — the crate

| # | Task | Depends on |
|---|---|---|
| [T02](tasks/T02-amiga-chips-crate.md) | amiga-chips-crate | T01 |

At the end: `PASS amiga-chips` in `measure.sh`, the architecture docs own the new crate, and the
north-star log has the step's column.

## Critical path

```
T01 → T02
```

Leaves: T02 only, the final deliverable.

## Parallel width

2 tasks · longest dependency chain 2 · up to 1 could run at once. The plan is serial by nature: T02
moves what T01 merges.

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T01 |
| **Medium** | T02 |

T01 will overrun if the pointer-wrap test shows a difference (a `Bus`-only mask path in `Blit::run`)
or if a merged caller's `WriteLog` order shifts and a `render_*` case goes red. The draft estimates
300–450 lines in all.

## Decisions still open

None block. The owner settled the one contradiction found at planning (DESIGN §7, 2026-10-25).
