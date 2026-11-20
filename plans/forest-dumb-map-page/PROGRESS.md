# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Building; T00 done.
**Last updated:** 2026-11-20
**Next `pir-work` will:** T03 (chrome-rig), the head of the critical path; T01, T06 and T07 beside it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | items-needed-source | — | ✅ | Reviewed: items_left = BCD(object-map coins - 50), set by all six load lists. Reran the oracle reading and map count; read the counter, packer and every items_left writer. Fix: docs/10 notes no coins in unused columns, README rerun needs the venv. Docs-only, no mutation. |
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

**Review queue:** (empty)

## Blocked on the user

Nothing.
