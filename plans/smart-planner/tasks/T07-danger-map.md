# T07 — danger-map

**Phase:** 1 · **Depends on:** T03 · **Weight:** medium

## Goal

Know where and when enemies are, so the search prefers moves that take no hit and waits for gaps.
Serves M6 now and hard mode later.

## Design sections this implements

DESIGN §2.5.

## Files

- `engine/crates/sf-plan/src/core/danger.rs`, `engine/crates/sf-plan/tests/danger.rs`

## Interface

```rust
pub struct Danger { /* per cell: sorted frame intervals occupied by an enemy hitbox, relative to sample start */ }
/// Steps a copy of `start` for `frames` frames with the frog parked where enemies cannot reach it
/// (the method T00/T04 found for holding the frog), recording enemy hitbox cells per frame.
pub fn sample(start: &Game, facts: &Facts, frames: u32) -> Danger;
/// Cost of occupying `cells` over absolute frames `at..at+len`, given the sample's start frame and the
/// enemies' period where one is detected; 0 when clear.
pub fn cost(d: &Danger, cells: &[(Cell, u32 /*frame offset*/)], at: u32) -> u32;
```

Enemy hitboxes are read from the object slots (h0+$1ba6c, 20 × $7c) as the old planners read them.

## Tests

- [ ] W1-1: a patrolling enemy's cells are occupied periodically; a cell no enemy visits costs 0.
- [ ] `cost` is non-zero for a path crossing an enemy at the sampled frame and zero one period-half later.
- [ ] Deterministic for the same start state.

## Done when

- [ ] Tests green; sampling W6-3 for 2 000 frames takes under 10 s (recorded).
