# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build
taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-20 — 10 fixed, 1 decided with the user

**Status:** planned 2026-11-20; nothing built.
**Last updated:** 2026-11-20
**Next `pir-work` will:** review T09; T01 title-read, because every case and app test names what it finds; T02 and
T08 can run beside it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | title-read | — | ⬜ | |
| T02 | case-goals | — | ⬜ | |
| T03 | title-idle-rows-cases | T01, T02 | ⬜ | |
| T04 | title-code-cases | T01, T02 | ⬜ | |
| T05 | title-quit-cases | T01, T02 | ⬜ | |
| T06 | game-over-cases | T01, T02 | ⬜ | |
| T07 | title-fixes | T03, T04, T05, T06 | ⬜ | |
| T08 | world-start-recording | — | ✅ | |
| T09 | start-level-worlds | T08 | 🔍 | Start Level… W1-1 to W6-1 with Cheat on, `--level 1-6 [--cheat]`, `Translated::to_world`; 4 unit, 7 e2e tests (one OS layer, three sizes). Deviations: `spec`, `chosen`, `script::parse` take the engine, since the hand window differs; W1-1 cheat-off records `level-1`, no write (`fd_app::world_start`), as `--level 1` did; `to_level_1` removed. |
| T10 | title-app-paths | T01 | ⬜ | |
| T11 | title-drill | T07, T09, T10 | ⬜ | |

**Review queue:** T09

## Blocked on the user

Nothing.
