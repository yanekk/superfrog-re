# T128 — slot-lamp-cut

**Phase:** 2 · **Depends on:** T74 · **Weight:** medium

Added by T74 (owner, 2026-10-14). The slot machine draws its lamps (`slot_draw_lamps`, h0+$645e)
from the raster wait on line 299 into the next frame. Whether a lamp's blit starts before the
frame's interrupt or after it decides whether that lamp's memory is new at the frame's end, and in
the original that turns on a few colour clocks: the sixth lamp of an off-draw (every lamp off) is
cut when the fifth starts at 309.143 or later (payout flash, `slot-hilo-8` original frame 1090;
all-lamps flash, `slot-hold-jackpot` at fire 40, frame 1487) and is not cut at 309.137 (jackpot
flash, same run, frame 1492: the sixth starts at line 0, hpos 13). The engine's lamp model
(`slot::draw`: fitted cycle counts, `LAMP_GAP`, T74's `LAMP_OFF` and `LAMP_BLANK`, `CUT_LINE`)
gives every off-draw the same timing, so it matches one side and not the other. The difference is
most likely audio DMA slots (the jackpot's effect $10 starts on channel 3 two frames before), as
T96 found for the story's blits.

T74 landed `slot-hold-jackpot` with the attract's fire on frame 41 (green; fire 40 and 50 are
red, 41 and 44 green) and logged the limit in FINDINGS.

## Goal

1. Find what moves the original's lamp blits by those colour clocks (the oracle's bus owners:
   `set_dma_profile`, T36; the audio slots `sf_paula::AudioDma::frame` gives, as `story::sim` uses
   them).
2. Model it in the lamps drawn before the frame's interrupt, so the engine cuts the same lamps the
   original cuts. Prefer the existing slot-exact bus (`front::story::sim`) to a new fitted term.

## Interface

No new public interface: `slot::draw::Draw` keeps its `Part`s.

## Tests

- A case `slot-hold-jackpot-40`: `slot-hold-jackpot`'s path with the attract's fire on frame 40,
  its script planned by `cargo run --release --example slot_plan -- start-w1-1-blink1 spin hold2
  spin` (no `PLAN_FIRE`), green; and the same with `PLAN_FIRE=50`.
- Every other `slot-*` case and the slot tests in `sf-conform/tests/front.rs` still green.

## Files

`engine/crates/sf-core/src/slot/draw.rs`, `engine/conform/{scripts,cases}/slot-hold-jackpot-*`.

## Done when

- [ ] `slot-hold-jackpot-40` and the fire-50 run green, every slot case still green.
