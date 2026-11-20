# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** planned 2026-11-20; nothing built.
**Last updated:** 2026-11-20 (T07 implemented)
**Next `pir-work` will:** review T07 (title-fixes); then T11 title-drill, which waits on it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | title-read | — | ✅ | |
| T02 | case-goals | — | ✅ | |
| T03 | title-idle-rows-cases | T01, T02 | ✅ | |
| T04 | title-code-cases | T01, T02 | ✅ | |
| T05 | title-quit-cases | T01, T02 | ✅ | |
| T06 | game-over-cases | T01, T02 | ✅ | |
| T07 | title-fixes | T03, T04, T05, T06 | 🔍 | Machine keyboard: an idle keyboard's first byte lands `KEY_BYTE_CYCLES` after the press (oracle: level 2 on line 7). All six failing title cases pass translated; run-all after: 60 of 61, `power-on-level-1` at 6301 as before. One new, five changed machine tests. Deviations: before-run not re-run, taken from recorded readings; three world 1 KODY cases differ mounted (FINDINGS). |
| T08 | world-start-recording | — | ✅ | |
| T09 | start-level-worlds | T08 | ✅ | |
| T10 | title-app-paths | T01 | ✅ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** T07

## Blocked on the user

Nothing.
