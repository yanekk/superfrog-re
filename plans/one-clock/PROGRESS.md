# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-27 — 3 fixed, 1 decided with the user

**Status:** T01 done; T02 next.
**Last updated:** 2026-10-28
**Next `pir-work` will:** implement T02.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | beam-pos-and-constants | — | ✅ | Review clean, no fix commit. Mutated `plus_slots`, `dma`, `paula_hpos`: amiga-clock tests red (client crates' unit tests stay green). Ran story_draws/blits, frog_cases, purity, beam, sf-core front/savestate, examples build; length-literal sweep clean; `channels` whole-game run gives the same verdict on base `1249729` as after (FINDINGS). |
| T02 | generic-clock | T01 | ⬜ | |
| T03 | front-end-and-slot-policies | T02 | ⬜ | |

**Review queue:** empty

## Blocked on the user

Nothing.
