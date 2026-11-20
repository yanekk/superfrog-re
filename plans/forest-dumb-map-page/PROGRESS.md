# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Building since 2026-11-20; T00 to T07 done, T08 in review.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T08; then T09 (secrets-layer) and, with T10 done, T11 (app-show-map).

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
| T08 | map-command | T02, T05 | 🔍 | Built `--all`/`--level` via `fd_map::write_site`/`write_levels`, live on; 4 tests in levels_chrome.rs (2 smoke). Deviations: fd-tools a normal dep (raw program bytes); MapLevel's entry has no verdict (default read Clean); misc layer names flames; `--level W2-1 --script` exits 2 until level-2 merges; route page keeps live off via fd-routeview::spec. |
| T09 | secrets-layer | T07, T08 | ⬜ | |
| T10 | app-live | T02, T05, T06 | ⬜ | |
| T11 | app-show-map | T08, T10 | ⬜ | |
| T12 | map-drill | T09, T11 | ⬜ | |

**Review queue:** T08

## Blocked on the user

Nothing.
