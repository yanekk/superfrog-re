# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** planned 2026-11-20; nothing built.
**Last updated:** 2026-11-20 (T07 reviewed)
**Next `pir-work` will:** T11 title-drill, its dependencies all ✅.

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
| T07 | title-fixes | T03, T04, T05, T06 | ✅ | One review fix: a test for a key changing while its byte clocks out (a surviving mutation) and `set_input`'s stale doc. Probed: fix reverted turns five KODY cases and three machine tests red; 14 key or boot-Esc cases rerun translated as recorded, `power-on-level-1` at 6301; `title-code-w1` mounted at 6429 reproduced, its mount attribution read-only. |
| T08 | world-start-recording | — | ✅ | |
| T09 | start-level-worlds | T08 | ✅ | |
| T10 | title-app-paths | T01 | ✅ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
