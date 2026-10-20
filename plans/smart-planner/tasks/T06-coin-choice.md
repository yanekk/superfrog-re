# T06 — coin-choice

**Phase:** 1 · **Depends on:** T03 · **Weight:** light

## Goal

Pick, before moving, the cheapest set of coins that meets the exit's count and the order to take them,
on map distances, so the planner stops walking into the hardest coins nearest-first.

## Design sections this implements

DESIGN §2.3.

## Files

- `engine/crates/sf-plan/src/core/coins.rs`, `engine/crates/sf-plan/tests/coins.rs`

## Interface

```rust
pub struct Choice { pub order: Vec<usize /*index into Facts::coins*/>, pub cost: u32, pub unreachable: Vec<usize> }
/// `dist(a, b)` is map frames between cells (u32::MAX if none). Deterministic; ties broken by coin index.
pub fn choose(coins: &[Cell], needed: u16, from: Cell, exits: &[Cell], dist: &dyn Fn(Cell, Cell) -> u32) -> Option<Choice>;
```

Method: candidate set by cheapest insertion from `from` toward the nearest exit until `needed` is met,
then 2-opt and swap-in/swap-out improvement until no gain, bounded by an iteration count (not a clock).
`None` when fewer than `needed` coins are reachable.

## Tests

- [ ] Line of coins: picks the `needed` nearest along the way to the exit, not a far cluster.
- [ ] A cheap cluster off to one side beats scattered near coins when its total is lower.
- [ ] Unreachable coins listed and never chosen; `None` when the count cannot be met.
- [ ] Same input, same output (order and cost), including on ties.
- [ ] 190 coins (W6 size) completes within 1 s in the test.

## Done when

- [ ] Tests green.
