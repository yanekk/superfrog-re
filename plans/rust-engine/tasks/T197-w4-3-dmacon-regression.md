# T197 — w4-3-dmacon-regression

**Phase:** 2 · **Depends on:** T194; blocks T21 · **Weight:** medium

Added by T196 with the owner's yes, 2026-10-16. `complete-w4-3` is in the test block
(`tests/frog_cases.rs`) and was green before T194 merged; since then it is red under checks 1–3
at 4546. T194 introduced the regression; the engine's fault under it is older.

## Goal

Make `complete-w4-3` green again under `sf-conform -- run` (checks 1–4), with no threshold changed
(DESIGN §2.1) and T194's fix kept (`enemies-w6-2-complete` stays green).

## What T196 measured

- Red at 4546, after the lag 4520–4545: `sfx_chan0` (`$c13f92`) channel 3's period `$98` on the
  original, `$80` on the engine, and its period ramp step 1 against 2.
- Frame 4543: the original ends at h0+$1b9c4 (phase 1, step 9) after 40 Paula writes, the last
  INTENA `$8010` at line 301. The engine lacks one of its DMACON `$8400` writes: the original's at
  line 252, after the copper interrupt's INTREQ `$10` at 249; the engine's frame has none there.
  So the engine's 40th write is `sfx_play`'s (a coin, phase 3), and its frame ends in `sfx_play`'s
  raster wait, where the original's never is.
- Before T194 that wrong held blank changed nothing. T194 runs `clr.l $20(a2)` before a held blank
  (as the original does), so the blank's `sfx_tick` now steps the old effect and the new one starts a
  ramp step on.
- At 48a8da3 (before T194) the case is green; on T194's merge (026e7f5) and with T196 it is red.

## Interface

None new.

## Files

- The engine's copper interrupt or blitter-priority code that drops the DMACON `$8400` write
  (`sf-core`, `level/scroll.rs` `copper_irq_blits` likely), and a test that fails without the fix.
- `engine/conform/cases/complete-w4-3.toml`: its comment, if what it says changes.

## Tests

- [ ] `complete-w4-3` green in `sf-conform -- run` (checks 1–4); `tests/frog_cases.rs`'s test green.
- [ ] A test pinning the engine's DMACON writes in frame 4543 against the original's.
- [ ] Every case green before stays green (`sf-conform -- all`).

## Done when

- [ ] `complete-w4-3` is green in the test block again, with no threshold changed.
