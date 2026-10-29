# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md).** Read the rows touching
the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-28 — 4 fixed, 3 decided with the user

**Status:** planned, nothing built.
**Last updated:** 2026-10-29
**Next `pir-work` will:** T01, the only task with no dependency; it records the baseline everything
else is compared with.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | toolkit-workspace | — | ✅ | |
| T02 | amiga-input | T01 | ✅ | |
| T03 | amiga-disk | T01 | ✅ | |
| T04 | asset-readers | T03 | ✅ | |
| T05 | amiga-oracle | T02, T03, T04 | ⬜ | |
| T06 | amiga-conform | T05 | ⬜ | |
| T07 | amiga-app-parts | T02 | ✅ | Review clean, no fix commit. Probed: base and branch apps ran one 200-frame script; frames, window shot, windows.txt, inputs, audio, seed byte-equal, only clock/pacing/memory differ by load. Mutation: dropping `bar_points` turned recorder_bar red. Tests: amiga-app 29, sf-app 102, sf-e2e smoke, recorder_bar, view, window, persist. frog_cases, story not rerun: sf-conform tree unchanged. |
| T08 | amiga-e2e | T07 | ⬜ | |
| T09 | amiga-record | T03, T06, T07 | ⬜ | |
| T10 | display-piece | T01 | ✅ | |
| T11 | dry-fit-and-finish | T04, T05, T08, T09, T10 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
