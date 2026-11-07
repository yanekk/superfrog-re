# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-07 — 8 fixed, 3 decided with the user

**Status:** Planned, not started. This branch merges into `main` only after `forest-dumb-level-1` has (DESIGN §7).
**Last updated:** 2026-11-07
**Next `pir-work` will:** T02 and T05 are open; T06 waits on T01, T03, T04.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | bring-in-level-1 | — | ✅ | |
| T01 | pcode-export | T00 | ✅ | Review: one fix, an unrelocated in-range constant was written as an address; now numeric, test locks it. Probed: game export cross-checked against the file's 1 002 relocs, rerun on saved project identical, Superfrog import tests green. Mutations of reloc matching and flow reading caught. Seven deviations held. |
| T02 | pcode-runtime | T00 | ⬜ | |
| T03 | pcode-translator | T02 | ⬜ | |
| T04 | judge-translated | T00, T02 | ⬜ | |
| T05 | code-writes-check | T00 | ⬜ | |
| T06 | trial-tool | T01, T03, T04 | ⬜ | |
| T07 | trial-fixes | T06 | ⬜ | |
| T08 | trial-report | T05, T07 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing yet. T08 ends with the owner's go or no-go.
