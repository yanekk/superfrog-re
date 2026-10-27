# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale tracker
costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The cell is an index; the account is the commit
message. Whoever writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-22 — 10 fixed, 4 decided with the user

**Status:** T00–T12 done.
**Last updated:** 2026-10-27
**Next `pir-work` will:** nothing; every task is done.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | wide-spawn-spike | — | ✅ | |
| T01 | wide-mode | — | ✅ | |
| T02 | wide-spawn | T01 | ✅ | |
| T03 | wide-capacity | T00, T01, T02 | ✅ | |
| T04 | wide-camera | T01 | ✅ | |
| T05 | wide-special-cases | T01 | ✅ | |
| T06 | wide-switch | T02, T03 | ✅ | |
| T07 | wide-recording | T01 | ✅ | |
| T08 | side-objects | T02, T03 | ✅ | |
| T09 | side-frog | T04, T08 | ✅ | |
| T10 | app-wiring | T05, T06, T07, T08, T09 | ✅ | |
| T11 | widescreen-drill | T10 | ✅ | |
| T12 | wide-every-level | T11 | ✅ | Review: 26 routes replay green (all, 120 s). Fixed: in-block test was W1-1, which also finishes in 4:3, so dropping widescreen survived; now W5-1 asserts wide finishes, 4:3 dies. `handover.py --step 0` now refused. Mutated wide start (sf-plan, replay), probed WW-1 without Spud (fails). W6-4 owner-played, his choice. |

**Review queue:** empty

## Blocked on the user

Nothing.
