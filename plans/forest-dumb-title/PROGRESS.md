# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** building; T01, T02, T08, T09 done, T05 awaiting review.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T05; T03, T04, T06 and T10 can run beside it, then T07 fixes
title-esc's frame-late quit with the other cases' failures.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | title-read | — | ✅ | |
| T02 | case-goals | — | ✅ | |
| T03 | title-idle-rows-cases | T01, T02 | ⬜ | |
| T04 | title-code-cases | T01, T02 | ⬜ | |
| T05 | title-quit-cases | T01, T02 | 🔍 | title-koniec passes translated (exit 5063); title-esc differs, a frame late (original exits 5012, translated 5013, first Paula at 5011), for T07. title_quit_cases.rs, 2 full tests. Deviation: both cases added to mount_level_1.rs's lists, whose smoke test holds every case file. |
| T06 | game-over-cases | T01, T02 | ⬜ | |
| T07 | title-fixes | T03, T04, T05, T06 | ⬜ | |
| T08 | world-start-recording | — | ✅ | |
| T09 | start-level-worlds | T08 | ✅ | |
| T10 | title-app-paths | T01 | ⬜ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** T05

## Blocked on the user

Nothing.
