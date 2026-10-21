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

**Status:** plan written; nothing built.
**Last updated:** 2026-10-21
**Next `pir-work` will:** T00 movement-map-spike, the gate for everything on the map; T01 can run beside it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and done ·
⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | movement-map-spike | — | ✅ | |
| T01 | bench-harness | — | ✅ | |
| T02 | baseline-sample | T01 | ⬜ | |
| T03 | level-facts | T00 | ⬜ | |
| T04 | movement-map | T03 | ⬜ | |
| T05 | map-cache | T04 | ⬜ | |
| T06 | coin-choice | T03 | ⬜ | |
| T07 | danger-map | T03 | ⬜ | |
| T08 | search | T04, T06, T07 | ⬜ | |
| T09 | route-repair | T08 | ⬜ | |
| T10 | oracle-check | T02, T08, T09 | ⬜ | |
| T11 | plan-command | T05, T10 | ⬜ | |
| T12 | app-timing | T11 | ⬜ | |
| T13 | map-soundness | T01, T08 | ⬜ | |
| T14 | tuning-run | T11 | ⬜ | |
| T15 | set-c-run | T12, T13, T14 | ⬜ | |
| T16 | baseline-counted | T15 | ⬜ | |
| T17 | new-counted | T13, T16 | ⬜ | |
| T18 | results-and-retire | T17 | ⬜ | |
| T19 | step-unseen-no-picture | T00; blocks T04 | 🔍 | `Scene::step` takes `draw`; unseen skips `level_as_drawn`. 4 equality tests (W6-1, W1-4, W6-3, a lost-life route), 14 s. Deviation: the frame a level's loop ends is still drawn, because `Game::shown` keeps it; skipping differed at frame 1545. Speeds in FINDINGS. |

**Review queue:** T19

## Blocked on the user

Nothing yet. T12 needs `pir/bug-recorder` merged into main (DESIGN D3).
