# Progress

Update this whenever a task changes state; it is the handoff between sessions. What the build taught is in
[FINDINGS.md](FINDINGS.md). Sixty words to a Notes cell, counted, flat prose; whoever writes a cell fixes
the over-budget cell they walk past.

**Plan reviewed:** 2026-11-20 — 10 fixed, 4 decided with the user

**Status:** Building; T01 done.
**Last updated:** 2026-11-20
**Next `pir-work` will:** T03 (chrome-rig), the head of the critical path; T00, T06 and T07 can run beside it; T02 waits on T00.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | items-needed-source | — | ⬜ | |
| T01 | objects-all-worlds | — | ✅ | `objects-more.toml` holds `$34` flame, oracle claim, `object_tables.rs` smoke. Review fixed the tautological table-discovery test (hard-coded names passed), now a scratch dir with a per-world table. Probed: independent Python recount of `$34` and seen cells; purity fails with the test in `fd-assets`; placed `$28` turns the flame claim red. Frame by `anim_tick` read-only. |
| T02 | map-levels | T00, T01 | ⬜ | |
| T03 | chrome-rig | — | ⬜ | |
| T04 | page-map-only | T03 | ⬜ | |
| T05 | page-live | T04 | ⬜ | |
| T06 | live-reader | — | ⬜ | |
| T07 | secrets-research | — | ⬜ | |
| T08 | map-command | T02, T05 | ⬜ | |
| T09 | secrets-layer | T07, T08 | ⬜ | |
| T10 | app-live | T02, T05, T06 | ⬜ | |
| T11 | app-show-map | T08, T10 | ⬜ | |
| T12 | map-drill | T09, T11 | ⬜ | |

**Review queue:** empty

## Blocked on the user

Nothing.
