# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-27 — 3 fixed, 1 decided with the user

**Status:** T01, T02, T03 done.
**Last updated:** 2026-10-28
**Next `pir-work` will:** nothing; the plan's tasks are done.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | beam-pos-and-constants | — | ✅ | |
| T02 | generic-clock | T01 | ✅ | |
| T03 | front-end-and-slot-policies | T02 | ✅ | Review: one fix, NORTH-STAR log delimiter row had 3 cells for 4 columns. Mutations caught: `spend_stolen` rounding (policy sweep), `Cx::busy` floor (`front` conform). Ran `front`, `slot_routines`, `beam`, `alone`, measure.sh, docs test green; `sf-conform all`, `whole_game` read-only. Fade, `interrupt_lines` divisions logged. |

**Review queue:** *(empty)*

## Blocked on the user

Nothing.
