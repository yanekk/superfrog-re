# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md).** Read the rows touching
the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-28 — 4 fixed, 3 decided with the user

**Status:** planned, nothing built.
**Last updated:** 2026-10-29
**Next `pir-work` will:** T02 and T03, whose only dependency (T01) is done.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | toolkit-workspace | — | ✅ | |
| T02 | amiga-input | T01 | ⬜ | |
| T03 | amiga-disk | T01 | ⬜ | |
| T04 | asset-readers | T03 | ⬜ | |
| T05 | amiga-oracle | T02, T03, T04 | ⬜ | |
| T06 | amiga-conform | T05 | ⬜ | |
| T07 | amiga-app-parts | T02 | ⬜ | |
| T08 | amiga-e2e | T07 | ⬜ | |
| T09 | amiga-record | T03, T06, T07 | ⬜ | |
| T10 | display-piece | T01 | ✅ | `amiga_chips::display` (`Planes`, `Colours`, `window`, `draw`). Review: reverted a whole-crate `cargo fmt` of 75 files outside the task; added a BPL2MOD test (mutation survived). PIPELINE mutation red in sf-core; beam, frog_cases 36, story `--ignored` 9 green. Deviation held by the draw-equals-story_screen test. |
| T11 | dry-fit-and-finish | T04, T05, T08, T09, T10 | ⬜ | |

**Review queue:** empty

## Blocked on the user

Nothing.
