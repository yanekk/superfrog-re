# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md).** Read the rows touching
the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-28 — 4 fixed, 3 decided with the user

**Status:** every task built; T11 awaiting review.
**Last updated:** 2026-10-30
**Next `pir-work` will:** review T11.

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
| T11 | dry-fit-and-finish | T04, T05, T08, T09, T10 | 🔍 | `toolkit/crates/dry-fit` (lib+bin, 7 tests), toolkit README, README section, NORTH-STAR column. `sf-conform all` 216/216 identical to T01; `whole_game` 7 green. Deviations: fixed a T07 escape, opt-level overrides for amiga-app/input/record/disk (soak fell behind unoptimised; commit 689b2513); `docs/architecture/README.md` unchanged (no checked table names a crate). |

**Review queue:** T11

## Blocked on the user

Nothing.
