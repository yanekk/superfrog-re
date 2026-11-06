# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-01 — 15 fixed, 6 decided with the user
**Amended:** 2026-11-03 — the first game's lessons: porting rulebook, port register and coverage, T31 added (owner)
**Amended:** 2026-11-05 — translation by Muse and small sessions (brief): T32 to T48 added; T06, T08 to T11, T22, T24, T25, T27 reshaped (owner).
**Amendment reviewed:** 2026-11-05 — 15 fixed, 4 decided with the owner (T49 added)
**Amended mid-run:** 2026-11-05 — Muse writes generators (T60, T61; T49 uses them) and an engine-shape trial (T62) (owner). Read back 2026-11-05: 11 fixed, 1 decided (T62 measures on T08's and T09's routines, blocks nothing).

**Amended mid-run:** 2026-11-06 — cost per step (DESIGN §4.4): task packs first in every pending task, a smaller start for workers, Muse writes porting tasks' engine code with a quality loop; T66 answered (owner). Run stopped for the edit.

**Status:** Building; 63 of 73 tasks ✅. Pending tasks start from their pack (`tools/task_pack.py`).
**Last updated:** 2026-11-06
**Next `pir-work` will:** T24, T52 and T72 (now unblocked by T25), then T47 and T46 as their dependencies finish. Start the run with `ENABLE_CLAUDEAI_MCP_SERVERS=false pir start forest-dumb-level-1`.

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
| T06 | level-runner | T03, T04, T31, T32 | ✅ | |
| T33 | translate-service | T32 | ✅ | |
| T34 | translate-inventory | T00 | ✅ | |
| T35 | gen-character | T33, T34 | ✅ | |
| T36 | gen-camera | T33, T34 | ✅ | |
| T37 | gen-drawing | T33, T34 | ✅ | |
| T38 | gen-objects | T33, T34 | ✅ | |
| T39 | gen-rest | T33, T34 | ✅ | |
| T40 | batch-translate | T35, T36, T37, T38, T39 | ✅ | |
| T07 | level-entry | T05, T06, T32 | ✅ | |
| T08 | character-movement | T07, T40 | ✅ | |
| T09 | camera-scroll | T07, T40 | ✅ | |
| T10 | level-drawing | T08, T09, T40 | ✅ | |
| T41 | case-run-jump | T08, T09 | ✅ | |
| T42 | case-walls-pits | T08, T09, T41 | ✅ | |
| T43 | case-scroll | T08, T09 | ✅ | |
| T44 | case-picture | T10 | ✅ | |
| T11 | character-cases | T01, T41, T42, T43, T44 | ✅ | |
| T12 | app-toolkit-move | — | ✅ | |
| T13 | forest-dumb-app | T01, T10, T12 | ✅ | |
| T14 | recorder | T12, T13 | ✅ | |
| T15 | recording-compare | T03, T14 | ✅ | |
| T16 | app-drill | T13, T14 | ✅ | |
| T17 | planner-toolkit-move | T02 | ✅ | |
| T18 | map-page-toolkit-move | T17 | ✅ | |
| T19 | map-page | T11, T18 | ✅ | |
| T20 | planner-level-1 | T11, T17, T19 | ✅ | |
| T21 | object-meanings | T05, T06, T32 | ✅ | |
| T49 | translate-objects | T21, T38, T40 | ✅ | |
| T22 | collectibles | T11, T20, T21, T40, T49 | ✅ | |
| T23 | lives-and-game-over | T11, T13, T14, T20 | ✅ | |
| T24 | enemies | T20, T21, T22, T23 | ⬜ | |
| T45 | case-collect | T22 | ✅ | |
| T46 | case-enemies | T24, T45 | ⬜ | |
| T25 | mechanics | T11, T20, T21, T22 | ✅ | Reviewed ✅, two fixes: play screen's `Blit::Element`/`Picture` untested (test added); `DisplayData` lacked `_ZNIKAJKA`, `_WYBIJACZKA`, `_Plomien` (drawn black; test fails without). Probed: rig mutants, branch coverage, unfreezing. Merge with T70: frozen cases freeze the engine's `h0_6882`, `h0_24ce` too (owner); route.rs re-measured. |
| T26 | level-end | T11, T13, T14, T23 | ✅ | |
| T47 | case-mechanics | T25, T45 | ⬜ | |
| T27 | full-route | T20, T26, T45, T46, T47 | ⬜ | |
| T48 | route-on-original | T27 | ⬜ | |
| T28 | finish-end-to-end | T16, T48 | ⬜ | |
| T29 | level-drill | T15, T28 | ⬜ | |
| T50 | sf-menus-os-test-stale | — | ✅ | |
| T51 | character-anim-input | T07, T40; blocks T08 | ✅ | |
| T52 | creature-drawing | T10, T22; blocks T46 | ⬜ | |
| T53 | gen-character-2 | T33, T34; blocks T40 | ✅ | |
| T54 | gen-objects-2 | T33, T34; blocks T40 | ✅ | |
| T55 | edge-redraw | T09; blocks T10, T41, T42, T43 | ✅ | |
| T56 | gen-edge-redraw | T33, T34; blocks T40 | ✅ | |
| T57 | sf-scale-fullscreen-os-test | — | ✅ | |
| T58 | translate-btst-fix | T33; blocks T40 | ✅ | |
| T59 | oracle-sprite-collision | T21; blocks T24, T41, T42, T43, T44, T45, T46, T47 | ✅ | |
| T60 | rig-branch-coverage | T32 | ✅ | |
| T61 | generate-service | T33, T60; blocks T49 | ✅ | |
| T62 | engine-shape-trial | T08, T09, T33 | ✅ | |
| T63 | game-frame-rig | T08, T09, T10, T22, T23, T24, T25, T26, T51, T52, T55; blocks T27 | ⬜ | |
| T64 | listing-odd-bytes | T08, T51 | ✅ | |
| T66 | reshape-decision | T62; blocks T22 | ✅ | Owner answered 2026-11-06: yes, Muse tidies first through a quality loop (DESIGN §4.4, docs/11 "Reshaping by Muse"). Closed by the owner's amendment, no worker. |
| T65 | level-start-draw | T55; blocks T13, T20 | ✅ | |
| T67 | pass-input-read | T06; blocks T42, T43, T44, T15 | ✅ | |
| T69 | level-start-state | T07, T65; blocks T20, T28 | ✅ | |
| T68 | app-drill-os-half | T16; blocks T28 | ✅ | |
| T70 | playfield-contact | T23; blocks T24, T46 | ✅ | |
| T71 | status-redraw-race | T70; blocks T27 | ✅ | |
| T72 | water-contact | T25, T70; blocks T47, T27 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
