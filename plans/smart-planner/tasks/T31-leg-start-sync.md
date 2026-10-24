# T31 — leg-start-sync

**Phase:** 3 · **Depends on:** T26; blocks T17 · **Weight:** medium

Added by T26 with the owner's yes (2026-10-23).

## Goal

Make set B's W2-2, W5-1 and W6-3 leg starts faithful, so a route planned from them can pass the
faithful check, then re-plan those three guided in set B. The fix is to **where and how the leg's start is synced** (the leg
snapshot, the `SyncPoint` `sf-bench` verifies from, the start `sf-plan --snap` plans from, or the
engine's transplant of a snapshot taken there). It is never a loosening of the faithful check's
tolerances: no new `exclude`, no lag bar raised, no check skipped (DESIGN §1 Stance, §8). If the
only fix found would loosen the check, stop and ask the owner.

## What T26 found

`sf-bench --set B` W2-2 (leg-05, `game-whole` frame 48527): the route failed `not faithful:
diverged at frame 1`, `score_digit` ($c09a14) original $0, engine $ffff. The committed
`chain-w2-2` fails the same way from the same `SyncPoint::AtFrame`, so no route can pass. Cause:
W2-1's last pass leaves `vbl_count` 2, so W2-2's first pass begins inside `level_start`'s own
frame (rust-engine FINDINGS T209; `Game::step` handles it from power-on, `whole-w2-2-start`
guards it). A transplant at 48527 starts the engine one pass behind the original.

Synced at `game-whole` frame 47800 (W2-1 near its exit) with `game-whole`'s frames 47800..48527
spliced before the route, the committed `chain-w2-2` passes and exits. T26's planned route still
failed check 4 there (frame 1819: an item taken, marker 38 at 67,63, unmatched): it was planned
on the engine a pass behind, so W2-2 must be re-planned on the fixed start.

T26 checked every set-B leg the same way (the committed `chain-*` route, 200 frames from the
leg's `SyncPoint`): W5-1 (leg 17) and W6-3 (leg 23) diverge at frame 1 on the same `score_digit`
write; the other legs start clean (W1-1 and W5-2, which play standalone routes, completed in T26).

## Files

- `engine/crates/sf-bench/src/sets.rs`, `verify.rs` (the set-B start and its sync point), and/or
  `engine/crates/sf-plan/src/main.rs`, `run.rs` (the `--snap` start), and/or `sf-conform`'s
  `sync` or `sf-core`'s snapshot transplant, each change with a test that fails without it.
- `plans/smart-planner/bench/tuning/results.jsonl`, `RESULTS.md`: W2-2, W5-1, W6-3 set B rerun.
- `plans/smart-planner/FINDINGS.md`.

## Tests

- A test that the committed `chain-w2-2`, `chain-w5-1` and `chain-w6-3` pass the faithful check
  from their set-B starts as `sf-bench` builds them (each failing today at frame 1).
- Every other set-B leg's start unchanged, or, if the fix changes every leg's start, the legs
  T26 completed rerun and still completing.

## Done when

- [ ] The committed `chain-w2-2`, `chain-w5-1`, `chain-w6-3` pass the faithful check from their
      set-B starts, with no tolerance of the faithful check changed.
- [ ] W2-2, W5-1, W6-3 set B rerun guided (T26's command, `--levels W2-2,W5-1,W6-3`, run alone);
      each completes, or its failure is one FINDINGS row with cause brought to the owner.
- [ ] Or, if the only fix loosens the check, the owner was asked and the answer is in FINDINGS.
