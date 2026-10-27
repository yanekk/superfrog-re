# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale tracker
costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The cell is an index; the account is the commit
message. Whoever writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-22 — 10 fixed, 4 decided with the user

**Status:** T00–T11 done; T12 implemented, awaiting review.
**Last updated:** 2026-10-27
**Next `pir-work` will:** review T12 wide-every-level.

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
| T12 | wide-every-level | T11 | 🔍 | 26 widescreen routes; `widescreen_complete.rs` (W1-1 in block, all 26 `--ignored`, green 140 s). W6-4 is owner-played (his choice). Deviations: app start, not `--engine-only` (test needs no oracle); WB-1 by `plan_bonus --widescreen`; WW-1 final-win's fire, Spud on; opt-in widescreen `handover.py`, `bug_trail`, `sf-routeview`. |

**Review queue:** T12

## Blocked on the user

Nothing.
