# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Planned 2026-11-20; nothing built.
**Last updated:** 2026-11-20
**Next `pir-work` will:** T10 (app-live), then T11 and T12.

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
| T09 | secrets-layer | T07, T08 | ✅ | Review fixed one defect: `--script` route page dropped W1-1's 133 secrets (fd-routeview `replay`), test locks it. Deviation (separate `secrets` field) held. Probed T84 route markers on secret cells (none); mutated JSON order and layer on-at-load, both caught; judged Chrome screenshots worker-driven. |
| T10 | app-live | T02, T05, T06 | ⬜ | |
| T11 | app-show-map | T08, T10 | ⬜ | |
| T12 | map-drill | T09, T11 | ⬜ | |

**Review queue:** (empty)

## Blocked on the user

Nothing.
