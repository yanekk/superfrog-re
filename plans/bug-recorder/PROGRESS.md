# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale
tracker costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**; read the rows touching
the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message. Whoever
writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-20 — 4 fixed, 4 decided with the user

**Status:** Plan written 2026-10-20; nothing built.
**Last updated:** 2026-10-20
**Next `pir-work` will:** T00, the spike every other task waits on.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | prove-the-ground | — | ✅ | |
| T01 | recording-format | T00 | ✅ | |
| T02 | level-start | T01 | ✅ | |
| T03 | original-start | T02 | ✅ | |
| T04 | replay-tool | T01, T02 | ✅ | |
| T05 | compare-tool | T03, T04 | ✅ | |
| T06 | app-recorder | T02 | ✅ | |
| T07 | app-replay | T06 | ✅ | |
| T08 | start-level-dialog | T06 | ✅ | |
| T09 | recording-bar | T06 | ✅ | Review clean, no fix commit. Script layer and unit tests green; mutations of `picture_area` and the shrink on STOP both caught. OS layer unverified by review: screen locked, every AX test fails including base `controls.rs`; owner chose to skip, rests on the builder's green run 2026-10-20. |
| T10 | save-window | T06 | ⬜ | |
| T11 | recorder-drill | T04, T07, T08, T09, T10 | ⬜ | |
| T12 | fix-procedure | T05, T11 | ⬜ | |
| T13 | black-screen | T12 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing yet. T13 will need the owner to record the black-screen bug in the app.
