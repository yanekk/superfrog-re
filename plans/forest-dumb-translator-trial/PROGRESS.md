# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-07 — 8 fixed, 3 decided with the user

**Status:** Planned, not started. This branch merges into `main` only after `forest-dumb-level-1` has (DESIGN §7).
**Last updated:** 2026-11-07
**Next `pir-work` will:** review T04; T01, T03 and T05 can be built meanwhile.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | bring-in-level-1 | — | ✅ | |
| T01 | pcode-export | T00 | ⬜ | |
| T02 | pcode-runtime | T00 | ✅ | |
| T03 | pcode-translator | T02 | ⬜ | |
| T04 | judge-translated | T00, T02 | 🔍 | `fd-judge --translated`, `judge::translated::check`, `FD_TRANSLATED` stub; 4 tests. Deviations: `Image` also takes hunk sizes (T02's interface), from new `Rig::hunk_sizes`; stops reach the rig as `routine::PortStop` (routine.rs, unlisted); the flipped constant fails 46 of 50 states, not all; no `fd-conform` opt-level override (FINDINGS). |
| T05 | code-writes-check | T00 | ⬜ | |
| T06 | trial-tool | T01, T03, T04 | ⬜ | |
| T07 | trial-fixes | T06 | ⬜ | |
| T08 | trial-report | T05, T07 | ⬜ | |

**Review queue:** T04

## Blocked on the user

Nothing yet. T08 ends with the owner's go or no-go.
