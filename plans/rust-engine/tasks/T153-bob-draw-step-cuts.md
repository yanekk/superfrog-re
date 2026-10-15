# T153 — bob-draw-step-cuts

**Phase:** 2 · **Depends on:** T121; blocks T21 · **Weight:** medium

Added by the T121 review (owner, 2026-10-15): T121's goal 1, which T121 did not build. T121 removed
the W2-2 pins (gone with T102's flag wait) and left `bob_draw` with no steps of its own, on the claim
that each erase's and draw's INTENA write already places a cut per bob (`Cut::writes`). A pin
remains straight after a frame the original ended inside the bob drawer: `enemies-w1-1` frame 372
(the case file calls it "T121's class"), `playfield_buffer_0` at $019bd0, with the original's frame
371 `Late { phase: 5, step: NO_STEP }`. Same lag model as T101 (`level/frame.rs`, `level/beam.rs`).

## Goal

1. Give the bob drawer the original's cut points (`level/objects.rs` `bob_draw`, `draw_part`,
   `erase`): a step per bob erased and per bob drawn (and its blits), and the pacing's step in
   phase 5 read from the original (`sf-conform/src/sync.rs`), so `Pacing::Late { phase: 5, step }`
   names where the original's frame ended and the engine ends its frame there. Include the case
   T121 left untested: a bob erased with nothing to erase makes no INTENA write.
2. Remove the bob-drawer pins: `enemies-w1-1` 372; and, after checking each against the original
   (`sf-conform record --every --only` shows each frame's pacing), whichever of `enemies-w3-1` 201,
   `enemies-w3-3` 256 and `enemies-w3-activation` 201 is a cut inside the bob drawer (each follows
   a phase-5 late frame; FINDINGS' T101 row puts `enemies-w3-1` 201 in phase 12 instead). A pin
   that is not the drawer's stays, with a FINDINGS row saying what it is.

## Interface

`bob_draw` gets the original's steps inside phase 5, so `Pacing::Late { phase: 5, step }` names a
real step; the case format is unchanged.

## Files

`engine/crates/sf-core/src/level/{objects.rs,frame.rs,beam.rs}`; `engine/crates/sf-conform/src/sync.rs`;
`engine/conform/cases/enemies-w1-1.toml` and the W3 cases above; `sf-conform/tests/enemies_w1.rs`,
`enemies_w3.rs`.

## Tests

- `enemies_w1::w1_1` with no re-sync at 372 (its case `resync = false` if it has no other pin).
- Each W3 pin found to be the drawer's gone from `enemies_w3.rs`, its case `resync = false` where
  none is left.
- A test that a frame cut at a phase-5 step ends the engine's frame between those two bobs.
- Every existing level case stays green, `items-fruit-w2-2` and `enemies-w2-2` among them.

## Done when

- [ ] `enemies-w1-1`'s pin at 372 is gone.
- [ ] Each of `enemies-w3-1` 201, `enemies-w3-3` 256, `enemies-w3-activation` 201 is gone, or kept
      with a FINDINGS row showing it is not a cut inside the bob drawer.
- [ ] The existing level cases stay green.
