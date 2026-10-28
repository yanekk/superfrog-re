# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-27 — 3 fixed, 1 decided with the user

**Status:** T01 ✅; T02 implemented, awaiting review.
**Last updated:** 2026-10-28
**Next `pir-work` will:** review T02.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | beam-pos-and-constants | — | ✅ | |
| T02 | generic-clock | T01 | 🔍 | `Clock<T: Timing>`, blit timing, `Timeline<J>` in amiga-clock; sf-core keeps `Level`, `timeline::of`, `timeline::run`. 9 tests: old-code sweeps, pinned save bytes. Deviations: `Timing_` is `DisplayTiming`; `timeline::run` maps `TimelineYield` to `Yield`; timeline sweep uses seeded restart-shaped traces, not a disk recording. §4 runs green; `channels` as base. |
| T03 | front-end-and-slot-policies | T02 | ⬜ | |

**Review queue:** T02

## Blocked on the user

Nothing.
