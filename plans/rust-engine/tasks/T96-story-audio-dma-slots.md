# T96 — story-audio-dma-slots

**Phase:** 2 · **Depends on:** T53; blocks T56 · **Weight:** medium

Added by T53, owner's yes 2026-10-13. Carries T53's "every draw of scenes 1 and 4" check, which
needs audio DMA.

## Goal

`story::sim` gives Paula's audio DMA its slots: the cycle a channel fetches a sample word takes
its DMA slot (`$0D`, `$0F`, `$11`, `$13` for channels 0–3), so blits and the CPU wait for it as
the reference emulator has them. The slots come from the channels' register writes (DMACON's
audio bits, `AUDxLC`, `AUDxLEN`, `AUDxPER`) and their cycles, through a port of vAmiga's audio
DMA state machine (`Paula/Audio/StateMachine.cpp`, `audxDR` serviced by
`Agnus::serviceDASEvent`).

## Found in T53

- Oracle frame 3 033's fixed DMA and blitter slots match `Fixed` exactly; 3 034 differs only by
  AUD0 at slot `$0D`. Channel 0's effect starts with DMACON `$8001` at line 48, `AUD0PER = $17d`,
  `AUD0LEN = $5c8`: two fetches on lines 49 and 50, then one every ~3.36 lines (`2 × PER`).
- That first blit after it (BLTSIZE `$2606`, oracle frame 3 033's draw) ends 14 cycles late.
  The profile's `owners` row labelled line *L* is line *L − 1*'s, and its codes are vAmiga's
  `BusOwner` (1 CPU, 2 refresh, 4–7 AUD0–3, 24 copper, 25 blitter).
- Scene 4's first effect: DMACON `$8001` in 6 240, line 17; `AUD0PER` and `AUD0VOL` rewritten
  every frame after.

## Interface

`sim::Fixed` (or a sibling the `Sim` asks) takes the audio writes with their cycles; the intro
feeds them from its sound driver's writes (T55 times them). Conformance feeds the oracle's.

## Files

`front/story/sim.rs` (or a new `front/story/paula_dma.rs`), `front/story/intro.rs`;
`sf-conform/tests/story_draws.rs`.

## Tests

- [ ] The audio slots a channel takes, against the oracle's DMA profile (`set_dma_profile`,
  `line_samples`), across scene 1's effect.
- [ ] `story_draws.rs` widened to every draw of scenes 1 (3 022..3 727) and 4 (6 192..6 439).

## Done when

- [ ] Given the original's draw start and memory there, every BLTSIZE write of every draw in
  scenes 1 and 4 lands on the original's cycle (T53's done-when, carried here).
