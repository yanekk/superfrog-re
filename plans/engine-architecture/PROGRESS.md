# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale
tracker costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**; read the rows touching
the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message. Whoever
writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-22 — 2 fixed, 1 decided with the user

**Status:** Every chapter ✅; T18 (overview) implemented, awaiting review.
**Last updated:** 2026-10-23
**Next `pir-work` will:** review T18.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | skeleton-and-check | — | ✅ | |
| T02 | memory-model | T01 | ✅ | |
| T03 | copied-in-data | T01 | ✅ | |
| T04 | cpu-time-and-beam | T01 | ✅ | |
| T05 | frame-interrupts-input | T01 | ✅ | |
| T06 | display-hardware | T01 | ✅ | |
| T07 | sound | T01 | ✅ | |
| T08 | game-flow | T01 | ✅ | |
| T09 | front-end | T01 | ✅ | |
| T10 | story-and-ending | T01 | ✅ | |
| T11 | level | T01 | ✅ | |
| T12 | frog-and-items | T01 | ✅ | |
| T13 | enemies-and-bosses | T01 | ✅ | |
| T14 | slot-and-bonus | T01 | ✅ | |
| T15 | disks-and-assets | T01 | ✅ | |
| T16 | app | T01 | ✅ | |
| T17 | verification | T01 | ✅ | |
| T18 | overview-and-summary | T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T12, T13, T14, T15, T16, T17 | 🔍 | Overview: diagram, subsystem-by-mechanism table, totals with verification's split, rewrite order, open questions. `ALLOW_TBD = False`; README linked; no `owns` moved (FINDINGS). No test added. Deviations: overview table checked once against `totals`, not tested (line counts churn; light check §2.4). Subsystem links are a table, not diagram edges. |

**Review queue:** T18

## Blocked on the user

Nothing.
