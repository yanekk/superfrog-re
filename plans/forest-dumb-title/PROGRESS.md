# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** T01 to T10 ✅; T11 built, its OS-layer check waiting on the owner (screen locked).
**Last updated:** 2026-11-20
**Next `pir-work` will:** finish T11: once the owner unlocks the screen, run `FD_E2E_OS=1 cargo test --release -p fd-e2e --test start_level_worlds -- --ignored --test-threads 1 os_layer`, then hand T11 to review.

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
| T11 | title-drill | T07, T09, T10 | 🟡 | Automated half green: `title_drill.rs` (5 tests), rolled-keys fix in `cia.rs` with `title-code-rolled`, docs closed. **Unverified: real-window half** (`start_level_worlds` OS-layer test at three sizes) not run, screen locked; waits for the owner to unlock it. No deviation from the task doc. |

**Review queue:** *(empty)*

## Blocked on the user

T11: unlock the Mac's screen for about 3 minutes (the real Start Level… window test).
