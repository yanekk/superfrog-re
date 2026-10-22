# Implementation plan

14 tasks in 4 phases. Each has a file in [tasks/](tasks/) with its goal, the files it touches, the
interfaces it defines, and what "done" means.

Track state in [PROGRESS.md](PROGRESS.md). Read [DESIGN.md](DESIGN.md) first.

---

## Shape of the build

- The riskiest unknown goes first, as a spike: whether a recording made in the unpaced app can be
  played on the lagging original from a matching start (DESIGN §2.9, D2). Everything after T00 rests
  on its answer.
- Everything testable headlessly is built before any window: the format, the start, the original's
  start, the replay and compare tools (Phase 1). The app then wires its surfaces to machinery already
  proven against the oracle.
- The app's recorder is built and tested through script events (T06) before its windows, so each
  window task (T08–T10) only adds a view over a model that already works.

```
Phase 0  ▸  T00                      prove the ground           throwaway
Phase 1  ▸  T01 … T05                format, starts, agent tools headless, against the oracle
Phase 2  ▸  T06 … T11                the app: recorder, replay, windows, drill
Phase 3  ▸  T12, T13                 the fix procedure, the acceptance bug
```

---

## Phase 0 — Prove the ground

| # | Task | Depends on |
|---|---|---|
| [T00](tasks/T00-prove-the-ground.md) | prove-the-ground | — |

**T00 gates** DESIGN §2.9 (the retiming rule, or the per-leg fallback) and D2 (which level start,
and so what `seed` in the recording means). Throwaway code, deleted afterwards; its answers go in
`FINDINGS.md` and, where they settle D2 or §2.9, in an amendment note on T01–T03.

## Phase 1 — Headless machinery

| # | Task | Depends on |
|---|---|---|
| [T01](tasks/T01-recording-format.md) | recording-format | T00 |
| [T02](tasks/T02-level-start.md) | level-start | T01 |
| [T03](tasks/T03-original-start.md) | original-start | T02 |
| [T04](tasks/T04-replay-tool.md) | replay-tool | T01, T02 |
| [T05](tasks/T05-compare-tool.md) | compare-tool | T03, T04 |

At the end: an agent can take a recording text (made by a test, not yet the app), replay it
headlessly, play it on the original and turn it into a case.

## Phase 2 — The app

| # | Task | Depends on |
|---|---|---|
| [T06](tasks/T06-app-recorder.md) | app-recorder | T02 |
| [T07](tasks/T07-app-replay.md) | app-replay | T06 |
| [T08](tasks/T08-start-level-dialog.md) | start-level-dialog | T06 |
| [T09](tasks/T09-recording-bar.md) | recording-bar | T06 |
| [T10](tasks/T10-save-window.md) | save-window | T06 |
| [T11](tasks/T11-recorder-drill.md) | recorder-drill | T04, T07, T08, T09, T10 |

At the end: the owner can start a level from the dialog, record, STOP, save a zip, and anyone can
`--replay` it. The `sf-e2e` script layer and OS layer cover it; the drill has used it whole.

## Phase 3 — The fix flow

| # | Task | Depends on |
|---|---|---|
| [T12](tasks/T12-fix-procedure.md) | fix-procedure | T05, T11 |
| [T13](tasks/T13-black-screen.md) | black-screen | T12 |
| [T14](tasks/T14-original-follows-recording.md) | original-follows-recording | T05, T12; blocks T13 |

At the end: the acceptance bug has been recorded by the owner and taken through `docs/BUG-FIX.md`.

---

## Critical path

```
T00 → T01 → T02 → T06 → T08 → T11 → T12 → T13
```

T03 → T05 runs beside the app phase and joins at T12; T04 can start as soon as T02 is in. T07, T09
and T10 are as long as T08 and any of them may be the one on the path.

Leaves: T13 only.

## Parallel width

14 tasks · longest dependency chain 8 · up to 6 could run at once (T03, T04, T07, T08, T09, T10).

## Rough sizing

| Weight | Tasks |
|---|---|
| **Heavy** | T00, T05, T13 |
| **Medium** | T02, T03, T06, T08, T09, T10, T11 |
| **Light** | T01, T04, T07, T12 |

T00 may overrun if neither start matches the original at the transition; T05 if the retiming needs
the per-leg fallback. T13 is unknown by nature: the fix is wherever the bug is.

## Decisions still open

- D2 and §2.9 are open until T00; nothing after T00 starts before it.
- Whether a bug the replay shows but the paced comparison does not (§2.10 step 7) needs its own
  kind of permanent test. Settled when one is first met, with the owner; it does not block.
