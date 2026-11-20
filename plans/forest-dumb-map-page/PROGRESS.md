# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Building; T06 done.
**Last updated:** 2026-11-20
**Next `pir-work` will:** T03 (chrome-rig), the head of the critical path; T00, T01 and T07 can run beside it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | items-needed-source | — | ⬜ | |
| T01 | objects-all-worlds | — | ⬜ | |
| T02 | map-levels | T00, T01 | ⬜ | |
| T03 | chrome-rig | — | ⬜ | |
| T04 | page-map-only | T03 | ⬜ | |
| T05 | page-live | T04 | ⬜ | |
| T06 | live-reader | — | ✅ | Clean, no fix commit. Ran the 6 made-up-RAM tests, purity and all 4 headless tests green. Mutations caught: play rule, column/row swap, hand-side creature kind. Probed: world 2's first pass has 0 changed cells, all six level maps 38 400 bytes; a lone Game ends a pass and a pixel off (irq 601 vs 600), so the transplant deviation holds. |
| T07 | secrets-research | — | ⬜ | |
| T08 | map-command | T02, T05 | ⬜ | |
| T09 | secrets-layer | T07, T08 | ⬜ | |
| T10 | app-live | T02, T05, T06 | ⬜ | |
| T11 | app-show-map | T08, T10 | ⬜ | |
| T12 | map-drill | T09, T11 | ⬜ | |

**Review queue:** empty

## Blocked on the user

Nothing.
