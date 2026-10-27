# Progress

**Update this whenever a task changes state.** It is the handoff between sessions.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The account is the commit message.

**Plan reviewed:** 2026-10-25 — 2 fixed, 3 decided with the user

**Status:** T01 implemented, awaiting review.
**Last updated:** 2026-10-27
**Next `pir-work` will:** review T01.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T01 | one-copy-each | — | 🔍 | Probe: per-modulo and start-only masks differ, so `BlitMem::MASK_EACH_LINE` (true for Bus). Deviations: widescreen's render/bobs.rs and render/mod.rs decodes merged too (owner yes); `hash_memory` free fn, `byte_at` Option reader; sf-plan's core_memory callers updated. measure.sh 1/3/1/2; sf-conform 41 cases and story 9+1 green. 6 new tests. |
| T02 | amiga-chips-crate | T01 | ⬜ | |

**Review queue:** T01

## Blocked on the user

Nothing.
