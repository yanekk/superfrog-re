# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** building; T08 ✅.
**Last updated:** 2026-11-20
**Next `pir-work` will:** T09 start-level-worlds (T08 ✅); T01 title-read and T02 beside it.

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
| T08 | world-start-recording | — | ✅ | Clean; one fix: fd-app README lists the world-start exports. Ran world 3 cheat replay full (RAM equal, frame 497); mutation dropping the replay's cheat write went red. Probed bug replay, `--engine hand` and compare on hand-made world zips. Format `start world` plus level line held; Start Level offers W1-1 until T09. |
| T09 | start-level-worlds | T08 | ⬜ | |
| T10 | title-app-paths | T01 | ⬜ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
