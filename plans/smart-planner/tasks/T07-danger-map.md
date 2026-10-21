# T07 — danger-map

**Phase:** 1 · **Depends on:** T03 · **Weight:** light

## Goal

Keep the frog's hearts: judge a move's danger by playing it on a copy of the live state, enemies
included, and when it costs a heart, find the earliest short wait after which it does not. Serves M6
now and hard mode later. No pre-sampled enemy timetable (DESIGN D14: enemies spawn as their edge
scrolls in, so a parked-frog sample misses them and their phase). The slug is kept from the original
plan; the task is the live check.

## Design sections this implements

DESIGN §2.5, D14.

## Files

- `engine/crates/sf-plan/src/core/danger.rs`, `engine/crates/sf-plan/tests/danger.rs`

## Interface

```rust
pub const WAITS: [u16; 3] = [12, 25, 50];                    // frames idled on the start node before a retry (§2.5)
pub struct Outcome { pub end: Game, pub hearts_lost: u8, pub died: bool }
/// Plays `inputs` on a clone of `start`; hearts read as the old planners read them (a 1-up is not a loss).
pub fn play(start: &Game, inputs: &[(u8, u16)]) -> Outcome;
/// `play`, and if a heart is lost, the same inputs after each of `WAITS` idle frames in order; returns
/// the first variant that loses nothing with its wait (`Some(0)` when the move itself is safe), else
/// the original outcome and `None`.
pub fn play_safe(start: &Game, inputs: &[(u8, u16)]) -> (Outcome, Option<u16>);
/// Cost the search adds for a lost heart: larger than any detour `max_frames` allows.
pub fn heart_cost(max_frames: u32) -> u64;
```

Fixed hazards are `Facts::hazards` (T03) and kept off the map by T04; this task does not handle them.

## Tests

- [ ] W1-1, a fixture where walking into a patrolling enemy loses a heart: `play_safe` returns a wait
      from `WAITS` after which the same walk loses nothing.
- [ ] A move with no enemy near loses nothing, and no retry is played.
- [ ] A 1-up gained is not counted as a heart lost.
- [ ] Deterministic for the same start state.

## Done when

- [ ] Tests green; `play_safe` on a 40-frame move with all three retries recorded on W6-3 (expected
      ~0.12 s at ~1 300 frames/s; retries run only after a heart is lost).
