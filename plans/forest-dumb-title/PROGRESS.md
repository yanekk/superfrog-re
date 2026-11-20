# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** building; T01, T02, T05, T08, T09 done.
**Last updated:** 2026-11-20
**Next `pir-work` will:** build T03, T04, T06 and T10, then T07 fixes
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
| T05 | title-quit-cases | T01, T02 | ✅ | Review: one fix commit, a stale KONIEC release frame in a comment and the mounted case count. Probed both cases by the CLI, translated and mounted (koniec same, exit 5063; esc a frame late). Mutated a goal frame and Esc's input: both tests red. Esc for T07. |
| T06 | game-over-cases | T01, T02 | ⬜ | |
| T07 | title-fixes | T03, T04, T05, T06 | ⬜ | |
| T08 | world-start-recording | — | ✅ | |
| T09 | start-level-worlds | T08 | ✅ | |
| T10 | title-app-paths | T01 | ⬜ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** —

## Blocked on the user

Nothing.
