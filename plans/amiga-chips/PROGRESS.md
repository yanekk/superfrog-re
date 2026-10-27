# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-25 — 2 fixed, 3 decided with the user

**Status:** T01 reviewed ✅; T02 ready.
**Last updated:** 2026-10-27
**Next `pir-work` will:** build T02.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | one-copy-each | — | ✅ | Review: one fix, a test for A's first/last-word masks over the Bus (dropping `alwm` passed every suite). Mutations of MASK_EACH_LINE and panel order caught. Reran measure.sh 1/3/1/2, sf-conform 41 cases, story 9+1: green. Scroll/panel pointers stay below `$80000`, so per-line masking is a no-op there (read-only). |
| T02 | amiga-chips-crate | T01 | ⬜ | |

**Review queue:** empty

## Blocked on the user

Nothing.
