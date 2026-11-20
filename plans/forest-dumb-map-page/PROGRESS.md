# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Every task built and reviewed.
**Last updated:** 2026-11-20
**Next `pir-work` will:** nothing left; pir's end gate runs smoke on the feature branch.

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
| T09 | secrets-layer | T07, T08 | ✅ | |
| T10 | app-live | T02, T05, T06 | ✅ | |
| T11 | app-show-map | T08, T10 | ✅ | |
| T12 | map-drill | T09, T11 | ✅ | Review clean, no fix commit. Drill rerun green (592 s, quit to "App not running" 3.4 s); screens judged. Mutated world gate, torn-read fallback, loaded-data check, secret count, EXIT label, secret mark: each red. Oracle `live_reader` 4/4, `fd-e2e` live 5/5, `live_chrome` 5 runs. Owner decisions on wording and secret counts `read-only`. |

**Review queue:** (empty)

## Blocked on the user

Nothing.
