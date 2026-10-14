# T120 — conform-no-silent-pass

**Phase:** 2 · **Depends on:** T05, T36; blocks T21 · **Weight:** medium

Added by the owner (2026-10-14), written by T43. Today, when the original never finishes a pass,
`sf-conform` reads it as endless lag (`sync::mid_pass`, `Outcome::straddled`) and skips the
comparison, so the case stays green: the T85 review found the old `items-upper-w2-1` (commit
`4789f2e`'s 1 480-frame script) had 468 such frames and compared nothing from frame ~1012 (FINDINGS,
T85 review). With world 2's enemies live the original lags far more (`enemies-w2-2`: 274 skipped
frames of 880), so the gap matters more.

## Goal

A conformance case can no longer pass while comparing nothing.

1. The runner counts, per case, the frames it actually compared and the longest run of skipped
   ones (mid-pass, straddled, after-lag), and prints both in its report (`run`, `all`, `scan`).
2. A case fails past a limit on the longest skipped run (and, if the data supports it, on the
   share of frames compared). Fit the limit to the real lag lengths measured in the existing cases.
3. Measure every current case. One that now fails is either fixed or pinned with a reason, as the
   re-sync pins are (`frog_cases.rs`, `enemies_w2_cases.rs`).

## Interface

`sf_conform::Outcome` gains the frames compared and the longest skipped run; `Case` gains a
per-case pin on the limit only if a case needs one. The case file format is otherwise unchanged.

## Tests

- A run whose skipped stretch passes the limit fails; one under it passes.
- The old `items-upper-w2-1` script (commit `4789f2e`) run as a case fails on the limit.

## Files

`engine/crates/sf-conform/src/{lib.rs,report.rs,case.rs}` (the counts, the limit, a per-case pin if
one is needed); the case files and tests that need a pin.

## Done when

- [ ] The old `items-upper-w2-1` script (commit `4789f2e`, 1 480 frames) fails.
- [ ] Every current case measured (compared frames, longest skipped run) and recorded; any that now
      fails is fixed or pinned with a reason.
- [ ] `sf-conform -- all` runs green.
