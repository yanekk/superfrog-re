# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-27 — 3 fixed, 1 decided with the user

**Status:** T01 implemented, awaiting review.
**Last updated:** 2026-10-28
**Next `pir-work` will:** review T01.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | beam-pos-and-constants | — | 🔍 | `amiga-clock`: `BeamPos`, four lengths, 6 tests; copper `Beam` aliases it; every length reads it. Deviations: story `cpu::LINE` renamed `LINE_SLOTS` (counted name); front/slot names kept as `use … as` aliases; base `story`/`channels` not run, `channels` red pre-existing (FINDINGS). |
| T02 | generic-clock | T01 | ⬜ | |
| T03 | front-end-and-slot-policies | T02 | ⬜ | |

**Review queue:** T01

## Blocked on the user

Nothing.
