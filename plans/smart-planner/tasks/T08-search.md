# T08 — search

**Phase:** 1 · **Depends on:** T04, T06, T07 · **Weight:** heavy

## Goal

The planner proper, on the engine alone: follow the chosen coins along the map by playing whole moves,
fall back to frame-level search where the live game differs from the map, keep one archive for the
whole level, avoid hits, and return the same route for the same seed. Phase 2 adds the original.

## Design sections this implements

DESIGN §2.1 (goal, boss), §2.10 (via/avoid in the search), §2.4, §2.5 (use), §2.3 (re-choice), §2.9 (budget, unreachable count), §3.1
(threads as partitions).

## Files

- `engine/crates/sf-plan/src/core/search.rs`, `engine/crates/sf-plan/src/core/archive.rs`
- `engine/crates/sf-plan/src/run.rs` (shell: runs partitions on `std::thread::scope`, enforces the
  wall-clock budget by asking the core to stop between steps)
- `engine/crates/sf-plan/tests/search.rs`

## Interface

```rust
pub struct Options { pub seed: u64, pub max_frames: u32, pub beam: usize, pub hold: u8,
                     pub via: Vec<Cell>, pub avoid: Vec<(Cell, Cell)> }   // hints, DESIGN §2.10; empty when unaided
pub struct Planner { /* facts, map, danger, choice, archive, rng(seed) */ }
impl Planner {
    pub fn new(start: Game, facts: Facts, map: MoveMap, danger: Danger, opt: Options) -> Result<Planner, Unplannable>;
    /// One unit of search work, split into `parts` for the shell's threads.
    pub fn step_parts(&mut self, n: usize) -> Vec<Part>;
    pub fn run_part(&self, part: &Part) -> PartResult;       // pure; called from any thread
    pub fn merge(&mut self, results: Vec<PartResult>) -> Progress; // in part order; Progress::{Working, Done(Route), Stuck(Reason)}
    pub fn best_partial(&self) -> Route;
}
pub struct Route { pub inputs: Vec<FrameInput>, pub engine_frames: u64, pub hearts_lost: u8 }
pub fn plan_engine(start: Game, facts: Facts, map: MoveMap, danger: Danger, opt: Options,
                   threads: usize, budget: Duration) -> Result<Route, (Route, Reason)>; // shell, run.rs
```

A path dies on life lost (1-up gained is not a loss), `DYING`, or a caught engine panic. Done means
`level_state` left −2 by the exit and the frog survives a 40-frame idle tail.

## Tests

- [ ] W1-1 from its `AtLevel` state transplanted (engine only): `plan_engine` reaches the exit within
      the test's 2-minute cap.
- [ ] Same seed twice → byte-identical `inputs`, with 1 and with 8 threads (M8 at core level).
- [ ] A synthetic case where the map path is blocked by a live enemy: the beam fallback finds a way and
      the result loses no heart.
- [ ] The archive: after taking a coin, the next target starts from an archived state (counter of
      archive hits > 0 on W1-1).
- [ ] `Unplannable` at once when reachable coins < needed.
- [ ] `via` cells are passed in order before the remaining coin targets; a path entering an `avoid` box dies.

## Done when

- [ ] Tests green; W1-4 and W6-3 planned engine-only from their starts within 30 min each, times in a
      FINDINGS row (not gated here; the gate is T14).
