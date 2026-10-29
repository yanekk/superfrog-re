# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md).** Read the rows touching
the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-28 — 4 fixed, 3 decided with the user

**Status:** planned, nothing built.
**Last updated:** 2026-10-29
**Next `pir-work` will:** T03, T07 or T10 (T01, T02 done).

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | toolkit-workspace | — | ✅ | |
| T02 | amiga-input | T01 | ✅ | Review clean, no fix commit. Pre-move build serialises CiaKeyboard and KeySet to the bytes the new test pins; feed-order mutation went red; engine workspace compiles; purity, script, level_routines, alone, beam, architecture tests green. frog_cases and story `--ignored` not rerun (builder ran them, read-only). |
| T03 | amiga-disk | T01 | ⬜ | |
| T04 | asset-readers | T03 | ⬜ | |
| T05 | amiga-oracle | T02, T03, T04 | ⬜ | |
| T06 | amiga-conform | T05 | ⬜ | |
| T07 | amiga-app-parts | T02 | ⬜ | |
| T08 | amiga-e2e | T07 | ⬜ | |
| T09 | amiga-record | T03, T06, T07 | ⬜ | |
| T10 | display-piece | T01 | ⬜ | |
| T11 | dry-fit-and-finish | T04, T05, T08, T09, T10 | ⬜ | |

**Review queue:** empty

## Blocked on the user

Nothing.
