# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** planned 2026-11-20; nothing built.
**Last updated:** 2026-11-20
**Next `pir-work` will:** T03, T06 and T10; T07 waits on T03 and T06.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | title-read | — | ✅ | |
| T02 | case-goals | — | ✅ | |
| T03 | title-idle-rows-cases | T01, T02 | ⬜ | |
| T04 | title-code-cases | T01, T02 | ✅ | Clean, no fix commit. Ran all six tests and T01's title_codes.rs green; mutation (cheat script typing 2401) turned code_2400 red; probes: `run` of w5 and w1 mounted read the same 5051 difference, goal 6438. Deviations held. Translated passing is T07's: all five differ from 5051. |
| T05 | title-quit-cases | T01, T02 | ✅ | |
| T06 | game-over-cases | T01, T02 | ⬜ | |
| T07 | title-fixes | T03, T04, T05, T06 | ⬜ | |
| T08 | world-start-recording | — | ✅ | |
| T09 | start-level-worlds | T08 | ✅ | |
| T10 | title-app-paths | T01 | ⬜ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
