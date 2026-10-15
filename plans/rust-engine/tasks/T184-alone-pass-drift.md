# T184 — alone-pass-drift

**Phase:** 2 · **Depends on:** T182; blocks T21, T18, T115, T147, T148, T149, T150, T151, T154, T166, T167, T168 · **Weight:** medium

Added by T182 with the owner's yes, 2026-10-15 (DESIGN §2.1 check 4).

## Goal

The engine run alone keeps its 25 Hz pass in step with the original's through the original's late
frames, so the cases T182 found red alone play alone green.

Measured by T182 (2026-10-15, `sf-conform run --alone`): after some late frames, the original's
frame ending in phase 5 or 12 of its pass with that frame's Paula writes all made, the engine's pass
falls one frame off the original's (`vbl_count` 0 against 1) and never comes back, as nothing
re-syncs it. Play then parts a few hundred frames later. First differing variable, and first
differing milestone:

| Case | `vbl_count` first differs | First milestone that differs |
|---|---|---|
| `complete-w4-2` | 1927 | 7950, an item (marker 55) the original does not take |
| `enemies-w5-2-complete` | 2196 | 3261, a coin |
| `enemies-w6-1-complete` | 530 | 535, a coin |
| `enemies-w1-1` | 372 | 407, a coin |
| `enemies-w3-1`, `enemies-w3-activation` | 201 | 237, a coin |

`front-attract` also fails alone: the demo's score parts at 3609 and the run ends at 7450 against the
original's 7350. Find whether it is the same fault. The keyboard poll's phase after the engine's
instant loads (`kbd_toggle`, which moves INTENA writes and so where a late frame's write-counted cut
falls) is the other candidate T182 saw (`tests/alone.rs`, the W1-1 replay).

FINDINGS has the history: T101 (2026-10-14) found that cutting at a pass's end needs the original's
`vbl_count` in `Pacing::Late`, and that a check at each phase broke W1/W5 cases; T121's review lists
the phase-5 lags (W1-1 372, W3-1 200, W3-3 255, W3-activation 200, W6-1 530). Their owners, T131 and
T153, were dropped by the amendment.

Check 4's rule stands: the engine alone gets only the inputs, the original's pacing and its clock's
seeds. A fix may add to `Pacing` what the original's schedule says (as `Pacing::Late` already carries
the call and step), not state taken from its memory.

## Design sections this implements

DESIGN §2.1 checks 1 and 4.

## Files

- `engine/crates/sf-core/src/level/{frame.rs,mod.rs,beam.rs}`, `engine/crates/sf-core/src/game.rs`
  (`Pacing`) as needed.
- `engine/crates/sf-conform/src/sync.rs` (`pacing`) as needed.
- The five tests T182 left on checks 1–3 (`Alone::Off`, each naming T184): `enemies_w1.rs` `w1_1`,
  `enemies_w3.rs` `w3_1` and `activation`, `enemies_w5_cases.rs` `enemies_w5_2_complete`, `front.rs`
  `the_attract_cycle_matches_the_original`.

## Tests

- [ ] `sf-conform run --alone` on each case in the table and `front-attract`: green, or its first
      divergence a FINDINGS row with its cause and owner.
- [ ] Every case green under checks 1–3 before stays green (`sf-conform all`).

## Done when

- [ ] The five tests above run check 4 again (their `Alone::Off` removed), or each that cannot is a
      FINDINGS row with its cause and an owning task.
