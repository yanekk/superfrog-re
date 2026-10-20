# T09 — route-repair

**Phase:** 1 · **Depends on:** T08 · **Weight:** medium

## Goal

Start from an old route and re-search only where the new start pulls the frog off it: for whole-game
legs whose carried state differs from the standalone start, and for T10's stretch confirmation where
the original parts from the engine.

## Design sections this implements

DESIGN §2.7.

## Files

- `engine/crates/sf-plan/src/core/repair.rs`, `engine/crates/sf-plan/tests/repair.rs`

## Interface

```rust
pub struct Trail { pub cells: Vec<Cell> /*frog cell per input frame of the old route, from its own run*/, pub inputs: Vec<FrameInput> }
pub fn trail(start: &Game, old: &InputScript) -> Trail;
/// Replays `trail.inputs` from `start` while the frog stays within one cell of `trail.cells[i]`; returns
/// the index where it parted (or None if it played through).
pub fn follow(start: &Game, trail: &Trail) -> (Game, usize, Option<usize>);
/// Re-plans from the parted state to the nearest later trail cell by map distance, then follows on.
pub fn repair(planner: &mut Planner, trail: &Trail, parted_at: usize) -> Progress;
```

## Tests

- [ ] An old route replayed from its own start parts nowhere and is returned unchanged.
- [ ] The same route from a start with one input frame dropped (a carried-state stand-in): parts, is
      repaired, reaches the exit; the repaired route shares the old route's tail.
- [ ] Deterministic for one seed.

## Done when

- [ ] Tests green; one set-B leg (W2-1, carried RNG, FINDINGS T205) repaired engine-only from its
      standalone route, frames and time in a FINDINGS row.
