# T43 — slot-prizes

**Phase:** 2 · **Depends on:** T42; blocks T21 · **Weight:** medium

Added by T19 (owner, 2026-10-13). The prize ladder, gamble, hi-lo game and jackpot, each in a green
case; the slot's documentation. Completes what T19 was for.

## The original (h0 offsets)

- Award $51c4 (d7 = rung): win 0; −1 returns; lamps off; ladder lamps ($57b6: 10 pointers best
  first, the 10th the bust lamp $688a, then 0) cleared (+0, +6); $a → hi-lo $57e2 (back with d7:
  0 → offer the jackpot rung, else collect). $520a: rung 0 win 0; 1 win = score shown (double);
  2 a life; 3..8 += 75 000, 50 000, 40 000, 30 000, 20 000, 10 000; rung lamp +6 set; sfx 8−rung.
  Scan for the lit rung k: k = 9 → bust ($5712: sfx 9, win 0, 20 × the bust lamp); k = 0 → every
  lamp 20 × then the jackpot; else flashing = rung k−1, rung min(k+3, 9), $6892, $68d2, $68da; lit
  = rung k; offer ($615c = 1); menu. Collect ($5528): sfx $b, win paid (no sound). Gamble: rng(100)
  < 50 → rung −1 and offer again; else sfx 9, rung +3: bust if the bust lamp was offered, else 10 ×
  its lamp, collect.
- Hi-lo $57e2: mode $6152, rung 8; each turn sfx 8−rung, flashing $68d2, $68ba, $68ca, $68da,
  $68e2, rung lamp; spin's button takes (and costs a coin if any: the menu's spin path, kept);
  higher/lower: 10 frames, the number turns rng(50)+50 frames, skipping its start ($598e); right →
  rung −1 (at 0 taken), wrong → sfx 9, 20 × (clear win, bust lamp), d7 −1. Take: sfx $a.
- Jackpot $5628: each coin left counted as 5 000, then 150 000, sfx $10, the next level's code
  (`password_for_level` $2464: entry after world/level's) printed at buffer 0+$7abe, its lamp
  $6902 flashing until fire, sfx $15.
- Cases (scripts planned on the engine, then checked on the oracle; T19 wrote a planner that at
  each hi-lo turn tries "higher" on a cloned `Game` and keeps the winning guess). Offsets: with
  `start-w1-1-late` the first spin is `8 $18 0` (hi-lo); with `start-w1-1-blink1` the second spin
  allows holds: hold reel 1 → `8 8 $18` (20 000), hold reel 2 → `$18 $18 $18` (jackpot by line).
  Planned results: hi-lo taken at rung 8..0 = 10 000, 20 000, 30 000, 40 000, 50 000, 75 000,
  a life, double (of 0), 155 000 (jackpot + a coin); the gamble from 20 000 here loses (bust).
  A gamble win and the free spin are not reachable at these seeds: say so in FINDINGS.
- Script frames count from the sync point: engine frame f is the original's 758 + f.

## Files

- `engine/crates/sf-core/src/slot/payout.rs`, `engine/conform/{scripts,cases}/slot-*`,
  `docs/13-front-end.md` (a "slot machine" section), `re/symbols.toml`.

## Interface

No new type: the prizes fill T42's `SlotOutcome { award: Award { points, lives } }`.

## Tests

- [ ] Cases: `slot-hilo-0`..`8`, `slot-hilo-wrong`, `slot-hold-20000`, `slot-hold-jackpot`,
  `slot-gamble`, green; one of them in the test block.

## Done when

- [ ] Every prize on the ladder, the bust and the jackpot in a green case; docs/13 updated.
