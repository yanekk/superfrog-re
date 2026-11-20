# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** building; T01 done.
**Last updated:** 2026-11-20
**Next `pir-work` will:** T02 and T09, then T03 to T06 and T10, which T01 unblocked.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | title-read | — | ✅ | Review: one fix, high-score names were not LAYOUT rows (now `hiscore_name_N+$0/+$4/+$8`, held by level_entry); docs/16 countdown branch. Probed: world and cheat at load_world entry by poke_at sentinels, game-over branches and docs/07 claims in the listing. Mutations caught: a code digit, `title_start_flag`. Lives-out path read-only. |
| T02 | case-goals | — | ⬜ | |
| T03 | title-idle-rows-cases | T01, T02 | ⬜ | |
| T04 | title-code-cases | T01, T02 | ⬜ | |
| T05 | title-quit-cases | T01, T02 | ⬜ | |
| T06 | game-over-cases | T01, T02 | ⬜ | |
| T07 | title-fixes | T03, T04, T05, T06 | ⬜ | |
| T08 | world-start-recording | — | ✅ | |
| T09 | start-level-worlds | T08 | ⬜ | |
| T10 | title-app-paths | T01 | ⬜ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
