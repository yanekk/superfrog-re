# Implementation plan

11 tasks in 4 phases, from `plans/amiga-toolkit-prep/DRAFT-5-extract-toolkit.md` phase A as the
owner settled it at planning (DESIGN §7). Each has a file in [tasks/](tasks/) with its goal, files,
interface and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

## Shape of the build

The baseline goes first: T01 runs `sf-conform -- all` on its base before it changes anything, so
every later red can be compared with a recorded verdict. T01 then builds the workspace and moves the
three crates that are toolkit crates already, so every later move has a home, a purity test, a name
check and a test line to land in. The moves then run side by side, each one rewiring Superfrog to
the moved code with re-exports at the old paths, so no move touches another's call sites. The dry
fit and the whole-game runs come last, on the toolkit as built.

```
Phase 1  ▸  T01                     baseline, workspace, chips/clock/paula, Picture, checks
Phase 2  ▸  T02 T03 T04 T05 T07 T10   the moves, and the display piece
Phase 3  ▸  T06 T08 T09             what builds on the oracle and the app parts
Phase 4  ▸  T11                     dry fit, full runs, READMEs, north-star column
```

## Phase 1 — the ground

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-toolkit-workspace.md) | toolkit-workspace | — |

At the end: the baseline is recorded, `toolkit/` builds and tests on its own with three crates and
`Picture`, the name check and purity tests pass, the test line runs the toolkit.

## Phase 2 — the moves

| # | Task | Depends on |
|---|---|---|
| [T02](tasks/T02-amiga-input.md) | amiga-input | T01 |
| [T03](tasks/T03-amiga-disk.md) | amiga-disk | T01 |
| [T04](tasks/T04-asset-readers.md) | asset-readers | T03 |
| [T05](tasks/T05-amiga-oracle.md) | amiga-oracle | T02, T03, T04 |
| [T07](tasks/T07-amiga-app-parts.md) | amiga-app-parts | T02 |
| [T10](tasks/T10-display-piece.md) | display-piece | T01 |

At the end: every crate of DESIGN §2.2 except `amiga-conform`, `amiga-e2e` and `amiga-record` is in the toolkit,
Superfrog uses them, the oracle boots Kickstart with no disk.

## Phase 3 — built on the oracle and the app parts

| # | Task | Depends on |
|---|---|---|
| [T06](tasks/T06-amiga-conform.md) | amiga-conform | T05 |
| [T08](tasks/T08-amiga-e2e.md) | amiga-e2e | T07 |
| [T09](tasks/T09-amiga-record.md) | amiga-record | T03, T06, T07 |

## Phase 4 — prove it

| # | Task | Depends on |
|---|---|---|
| [T11](tasks/T11-dry-fit-and-finish.md) | dry-fit-and-finish | T04, T05, T08, T09, T10 |

At the end: the dry fit passes from toolkit crates alone, `sf-conform -- all` matches T01's
baseline, `whole_game.rs` passes, READMEs and the north-star column are written.

## Critical path

```
T01 → T03 → T04 → T05 → T06 → T09 → T11
```

T02, T07, T08 and T10 are off it. T04 is on it because the oracle's loader reads hunks with its
reader (DESIGN §2.2). T05 is on it because `amiga-conform` compares against the
oracle's `Rgb12Frame` and `FrameCapture`.

Leaves: T11 only.

## Parallel width

11 tasks · longest dependency chain 7 · up to 3 could run at once (T02, T03, T10; then T04, T07, T10).

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T01 (the 2 h baseline and the workspace), T05, T09, T10 (sf-core's Denise core moved), T11 (about 3.5 h of runs) |
| **Medium** | T03, T04, T06, T07, T08 |
| **Light** | T02 |

Overruns: T05 if the disk-list change shifts a trace key, and its vAmiga rebuild refills caches; T09
if the recording format cannot stay byte-identical behind the game header; T11 if the first full run
after T05's rebuild meets a time limit (run it in parts). Parallel moves all edit `purity.rs`'s lists,
`architecture.py`'s moved-path table, chapter `owns` lists and `sf-app/build.rs`'s `GAME`: small
merges, expected.

## Decisions still open

None block. Every disagreement between the draft and the code was settled at planning (DESIGN §7).
