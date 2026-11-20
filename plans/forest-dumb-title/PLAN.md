# Implementation plan

11 tasks in 4 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches, the
interfaces it defines, and what "done" means. Track state in [PROGRESS.md](PROGRESS.md). Read
[DESIGN.md](DESIGN.md) first.

## Shape of the build

- The read goes first. Every case, app test and doc names the codes and keys the title takes, and
  nobody has read them; T01 is the plan's riskiest unknown and gates T03 to T06 and T10.
- Headless before the app. The cases and fixes (phases 1 and 2) run on the oracle and the translated
  program alone; the app tests (phase 3) check the app equal to a headless run, so they do not wait on
  the fixes, but the drill does.
- The runner's goals (T02) and the recording format (T08) depend on nothing and start with the read.

```
Phase 0  ▸  T01, T02, T08        the read, the runner's goals, the recording format
Phase 1  ▸  T03 … T06            the title cases, recorded on the oracle
Phase 2  ▸  T07                  every title case passing translated
Phase 3  ▸  T09, T10, T11        Start Level…, the app's title paths, the drill
```

## Phase 0 — The read and the ground

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-title-read.md) | title-read | — |
| [T02](tasks/T02-case-goals.md) | case-goals | — |
| [T08](tasks/T08-world-start-recording.md) | world-start-recording | — |

At the end: the codes, keys and screens are in `docs/16-title.md`, each code proven on the oracle; a
case can say "back at the title" and "quit"; a recording can start at any world with the cheat.

## Phase 1 — The cases

| # | Task | Depends on |
|---|---|---|
| [T03](tasks/T03-title-idle-rows-cases.md) | title-idle-rows-cases | T01, T02 |
| [T04](tasks/T04-title-code-cases.md) | title-code-cases | T01, T02 |
| [T05](tasks/T05-title-quit-cases.md) | title-quit-cases | T01, T02 |
| [T06](tasks/T06-game-over-cases.md) | game-over-cases | T01, T02 |

At the end: every case of DESIGN §2.2 runs on the oracle and records its goal; its translated reading is
written in its notes, pass or first difference.

## Phase 2 — Fixing

| # | Task | Depends on |
|---|---|---|
| [T07](tasks/T07-title-fixes.md) | title-fixes | T03, T04, T05, T06 |

At the end: every title case passes on `--engine translated`; the 50 existing cases read as before.

## Phase 3 — The app

| # | Task | Depends on |
|---|---|---|
| [T09](tasks/T09-start-level-worlds.md) | start-level-worlds | T08 |
| [T10](tasks/T10-title-app-paths.md) | title-app-paths | T01 |
| [T11](tasks/T11-title-drill.md) | title-drill | T07, T09, T10 |

At the end: the title's paths and Start Level… are driven by key presses in the app, equal to headless;
the drill has used both at every size; docs and roadmap updated.

## Critical path

```
T01 → T03..T06 → T07 → T11
```

T02 runs beside T01. T08 → T09 and T10 are off the critical path and slot in wherever convenient.

Leaves: T11 only. T07, T09 and T10 feed it, so the drill runs on the fixed program.

## Parallel width

11 tasks · longest dependency chain 4 (T01 → T03 → T07 → T11) · up to 6 could run at once (T03, T04,
T05, T06, T09, T10).

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T07, T01 |
| **Medium** | T04, T06, T08, T09, T10 |
| **Light** | T02, T03, T05, T11 |

T07 is open-ended: nobody knows how many cases fail or why, and the OS's title interrupts (translator
DESIGN §2.8) are a likely cause the machine does not model. If it grows, its worker asks to split it
(a task added mid-run). T01 grows if the title has screens nobody planned for (DESIGN §2.1).

## Decisions still open

- Whether the high scores are saved to disk (`fd-core/src/game_over.rs` records that the original
  never saves them, owner 2026-11-01; the read confirms it), whether they take a name, and what Esc and a wrong code do
  on the original: settled by T01's read; a disk write goes to the owner before it is planned around.
  Blocks only T05 and T06's exact goals.
