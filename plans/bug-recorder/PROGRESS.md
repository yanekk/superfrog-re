# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale
tracker costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**; read the rows touching
the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message. Whoever
writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-20 — 4 fixed, 4 decided with the user

**Status:** Every task reviewed ✅ (T13 2026-10-21).
**Last updated:** 2026-10-21
**Next `pir-work` will:** nothing; the plan's tasks are done. The black screen stays unreproduced.

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
| T09 | recording-bar | T06 | ✅ | |
| T10 | save-window | T06 | ✅ | |
| T11 | recorder-drill | T04, T07, T08, T09, T10 | ✅ | |
| T12 | fix-procedure | T05, T11 | ✅ | |
| T13 | black-screen | T12 | ✅ | Owner's zip `bug-W1-2-2026-10-21-0731.zip`: black screen absent, owner's note asked confirm only, so no fix or quirk report. Review reran replay (equal, 103 black frames 5393–5495) and compare (`same`); ran the full workspace suite (owner's yes): green but an open `smoke.rs` flake, fails on base too. No code, no mutation. |
| T14 | original-follows-recording | T05, T12; blocks T13 | ✅ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing. The owner recorded T13's zip on 2026-10-21; the black screen did not appear.
