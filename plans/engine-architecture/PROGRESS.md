# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale
tracker costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**; read the rows touching
the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message. Whoever
writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-22 — 2 fixed, 1 decided with the user

**Status:** Plan written 2026-10-22; nothing built.
**Last updated:** 2026-10-23
**Next `pir-work` will:** T18, now that every chapter is ✅.

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
| T17 | verification | T01 | ✅ | Reviewed: one fix, the beam-race row wrongly said no test plants a non-raced difference (`compare.rs` unit test does; false finding dropped). Probed every line count against `tools/architecture.py`, all 22 cited tests exist, patch hooks, 13 `FrameState` variables; mutated `raced_only` (red) and a cited path (red). Mix 4% amiga estimated, not counted. |
| T18 | overview-and-summary | T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T12, T13, T14, T15, T16, T17 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
