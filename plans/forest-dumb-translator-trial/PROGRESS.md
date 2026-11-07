# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-07 — 8 fixed, 3 decided with the user

**Status:** T00 implemented, awaiting review. This branch merges into `main` only after `forest-dumb-level-1` has (DESIGN §7).
**Last updated:** 2026-11-07
**Next `pir-work` will:** review T00; then T01, T02 and T05 can start.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | bring-in-level-1 | — | 🔍 | Merged `pir/forest-dumb-level-1` at `defe7d6b` with no conflict and no level 1 file edited. No new tests: judge builds, `--test judge` passes, Muse's `h0+$5462` port passes 2000/2000. Baseline row in FINDINGS. No deviations. |
| T01 | pcode-export | T00 | ⬜ | |
| T02 | pcode-runtime | T00 | ⬜ | |
| T03 | pcode-translator | T02 | ⬜ | |
| T04 | judge-translated | T00, T02 | ⬜ | |
| T05 | code-writes-check | T00 | ⬜ | |
| T06 | trial-tool | T01, T03, T04 | ⬜ | |
| T07 | trial-fixes | T06 | ⬜ | |
| T08 | trial-report | T05, T07 | ⬜ | |

**Review queue:** T00

## Blocked on the user

Nothing yet. T08 ends with the owner's go or no-go.
