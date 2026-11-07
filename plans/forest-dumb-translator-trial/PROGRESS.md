# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-07 — 8 fixed, 3 decided with the user

**Status:** Planned, not started. This branch merges into `main` only after `forest-dumb-level-1` has (DESIGN §7).
**Last updated:** 2026-11-07
**Next `pir-work` will:** T01, T03 and T04, whose dependencies are ✅.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | bring-in-level-1 | — | ✅ | |
| T01 | pcode-export | T00 | ⬜ | |
| T02 | pcode-runtime | T00 | ✅ | |
| T03 | pcode-translator | T02 | ⬜ | |
| T04 | judge-translated | T00, T02 | ⬜ | |
| T05 | code-writes-check | T00 | ✅ | `fd-conform code-writes`. Review clean, no fix commit. Full run reproduced: 33 cases, 80 964 frames, 0 changes. Mutations caught: no frame-to-frame compare, no compare with the loaded program, 2-byte lengths. Probed CLI exits 0/1/2, rig and judge tests after the loader move. Four deviations held. |
| T06 | trial-tool | T01, T03, T04 | ⬜ | |
| T07 | trial-fixes | T06 | ⬜ | |
| T08 | trial-report | T05, T07 | ⬜ | |

**Review queue:** empty

## Blocked on the user

Nothing yet. T08 ends with the owner's go or no-go.
