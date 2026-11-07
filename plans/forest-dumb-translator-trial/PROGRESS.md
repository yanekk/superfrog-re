# Progress

**Update this whenever a task changes state.** It is the handoff between sessions. What the build taught lives in [FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted.

**Plan reviewed:** 2026-11-07 — 8 fixed, 3 decided with the user

**Status:** Planned, not started. This branch merges into `main` only after `forest-dumb-level-1` has (DESIGN §7).
**Last updated:** 2026-11-07
**Next `pir-work` will:** review T03.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | bring-in-level-1 | — | ✅ | |
| T01 | pcode-export | T00 | ⬜ | |
| T02 | pcode-runtime | T00 | ✅ | |
| T03 | pcode-translator | T02 | 🔍 | pcode_rust.py and 9 made-up fixtures (committed translations) run by translated.rs (10 tests); 15 Python tests. Bus is `&mut dyn Bus` (51 s vs 52 s optimised build, 9 000 instructions). Deviations: module also exports `HUNK_SIZES` and `Routine`; the return check is at the call site, `RETURN` setting `PC`; export's hunk-relative form assumed (FINDINGS). |
| T04 | judge-translated | T00, T02 | ⬜ | |
| T05 | code-writes-check | T00 | ⬜ | |
| T06 | trial-tool | T01, T03, T04 | ⬜ | |
| T07 | trial-fixes | T06 | ⬜ | |
| T08 | trial-report | T05, T07 | ⬜ | |

**Review queue:** T03

## Blocked on the user

Nothing yet. T08 ends with the owner's go or no-go.
