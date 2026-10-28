# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-27 — 3 fixed, 1 decided with the user

**Status:** T01, T02 done; T03 built, awaiting review.
**Last updated:** 2026-10-28
**Next `pir-work` will:** review T03.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | beam-pos-and-constants | — | ✅ | |
| T02 | generic-clock | T01 | ✅ | |
| T03 | front-end-and-slot-policies | T02 | 🔍 | `amiga_clock::policy`, five functions, 5 sweep tests against the old code. Deviations: `Cx` keeps `line`/`into` (allowed); slot `spend`, `done` stay as thin wrappers so their tests are unchanged; `finish_frame`'s fade line divisions left inline (costs, not spend); `sf-conform all` run in two parts. |

**Review queue:** T03

## Blocked on the user

Nothing.
