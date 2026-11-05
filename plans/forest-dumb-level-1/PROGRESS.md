# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-01 — 15 fixed, 6 decided with the user
**Amended:** 2026-11-03 — the first game's lessons: porting rulebook, port register and coverage, T31 added (owner)
**Amended:** 2026-11-05 — translation by Muse and small sessions (brief): T32 to T48 added; T06, T08 to T11, T22, T24, T25, T27 reshaped (owner).
**Amendment reviewed:** 2026-11-05 — 15 fixed, 4 decided with the owner (T49 added)

**Status:** Amendment read back and its 4 decisions settled; ready to build. Nothing built.
**Last updated:** 2026-11-05
**Next `pir-work` will:** T00 and T32, because every level task rests on T00's measurements and every port on T32's rig.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | measure-play | — | ✅ | |
| T01 | smoke-room | — | ✅ | |
| T02 | route-pin | — | ✅ | |
| T31 | code-ran-counter | — | ✅ | |
| T32 | routine-rig | — | ✅ | |
| T03 | level-sync | T00 | ✅ | |
| T04 | picture-in-play | T00 | ✅ | |
| T30 | unpack-in-rust | — | ✅ | |
| T05 | level-readers | T03, T04, T30 | ✅ | |
| T06 | level-runner | T03, T04, T31, T32 | ⬜ | |
| T33 | translate-service | T32 | ✅ | |
| T34 | translate-inventory | T00 | ✅ | |
| T35 | gen-character | T33, T34 | 🔍 | 14 generators in `gen/character.rs` with a `COVER` table (reads, branches); 4 tests in `gen_character.rs`: 2 000 states, branches both ways via a trap detour, pokes within reads. `h0_4274` passed T33's service. Deviation: branch check by detour, the oracle has no trace. Two raster-wait branches named unreachable. |
| T36 | gen-camera | T33, T34 | ⬜ | |
| T37 | gen-drawing | T33, T34 | ⬜ | |
| T38 | gen-objects | T33, T34 | ⬜ | |
| T39 | gen-rest | T33, T34 | ⬜ | |
| T40 | batch-translate | T35, T36, T37, T38, T39 | ⬜ | |
| T07 | level-entry | T05, T06, T32 | ⬜ | |
| T08 | character-movement | T07, T40 | ⬜ | |
| T09 | camera-scroll | T07, T40 | ⬜ | |
| T10 | level-drawing | T08, T09, T40 | ⬜ | |
| T41 | case-run-jump | T08, T09 | ⬜ | |
| T42 | case-walls-pits | T08, T09, T41 | ⬜ | |
| T43 | case-scroll | T08, T09 | ⬜ | |
| T44 | case-picture | T10 | ⬜ | |
| T11 | character-cases | T01, T41, T42, T43, T44 | ⬜ | |
| T12 | app-toolkit-move | — | ✅ | |
| T13 | forest-dumb-app | T01, T10, T12 | ⬜ | |
| T14 | recorder | T12, T13 | ⬜ | |
| T15 | recording-compare | T03, T14 | ⬜ | |
| T16 | app-drill | T13, T14 | ⬜ | |
| T17 | planner-toolkit-move | T02 | ✅ | |
| T18 | map-page-toolkit-move | T17 | ✅ | |
| T19 | map-page | T11, T18 | ⬜ | |
| T20 | planner-level-1 | T11, T17, T19 | ⬜ | |
| T21 | object-meanings | T05, T06, T32 | ⬜ | |
| T49 | translate-objects | T21, T38, T40 | ⬜ | |
| T22 | collectibles | T11, T20, T21, T40, T49 | ⬜ | |
| T23 | lives-and-game-over | T11, T13, T14, T20 | ⬜ | |
| T24 | enemies | T20, T21, T22, T23 | ⬜ | |
| T45 | case-collect | T22 | ⬜ | |
| T46 | case-enemies | T24, T45 | ⬜ | |
| T25 | mechanics | T11, T20, T21, T22 | ⬜ | |
| T26 | level-end | T11, T13, T14, T23 | ⬜ | |
| T47 | case-mechanics | T25, T45 | ⬜ | |
| T27 | full-route | T20, T26, T45, T46, T47 | ⬜ | |
| T48 | route-on-original | T27 | ⬜ | |
| T28 | finish-end-to-end | T16, T48 | ⬜ | |
| T29 | level-drill | T15, T28 | ⬜ | |
| T50 | sf-menus-os-test-stale | — | ⬜ | |
| T51 | character-anim-input | T07, T40; blocks T08 | ⬜ | |
| T52 | creature-drawing | T10, T22; blocks T46 | ⬜ | |
| T53 | gen-character-2 | T33, T34; blocks T40 | ⬜ | |
| T54 | gen-objects-2 | T33, T34; blocks T40 | ⬜ | |
| T55 | edge-redraw | T09; blocks T10, T41, T42, T43 | ⬜ | |
| T56 | gen-edge-redraw | T33, T34; blocks T40 | ⬜ | |

**Review queue:** T35

## Blocked on the user

Nothing.
