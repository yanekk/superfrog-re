# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Building; T03 implemented 2026-11-20.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T03 (chrome-rig); T04 follows it. T00, T01, T06 and T07 can run beside it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | items-needed-source | — | ⬜ | |
| T01 | objects-all-worlds | — | ⬜ | |
| T02 | map-levels | T00, T01 | ⬜ | |
| T03 | chrome-rig | — | 🔍 | Built `amiga-browser`: Chrome over the DevTools pipe, Fetch blocks non-file requests, loads tracked by loaderId, session limit plus backstop kill, group killed and profile removed on drop. 5 Chrome tests, 3 unit; click test in smoke (1 s). Deviations: added `pid()`, `profile()`, `truthy`, `file_url` for tests; README index line in toolkit/CLAUDE.md. |
| T04 | page-map-only | T03 | ⬜ | |
| T05 | page-live | T04 | ⬜ | |
| T06 | live-reader | — | ⬜ | |
| T07 | secrets-research | — | ⬜ | |
| T08 | map-command | T02, T05 | ⬜ | |
| T09 | secrets-layer | T07, T08 | ⬜ | |
| T10 | app-live | T02, T05, T06 | ⬜ | |
| T11 | app-show-map | T08, T10 | ⬜ | |
| T12 | map-drill | T09, T11 | ⬜ | |

**Review queue:** T03

## Blocked on the user

Nothing.
