# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Building; T00 awaiting review.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T00; T03 (chrome-rig) heads the critical path, T01, T06 and T07 beside it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | items-needed-source | — | 🔍 | Found: every load list sets items_left to BCD(coins in the whole 38400-byte object map - 50), a pure function of `_MapaBobowN`. Read on the oracle for all six worlds, equal to the file count; docs/10 Items and a FINDINGS row. Spike deleted, README kept with rerun steps (code in 34b06ade). No tests: research task. No deviations. |
| T01 | objects-all-worlds | — | ⬜ | |
| T02 | map-levels | T00, T01 | ⬜ | |
| T03 | chrome-rig | — | ⬜ | |
| T04 | page-map-only | T03 | ⬜ | |
| T05 | page-live | T04 | ⬜ | |
| T06 | live-reader | — | ⬜ | |
| T07 | secrets-research | — | ⬜ | |
| T08 | map-command | T02, T05 | ⬜ | |
| T09 | secrets-layer | T07, T08 | ⬜ | |
| T10 | app-live | T02, T05, T06 | ⬜ | |
| T11 | app-show-map | T08, T10 | ⬜ | |
| T12 | map-drill | T09, T11 | ⬜ | |

**Review queue:** T00

## Blocked on the user

Nothing.
