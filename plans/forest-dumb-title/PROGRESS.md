# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** planned 2026-11-20; nothing built.
**Last updated:** 2026-11-20
**Next `pir-work` will:** build T06; T07 once T06 is ✅.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | title-read | — | ✅ | |
| T02 | case-goals | — | ✅ | |
| T03 | title-idle-rows-cases | T01, T02 | ✅ | Reviewed: one fix, the mounted test's stale "49 cases" note. Reran both full runs and `title-*` mounted (pass, goals 15014, 5141); probes saw row 2 at 5101 and `mt_songpos` 26 at 14906. Ignoring `after` goes red in the unit test and `title_rows` (met 4991). Owner's `after` choice read-only. |
| T04 | title-code-cases | T01, T02 | ✅ | |
| T05 | title-quit-cases | T01, T02 | ✅ | |
| T06 | game-over-cases | T01, T02 | ⬜ | |
| T07 | title-fixes | T03, T04, T05, T06 | ⬜ | |
| T08 | world-start-recording | — | ✅ | |
| T09 | start-level-worlds | T08 | ✅ | |
| T10 | title-app-paths | T01 | ✅ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** (empty)

## Blocked on the user

Nothing.
