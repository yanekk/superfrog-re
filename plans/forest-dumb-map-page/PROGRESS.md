# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Planned 2026-11-20; nothing built.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T02; T03 (chrome-rig) heads the critical path, T06 and T07 beside it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | items-needed-source | — | ✅ | |
| T01 | objects-all-worlds | — | ✅ | |
| T02 | map-levels | T00, T01 | 🔍 | fd-map: MapLevel per world from the files, object tables gathered by build.rs, LevelDump, default_out. 7 tests in levels.rs (smoke, disks), purity scans level.rs, map_oracle.rs full (all six equal, 76 s). Deviations: items_needed takes the object map (T00); oracle test in fd-conform; new `flame` cat; no exit is an error; conflicting tables keep the first. |
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

**Review queue:** T02

## Blocked on the user

Nothing.
