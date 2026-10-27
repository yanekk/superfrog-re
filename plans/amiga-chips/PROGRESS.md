# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-25 — 2 fixed, 3 decided with the user

**Status:** T01 done; T02 implemented, awaiting review.
**Last updated:** 2026-10-27
**Next `pir-work` will:** review T02.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | one-copy-each | — | ✅ | |
| T02 | amiga-chips-crate | T01 | 🔍 | Crate `amiga-chips` (blit, copper, memory, `is_paula` in lib.rs); sf-core paths re-export. 8 tests moved; Bus-based blit tests stay in sf-core since they need `Bus`/`State`. Copper module doc's `mod.rs` pointer reworded. measure.sh, docs check, all §4 sf-conform runs green. |

**Review queue:** T02

## Blocked on the user

Nothing.
