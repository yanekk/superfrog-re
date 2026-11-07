# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-07 — 8 fixed, 3 decided with the user

**Status:** In progress. This branch merges into `main` only after `forest-dumb-level-1` has (DESIGN §7).
**Last updated:** 2026-11-07
**Next `pir-work` will:** review T02; T03 and T04 wait on it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | bring-in-level-1 | — | ✅ | |
| T01 | pcode-export | T00 | ⬜ | |
| T02 | pcode-runtime | T00 | 🔍 | pcode-rt built: Bus, Regs (REG_SPACE 0x750, offset consts), Image, Stop with Display, load/store, ops; 22 unit tests, purity passes. Deviations: Image gains `sizes`, since hunk_of cannot find a hunk's end from bases; ops add int_notequal, int_lessequal, int_slessequal (standard P-code); Image::describe names addresses; code ranges must be sorted by hunk, start. |
| T03 | pcode-translator | T02 | ⬜ | |
| T04 | judge-translated | T00, T02 | ⬜ | |
| T05 | code-writes-check | T00 | ⬜ | |
| T06 | trial-tool | T01, T03, T04 | ⬜ | |
| T07 | trial-fixes | T06 | ⬜ | |
| T08 | trial-report | T05, T07 | ⬜ | |

**Review queue:** T02

## Blocked on the user

Nothing yet. T08 ends with the owner's go or no-go.
