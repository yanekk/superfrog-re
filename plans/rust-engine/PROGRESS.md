# Progress

**Update this whenever a task changes state.** It is the handoff between sessions; a stale
tracker costs the next session more than keeping it current ever saves.

**What the build taught lives next door in [FINDINGS.md](FINDINGS.md).** Read the rows
touching the task you pick up, and append yours there.

**Sixty words to a Notes cell, counted.** Flat prose. The cell is an index for the next session;
the account is the commit message. Whoever writes a cell also fixes the over-budget cell they walk past.

**Plan reviewed:** 2026-10-10 — 7 fixed, 7 decided with the user

**Status:** Plan written; nothing built.
**Last updated:** 2026-10-10
**Next `pir-work` will:** T00 oracle-spike, because every later test rests on the reference emulator; T01 and T02 can run beside it.

## Tasks

Legend: ⬜ not started · 🟡 in progress · 🔍 implemented, awaiting review · ✅ reviewed and
done · ⛔ blocked, needs a human.

| # | Task | Depends on | State | Notes |
|---|---|---|---|---|
| T00 | oracle-spike | — | ✅ | |
| T01 | rust-workspace | — | ✅ | |
| T02 | re-workbench | — | ✅ | |
| T03 | disk-import | T01 | ✅ | |
| T04 | atn-decompressor | T03 | ✅ | |
| T05 | oracle-harness | T00, T01 | ✅ | |
| T06 | routine-oracle | T04, T05 | ✅ | |
| T07 | exe-map | T02, T04, T05 | ✅ | |
| T08 | asset-decode | T04, T07 | ✅ | |
| T09 | asset-cache | T03, T08 | 🔍 | Cache codec, `decide`, hand-ported BLAKE3; 9 tests. Fresh import 287 ms, cache decode 43 ms (20 MB). Deviations: BLAKE3 hand-written in `src/blake3.rs` (crate builds with `cc`, off allowlist; crate is a dev-dep check). Speed test in `sf-conform/tests/asset_cache_speed.rs` (purity bans clocks in sf-assets tests). |
| T10 | paula-mixer | T01, T05 | ✅ | |
| T11 | audio-driver | T06, T07 | ⬜ | |
| T12 | core-state | T05, T06, T07 | ✅ | |
| T13 | renderer | T08, T12 | ⬜ | |
| T14 | frog-and-level | T06, T11, T12, T13 | ⬜ | |
| T15 | items-and-scoring | T14 | ⬜ | |
| T16 | enemies-worlds-1-3 | T15 | ⬜ | |
| T17 | enemies-worlds-4-6 | T15 | ⬜ | |
| T18 | bosses | T16, T17 | ⬜ | |
| T19 | slot-and-bonus | T15 | ⬜ | |
| T20 | front-end | T08, T11, T12, T13 | ⬜ | |
| T21 | game-flow | T18, T19, T20 | ⬜ | |
| T22 | app-rig | T09, T10, T12, T13 | ⬜ | |
| T23 | app-shell | T21, T22 | ⬜ | |
| T24 | macos-menus | T23 | ⬜ | |
| T25 | controls-and-gamepad | T24 | ⬜ | |
| T26 | save-states | T24 | ⬜ | |
| T27 | crt-and-widescreen | T24 | ⬜ | |
| T28 | level-override | T14, T24 | ⬜ | |
| T29 | app-drill | T21, T25, T26, T27, T28 | ⬜ | |
| T30 | acceptance | T29 | ⬜ | |

A Notes cell holds what was built or what the review found, the test count, and one line per
deviation from the task doc. A ✅ task's cell may be cut to one line once the next task has been reviewed.

**Review queue:** T09

## Blocked on the user

Nothing yet. Expected later: granting Accessibility and Screen Recording to the terminal (T22), the
8BitDo check (T25), the play session (T30), and the open decisions in PLAN.md once T07 reports.
