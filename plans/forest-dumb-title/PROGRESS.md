# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** building; T08 awaits review.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T08; T09 start-level-worlds can start once T08 is ✅. T01 and T02
as before.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | title-read | — | ⬜ | |
| T02 | case-goals | — | ⬜ | |
| T03 | title-idle-rows-cases | T01, T02 | ⬜ | |
| T04 | title-code-cases | T01, T02 | ⬜ | |
| T05 | title-quit-cases | T01, T02 | ⬜ | |
| T06 | game-over-cases | T01, T02 | ⬜ | |
| T07 | title-fixes | T03, T04, T05, T06 | ⬜ | |
| T08 | world-start-recording | — | 🔍 | Format v3 adds `start world` and `cheat on|off`; LevelId has six worlds; world_start_writes/bytes, LOAD_WORLD and start_reached sit beside to_level_1_input; replay_translated starts worlds. 6 new tests, world 3 replay full. Deviations: `start world` plus level line, not DESIGN `world-N` (task doc); location_in_ram any world; Start Level offers W1-1 until T09; bug compare refuses worlds. |
| T09 | start-level-worlds | T08 | ⬜ | |
| T10 | title-app-paths | T01 | ⬜ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** T08

## Blocked on the user

Nothing.
