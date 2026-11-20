# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** planned 2026-11-20; nothing built.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T03; T04 to T06 and T10 can run beside it, T07 once T03 to T06 are
✅.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | title-read | — | ✅ | |
| T02 | case-goals | — | ✅ | |
| T03 | title-idle-rows-cases | T01, T02 | 🔍 | title-idle-loop and title-rows pass translated and mounted, goals met at 15014 and 5141; title_idle_cases.rs, 1 smoke and 2 full tests. Deviations: `all` goals take `after = F` (owner's choice: both goals held on frame one), in case.rs, frames.rs, lib.rs and six tests' literals; both cases added to mount_level_1.rs `PASS_MOUNTED`, which must list every case. |
| T04 | title-code-cases | T01, T02 | ⬜ | |
| T05 | title-quit-cases | T01, T02 | ⬜ | |
| T06 | game-over-cases | T01, T02 | ⬜ | |
| T07 | title-fixes | T03, T04, T05, T06 | ⬜ | |
| T08 | world-start-recording | — | ✅ | |
| T09 | start-level-worlds | T08 | ✅ | |
| T10 | title-app-paths | T01 | ⬜ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** T03

## Blocked on the user

Nothing.
