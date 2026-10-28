# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-27 — 3 fixed, 1 decided with the user

**Status:** T01, T02 ✅; T03 next.
**Last updated:** 2026-10-28
**Next `pir-work` will:** implement T03.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | beam-pos-and-constants | — | ✅ | |
| T02 | generic-clock | T01 | ✅ | Review clean, no fix commit. Pinned save bytes rerun green on base `545a1fe`; mutations in clock slot test and timeline stepping caught; recorded W1-1 restart (42 writes) replays identically old versus new from 320 start lines; `beam`, `alone`, `frog_cases`, purity, `measure.sh` green. `story`/`channels --ignored` builder's run, not rerun. |
| T03 | front-end-and-slot-policies | T02 | ⬜ | |

**Review queue:** —

## Blocked on the user

Nothing.
