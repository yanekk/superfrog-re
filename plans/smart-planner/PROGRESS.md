# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale tracker
costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md)**: read the rows touching the
task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The cell is an index; the account is the commit
message. Whoever writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-20 — 9 fixed, 5 decided with the user
**Plan amended:** 2026-10-21 — D14–D17 (live danger, speed bands, explore instead of early give-up,
unreliable edges); T07 now light, T00 answers three more questions.
**Amendment reviewed:** 2026-10-21 — 6 fixes, 1 decision (a retry after a wait is a standing-start move)
**Plan amended:** 2026-10-22 — D19–D24: whole-level explorer core (T08–T10 rewritten), guide paths
allowed everywhere, map a compass; T13 and T20 dropped, T22 route-shorten added.
**Amendment reviewed:** 2026-10-22 — 18 fixes, 1 decision (owner: T10's W1-4 check runs guided)

**Plan amended:** 2026-10-24 — D25: a walk compass (wall-aware fill) joins T28's trial (owner).
**Status:** T00–T07, T19, T21 done; core rewritten by the amendment, not started.
**Last updated:** 2026-10-25
**Next `pir-work` will:** review T28 (compass-trial).

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | movement-map-spike | — | ✅ | |
| T01 | bench-harness | — | ✅ | |
| T02 | baseline-sample | T01 | ✅ | |
| T03 | level-facts | T00 | ✅ | |
| T04 | movement-map | T03 | ✅ | |
| T05 | map-cache | T04 | ✅ | |
| T06 | coin-choice | T03 | ✅ | |
| T07 | danger-map | T03 | ✅ | |
| T08 | search | T04, T06, T07 | ✅ | |
| T09 | route-repair | T08 | ✅ | |
| T10 | oracle-check | T02, T08, T09 | ✅ | |
| T11 | plan-command | T05, T10, T22 | ✅ | |
| T12 | app-timing | T11 | ✅ | |
| T14 | tuning-run | T11 | ✅ | |
| T15 | set-c-run | T12, T14 | ⬜ | |
| T16 | baseline-counted | T15 | ⬜ | |
| T17 | new-counted | T16 | ⬜ | |
| T18 | results-and-retire | T17 | ⬜ | |
| T19 | step-unseen-no-picture | T00; blocks T04 | ✅ | |
| T21 | engine-memory-lookup | T04 | ✅ | |
| T22 | route-shorten | T08 | ✅ | |
| T23 | w1-4-ending | T09, T10; blocks T14 | ✅ | |
| T24 | repair-cost | T14; blocks T15 | ✅ | |
| T25 | tuning-set-a | T14, T24; blocks T15 | ✅ | |
| T26 | tuning-set-b | T14, T25; blocks T15 | ✅ | |
| T27 | tuning-unaided | T14, T26; blocks T15 | ✅ | |
| T28 | compass-trial | T14, T27; blocks T15 | 🔍 | `core::walk` fill compass, `--compass walk`, 7 tests (`tests/walk.rs`, one ignored M8 route, passed). Trial 54 runs, `bench/compass-trial.md`: 5 of 18 each; walk 40.2 M frames against straight 65.2 M. Walk now `sf-plan`'s CLI default; `Options::default` stays straight. Deviation: full table in the bench file, the FINDINGS row summarises. |
| T29 | engine-sound-ramp | T25; blocks T15 | ✅ | |
| T30 | lag-repair | T25; blocks T15 | ✅ | |
| T31 | leg-start-sync | T26; blocks T17 | ✅ | |
| T32 | alone-pass-drift | T31; blocks T17 | ✅ | |
| T33 | dead-end-repair | T27, T30; blocks T17 | ⬜ | |

**Review queue:** *(empty)*

## Blocked on the user

Nothing yet. T12 needs `pir/bug-recorder` merged into main (DESIGN D3).
