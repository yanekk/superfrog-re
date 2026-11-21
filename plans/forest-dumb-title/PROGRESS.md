# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** T01 to T10 ✅; T11 🔍 awaiting review.
**Last updated:** 2026-11-21
**Next `pir-work` will:** review T11.

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
| T07 | title-fixes | T03, T04, T05, T06 | ✅ | |
| T08 | world-start-recording | — | ✅ | |
| T09 | start-level-worlds | T08 | ✅ | |
| T10 | title-app-paths | T01 | ✅ | |
| T11 | title-drill | T07, T09, T10 | 🔍 | Drill `title_drill.rs` (5 tests) worker-driven at three sizes; rolled-keys fix in `cia.rs` with case `title-code-rolled`; docs closed. OS layer `start_level_worlds` green at default, 400×300, 1200×900 (2026-11-21). No deviation from the task doc. |

**Review queue:** T11

## Blocked on the user

*(none)*
