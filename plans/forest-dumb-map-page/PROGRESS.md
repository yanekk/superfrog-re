# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Planned 2026-11-20; nothing built.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T05 (page-live); T07 can run beside it, then T08 and T10 once T05 is ✅.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | items-needed-source | — | ✅ | |
| T01 | objects-all-worlds | — | ✅ | |
| T02 | map-levels | T00, T01 | ✅ | |
| T03 | chrome-rig | — | ✅ | |
| T04 | page-map-only | T03 | ✅ | |
| T05 | page-live | T04 | 🔍 | LiveSpec, LiveFrame, write_live and the page's live mode: status bar, dot, creatures, changed cells, Follow, level switching. Six tests: live.rs 2, live_chrome.rs 2, one site.rs, one sf-routeview. Deviations: hand scroll tested as a wheel turn (rig has no drag), which stops Follow only in live mode; the font request is allowed as in T04; test hook `ROUTEVIEW.view()`. |
| T06 | live-reader | — | ✅ | |
| T07 | secrets-research | — | ⬜ | |
| T08 | map-command | T02, T05 | ⬜ | |
| T09 | secrets-layer | T07, T08 | ⬜ | |
| T10 | app-live | T02, T05, T06 | ⬜ | |
| T11 | app-show-map | T08, T10 | ⬜ | |
| T12 | map-drill | T09, T11 | ⬜ | |

**Review queue:** T05

## Blocked on the user

Nothing.
