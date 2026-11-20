# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Planned 2026-11-20; nothing built.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T11; T09 then T12 (map-drill).

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | items-needed-source | — | ✅ | |
| T01 | objects-all-worlds | — | ✅ | |
| T02 | map-levels | T00, T01 | ✅ | |
| T03 | chrome-rig | — | ✅ | |
| T04 | page-map-only | T03 | ✅ | |
| T05 | page-live | T04 | ✅ | |
| T06 | live-reader | — | ✅ | |
| T07 | secrets-research | — | ✅ | |
| T08 | map-command | T02, T05 | ✅ | |
| T09 | secrets-layer | T07, T08 | ⬜ | |
| T10 | app-live | T02, T05, T06 | ✅ | |
| T11 | app-show-map | T08, T10 | 🔍 | Game ▸ Show Map: page written on a thread, opened in Chrome; 3 unit, 3 e2e tests (one OS layer), owner saw Chrome open. Deviations: `would open` goes to stderr, the harness drops stdout; page written before the Chrome check so its path is named; owner ran `--engine hand`, no generated tree here. |
| T12 | map-drill | T09, T11 | ⬜ | |

**Review queue:** T11

## Blocked on the user

Nothing.
