# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-01 — 15 fixed, 6 decided with the user
**Amended:** 2026-11-03 — the first game's lessons: porting rulebook, port register and coverage, T31 added (owner)

**Status:** Plan written; nothing built.
**Last updated:** 2026-11-03
**Next `pir-work` will:** T00, because every level task rests on its measurements.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | measure-play | — | ⬜ | |
| T01 | smoke-room | — | ⬜ | |
| T02 | route-pin | — | ⬜ | |
| T31 | code-ran-counter | — | ⬜ | |
| T03 | level-sync | T00 | ⬜ | |
| T04 | picture-in-play | T00 | ⬜ | |
| T30 | unpack-in-rust | — | ⬜ | |
| T05 | level-readers | T03, T04, T30 | ⬜ | |
| T06 | level-runner | T03, T04, T31 | ⬜ | |
| T07 | level-entry | T05, T06 | ⬜ | |
| T08 | character-movement | T07 | ⬜ | |
| T09 | camera-scroll | T07 | ⬜ | |
| T10 | level-drawing | T08, T09 | ⬜ | |
| T11 | character-cases | T01, T08, T09, T10 | ⬜ | |
| T12 | app-toolkit-move | — | ⬜ | |
| T13 | forest-dumb-app | T01, T10, T12 | ⬜ | |
| T14 | recorder | T12, T13 | ⬜ | |
| T15 | recording-compare | T03, T14 | ⬜ | |
| T16 | app-drill | T13, T14 | ⬜ | |
| T17 | planner-toolkit-move | T02 | ⬜ | |
| T18 | map-page-toolkit-move | T17 | ⬜ | |
| T19 | map-page | T11, T18 | ⬜ | |
| T20 | planner-level-1 | T11, T17, T19 | ⬜ | |
| T21 | object-meanings | T05, T06 | ⬜ | |
| T22 | collectibles | T11, T20, T21 | ⬜ | |
| T23 | lives-and-game-over | T11, T13, T14, T20 | ⬜ | |
| T24 | enemies | T20, T21, T22, T23 | ⬜ | |
| T25 | mechanics | T11, T20, T21, T22 | ⬜ | |
| T26 | level-end | T11, T13, T14, T23 | ⬜ | |
| T27 | full-route | T20, T22, T24, T25, T26 | ⬜ | |
| T28 | finish-end-to-end | T16, T27 | ⬜ | |
| T29 | level-drill | T15, T28 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
