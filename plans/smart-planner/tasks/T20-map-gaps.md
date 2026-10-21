# T20 — map-gaps

**Phase:** 1 · **Depends on:** T04; blocks T08 · **Weight:** heavy

Added by T04 (owner, 2026-10-21): T04's D13 check found the measured map, with no enemy help,
reaching the exit with `coins_needed` object coins on only 12 of the 24 normal levels. The owner
chose to close the gaps before the search (T08) is built, not to leave them to its exploring.

## Goal

Every normal level's map (`core::map`, T04) reaches the exit through at least `coins_needed`
reachable object coins, or the level is shown to need an enemy (a stomp reward, a knock: D13) and
that is brought to the owner. The build stays within ~5 min a level on a quiet machine (10 threads).

## What T04 found

Per level (`cargo run --release -p sf-plan --example map_stats -- <level>`; FINDINGS T04 rows):

| Level | Map, T04 | Known |
|---|---|---|
| W2-1 | exit reached, 24 of 35 coins | The full per-switch-set walk (T04's earlier version) reached 60 of 65 from its walk out of the start with the gate written in; the coin shaft at 91,9-24 is reached by the pad at 91-92,35 (case `w2-1-complete`) |
| W2-3 | 71/66 coins, exit not | Never presses its fourth switch (gate 196,5) |
| W2-4 | 92 nodes, stuck in the start chamber | rust-engine FINDINGS T168: a pad throws ~25 rows; the hopper's two stomps make a reward. Possibly an enemy need (D13) |
| W3-2 | 28/99, exit not | not looked at |
| W3-4 | 52/99, exit not | not looked at |
| W4-2 | 98/99, exit not | The updraft at 63-66,74 lifts the frog into the upper level (case `complete-w4-2`, frames 1595-1746); the map's seeds there (68-70,75) sit among patrolling enemies and lose a heart on every move, waits of 12/25/50 too |
| W4-4 | 56/97, exit not | four switches |
| W5-2 | 147/99 coins, exit not | not looked at |
| W6-1 | 36/97, exit not | not looked at |
| W6-2 | 159/97 coins, exit not | two switches in order (rust-engine T150) |
| W6-4 | 137/99 coins, exit not | spikes at row 10 crossed only by flight |

The levels that pass: W1-1…W1-4, W2-2, W3-1, W3-3, W4-1, W4-3, W5-1, W5-3, W5-4, W6-3.

How to see where a map stops: `map_ascii <level> c0 c1 r0 r1` draws the map's nodes (`N`) over the
live markers; `facts_dump` gives exits and switches; `sf-conform record <case> --only --watch
x=h0+7d6:W --watch y=h0+7d8:W --watch og=h0+7fa:W` gives a committed route's frog cells on the
original (cell = ((x+$17)/16, (y−$10)/16)). T04 found each gap so far this way, by comparing the
map with the committed route.

## Design sections this implements

DESIGN §2.1 (the exit with the count met), §2.2, D13, D15, D18. Seeding stays by real moves (D18);
writing a switch's press into a state (T04) is the only write.

## Files

- `engine/crates/sf-plan/src/core/map.rs`, `engine/crates/sf-plan/src/core/moves.rs`
- `engine/crates/sf-plan/tests/map.rs`

## Interface

T04's, unchanged unless a gap needs it (a deviation recorded in PROGRESS).

## Tests

- [ ] `every_level_reaches_its_exit_with_its_coins` (T04, `#[ignore]`) passes, or lists only levels
      the owner has accepted as needing an enemy.
- [ ] `w4_2_exit_needs_the_switch_at_222_4` (T04, `#[ignore]`) passes.
- [ ] T04's block tests stay green and under ~30 s.

## Done when

- [ ] Tests green; per-level build time and frames for all 24 levels in one FINDINGS row.
- [ ] No level's map takes over 5 min to build on a quiet machine, or the owner has been asked.
- [ ] Any level still failing has been brought to the owner (an enemy need, D13) and the answer
      recorded.
