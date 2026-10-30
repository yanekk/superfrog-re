# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md).** Read the rows touching
the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-28 — 4 fixed, 3 decided with the user

**Status:** every task built and reviewed.
**Last updated:** 2026-10-30
**Next `pir-work` will:** nothing; the plan's test block runs at its end.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | toolkit-workspace | — | ✅ | |
| T02 | amiga-input | T01 | ✅ | |
| T03 | amiga-disk | T01 | ✅ | |
| T04 | asset-readers | T03 | ✅ | |
| T05 | amiga-oracle | T02, T03, T04 | ✅ | |
| T06 | amiga-conform | T05 | ✅ | |
| T07 | amiga-app-parts | T02 | ✅ | |
| T08 | amiga-e2e | T07 | ✅ | |
| T09 | amiga-record | T03, T06, T07 | ✅ | |
| T10 | display-piece | T01 | ✅ | |
| T11 | dry-fit-and-finish | T04, T05, T08, T09, T10 | ✅ | One fix: toolkit/README.md was owned by no architecture chapter, test_architecture_docs red; verification owns it. Mutated blit mask, copper wait, Paula period: each caught. Re-diffed builder's sf-conform output against baseline-T01 (0 differences, not re-run); measure.sh re-run matches; opt-level=2 confirmed by verbose build. whole_game not re-run (read-only). |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
