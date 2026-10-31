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
| T01 | label-check | — | ✅ | |
| T02 | smoke-runner | T01 | ✅ | |
| T03 | file-timings | T01 | ✅ | |
| T04 | full-command | — | ✅ | |
| T05 | toolkit-and-forest-labels | T02, T03 | 🔍 | Both `smoke.toml` written; all 38 entries green alone. `first_case` smoke by name (four tests, 65 s), `a_start_one_frame_early_is_caught_and_named` stays full. Deviation: `exe_map_claims` measured 4.5 s, so smoke. Added `src` covers to areas. |
| T06 | superfrog-labels | T02, T03 | ⬜ | |
| T07 | smoke-fits | T02, T05, T06 | ⬜ | |
| T08 | switch-and-directives | T04, T07 | ⬜ | |

**Review queue:** T05

## Blocked on the user

Nothing.
