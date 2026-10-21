# T06 — coin-choice

**Phase:** 1 · **Depends on:** T03 · **Weight:** light

## Goal

Pick, before moving, the cheapest set of coins that meets the exit's count and the order to take them,
on map distances, so the planner stops walking into the hardest coins nearest-first. A coin the map
cannot reach is far, not impossible (D16), and coins an edge takes on the way count (D17).

## Design sections this implements

DESIGN §2.3, D16, D17.

## Files

- `engine/crates/sf-plan/src/core/coins.rs`, `engine/crates/sf-plan/tests/coins.rs`

## Interface

```rust
pub struct Choice { pub order: Vec<usize /*index into Facts::coins*/>, pub cost: u64,
                    pub far: Vec<usize> /*chosen or not, coins with no map path*/ }
/// Map frames between two targets, a coin being reached by the cheapest edge that takes it; `None` if
/// the map has no path. The caller (T08) builds it over the MoveMap.
pub type Dist<'a> = &'a dyn Fn(Target, Target) -> Option<u32>;   // Target::{Node(u32), Coin(usize), Exit}
/// Coins taken on the way along the map path between two targets (Edge::coins), for crediting.
pub type OnTheWay<'a> = &'a dyn Fn(Target, Target) -> Vec<usize>;
pub fn far_cost(map_max_path: u32) -> u64;                   // larger than any map path on the level
pub fn choose(n_coins: usize, needed: u16, from: u32, dist: Dist, on_the_way: OnTheWay) -> Choice;
```

Method: candidate set by cheapest insertion from `from` toward the exit until `needed` is met, a coin
with no map path costing `far_cost`; coins on the way between consecutive targets are credited and the
set trimmed of coins no longer needed; then 2-opt and swap-in/swap-out improvement until no gain,
bounded by an iteration count (not a clock). Always returns a choice (D16: no early "impossible"); a
choice containing far coins tells the search it must explore for them.

## Tests

- [ ] Line of coins: picks the `needed` nearest along the way to the exit, not a far cluster.
- [ ] A cheap cluster off to one side beats scattered near coins when its total is lower.
- [ ] Far coins are chosen only when reachable coins cannot meet `needed`, and are listed in `far`.
- [ ] A path that takes two coins on the way counts both: a choice needing 3 coins picks one target
      past them, not three.
- [ ] Same input, same output (order and cost), including on ties.
- [ ] 190 coins (W6 size) completes within 1 s in the test.

## Done when

- [ ] Tests green.
