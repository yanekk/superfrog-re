# T121 — bob-draw-cut

**Phase:** 2 · **Depends on:** T101; blocks T21 · **Weight:** medium

Added by T43 (owner, 2026-10-14). With world 2's enemies live, the original's 25 Hz pass often
runs past a frame's end inside `bob_draw` (h0+$19120, the pass's phase 5): the runner's pacing reads
`Late { phase: 5, step: NO_STEP }` for long stretches (W2-2's `items-fruit-w2-2`, frames 261–310).
The engine draws every bob in one go (`level/objects.rs` `bob_draw`, `draw_part`; no `step_done`
inside), so its vertical blank comes after the whole drawing where the original's comes part way
through it. The first frame compared after such a stretch can differ in one bob's pixels of a
play-field buffer (`items-fruit-w2-2` frame 311: `playfield_buffer_0`, lines 177–186). Same lag model
as T101 (`level/frame.rs`, `level/beam.rs`).

## Goal

1. Give the bob drawer the original's cut points: a step per bob erased and per bob drawn (and its
   blits), so `Pacing::Late { phase: 5, step }` names where the original's frame ended and the
   engine ends its frame there, the vertical blank's work coming between the two parts.
2. Remove every pin it covers: each re-sync T43 pinned in a case (FINDINGS row "T43: pinned
   re-syncs"), making those cases `resync = false`.

## Interface

`bob_draw` gets the original's steps inside phase 5, so `Pacing::Late { phase: 5, step }` names a
real step; the case format is unchanged.

## Tests

- `items-fruit-w2-2` and `enemies-w2-2` green with `resync = false`, their pins gone from
  `frog_cases.rs` and `enemies_w2_cases.rs`.
- Every existing level case stays green.

## Files

`engine/crates/sf-core/src/level/{objects.rs,frame.rs,beam.rs}`; `engine/crates/sf-conform/src/sync.rs`
(the step the pacing reads in phase 5); the pinned cases and `sf-conform/tests/frog_cases.rs`.

## Done when

- [ ] Every pin in T43's FINDINGS row is gone and its case is green with `resync = false`.
- [ ] The existing level cases stay green.
