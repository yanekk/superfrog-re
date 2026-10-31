# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale
tracker costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows
touching the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The cell is an index for the next session;
the account is the commit message. Whoever writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-31 — 13 fixed, 2 decided with the user

**Status:** Plan written 2026-10-31. Nothing built.
**Last updated:** 2026-10-31
**Next `pir-work` will:** T01 and T04, which depend on nothing; T01 unblocks the most.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | label-check | — | ⬜ | |
| T02 | smoke-runner | T01 | ⬜ | |
| T03 | file-timings | T01 | ⬜ | |
| T04 | full-command | — | 🔍 | `tools/full.sh` plus flag in three `test.sh`; 9 tests. Proven on a fake cargo only, unrun for real. Deviation: added `--exact` beside `--skip`, because `--skip` alone matches substrings; a screen test in a module would need its path. |
| T05 | toolkit-and-forest-labels | T02, T03 | ⬜ | |
| T06 | superfrog-labels | T02, T03 | ⬜ | |
| T07 | smoke-fits | T02, T05, T06 | ⬜ | |
| T08 | switch-and-directives | T04, T07 | ⬜ | |

**Review queue:** T04

## Blocked on the user

Nothing.
