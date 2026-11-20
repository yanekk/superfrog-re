# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Every task built; T12 awaiting review.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T12 (map-drill).

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
| T12 | map-drill | T09, T11 | 🔍 | Drill `fd-e2e/tests/map_drill.rs` (full, ignored, 13 min) passes. Fixed with tests: live switch on the old map, torn live.js read, secret mark, one EXIT, level-end sample as play. Owner kept route wording, hid secret counts. Deviations: `live_from_ram` takes `Prev` (T06 code); T09's levels_chrome test updated; W1-1 fast but its last 1500 steps. |

**Review queue:** T12

## Blocked on the user

Nothing.
