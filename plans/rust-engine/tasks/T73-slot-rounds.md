# T73 — slot-rounds

**Phase:** 2 · **Depends on:** T72 · **Weight:** medium

Added by T19 (owner, 2026-10-13). The machine played: rounds, spins, the payout check, the score's
count-up, the end. T19 had `slot-fire` green (807 frames) before it was discarded.

## Goal

From the attract's fire to the machine's return, for rounds that pay nothing or a line's prize
collected, frame-exact; `Scene::Slot` returns `SlotOutcome { award }`.

## The original (h0 offsets)

- Main loop $4d4a: DMACON $8020; unless the free-spin flag ($49d0): hold menu $5de2, spin $5a06;
  payout $4eae (d7 out); award $51c4 (T74; here: collect only); count $50b6; clr.l $49d2; free spin
  $4e28; loop while coins ($49c6, signed) ≥ 1. Then 100 frames or fire; fade out (list h2+$6688);
  blank list; score = $49d6; `music_fade_wait`; if `level` < 4: the bank copied back from h1 (CPU,
  $3e80 longs at ~39 cycles: four frames, `CpuWork`), `sfx_bank_init`, `music_start`; return.
  The original then goes on in the same frame (end cases one frame before the return).
- Hold menu $5de2: lamps off; holds and $615a cleared; if win ($57b2) = 0 and paid ($49da) = 0:
  rng(8) = 6 allows holds; clr $49da; $5e2e: flashing list $6160 = spin lamp $68d2 (+ hold lamps
  $68ca/$68c2/$68ba of unheld reels when allowed; held ones to the lit list $61cc), $615c = 0,
  menu. A hold toggles and rebuilds (the original pops the menu's return and branches).
- Menu $5eb4: clr $615e, cursor $6064 = $606a, flashing lamps out, $49d2's lamp on, draw, 8
  frames; loop: raster, every 6th frame not.w the flashing lamps, lit ones $ffff, draw; then
  JOY1DAT&$303: 3 = right (next stop, last at $6072), then (re-read) $300 = left; each move sets
  the pointer x to the stop, y $e2, waits 6 frames; fire or mouse ends it. Reads come after the
  draw, which spills into the next frame: read that frame's input. Dispatch on pointer x: < $46
  spin ($598c = 1; a coin if $615c = 0 and coins; digits); < $77 hold reel 1 / hi-lo higher;
  < $a8 hold 2; < $ea hold 3 / lower; else gamble when offered. A press that does nothing
  restarts the menu (the pointer x is not reset).
- Spin $5a06: lamps off, hold lamps from holds, draw; flag $5dc8 = (rng(100) ≥ 10 and no holds);
  10 frames; counters a = rng(64)+64, b = a+rng(64)+32, c = b+rng(64)+32 (longs $5dd6..); holds
  saved ($5dca..); each frame: raster; hi-lo number +1 every other frame (toggle $5dc6, wraps 11→1)
  unless all held; digits; unheld reels with counter ≥ 0 advance (mod 32); unheld with counter
  < $20 stop on 0, 9 or $18 ($5d48: position |1, hold, sfx $11+reel on channel 3); reels drawn;
  newly held reels' positions &~1; flag set and reels all $18: saved hold 3 cleared, counter 3 =
  $10 (+ counter 2 if ≥ 0); counters −1; until all < 0. Positions &~1, 3 frames, holds restored.
- Payout $4eae lines: `$18 $18 $18` rung 0 (sfx 8), `8 8 8` 4 (lamp $689a, sfx 4), `0 0 0` 5
  ($68a2, 3), `8 8 $18` 7 ($68aa, 1), `0 0 $18` 8 ($68b2, 0), `8 $18 0` hi-lo ($a, sfx $a); a line
  sets $49da, sfx $c on channel 2, then 10 × (lamps off, its lamp, 4 frames, draw, all on, 4
  frames, draw).
- Count $50b6: lamps off, draw; win ≥ 160 000 at once, else 1 000 a frame; the number right-aligned
  in 8 (`$5174`), printed at buffer 0+$7683 with `$6f3a` (stops at `PAGE`).
- Free spin $4e28: win ≠ 0 and coins: rng(100) < 6 → flag, 20 × (3 frames, $690a lit).

## Files

- `engine/crates/sf-core/src/slot/{mod.rs,reels.rs,payout.rs}`, `game.rs`, `scene.rs`
- `engine/crates/sf-conform/tests/slot_routines.rs`, `engine/conform/{scripts,cases}/slot-fire*`

## Interface

`Game::enter_slot(coins)`, `Game::slot_outcome()`;
`SlotOutcome { award: Award { points, lives } }`.

## Tests

- [ ] Routine conformance on Moira for every reel state (32³) of `$4eae` (flash loop at h0+$504e
  stubbed `rts`, `sfx_play` stubbed `move.b d0,(a5)+; rts`) and every reel/position of `$5d48`.
- [ ] `slot-fire`: fire pulses every 80 frames from frame 40 (`scripts/slot-fire.txt`), 807
  frames, green.

## Done when

- [ ] Both routine tests and `slot-fire` green; `Game::enter_slot(coins)` and
  `Game::slot_outcome()` give `SlotOutcome { award: Award { points, lives } }`.
