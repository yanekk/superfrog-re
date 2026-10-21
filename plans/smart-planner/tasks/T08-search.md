# T08 — search

**Phase:** 1 · **Depends on:** T04, T06, T07 · **Weight:** heavy

## Goal

The planner proper, on the engine alone: follow the chosen coins along the map by playing whole moves,
fall back to frame-level search where the live game differs from the map, keep one archive for the
whole level, avoid hits by playing moves live with wait-and-retry (T07), explore when stuck instead of
declaring a level impossible, learn edges the map lacks and distrust edges that keep failing, and return
the same route for the same seed. Phase 2 adds the original.

## Design sections this implements

DESIGN §2.1 (goal), §2.10 (via/avoid in the search), §2.2 (learned and unreliable edges), §2.4
(including exploring when stuck), §2.5 (use), §2.3 (re-choice), §2.9 (budget, no early exit), §3.1
(threads as partitions), D14, D16, D17.

## Files

- `engine/crates/sf-plan/src/core/search.rs`, `engine/crates/sf-plan/src/core/archive.rs`
- `engine/crates/sf-plan/src/core/explore.rs` (the explore step: pick a least-visited archived state,
  play toward novelty; T13 adds its goal-free entry point to this file)
- `engine/crates/sf-plan/src/run.rs` (shell: runs partitions on `std::thread::scope`, enforces the
  wall-clock budget by asking the core to stop between steps)
- `engine/crates/sf-plan/tests/search.rs`

## Interface

```rust
pub struct Options { pub seed: u64, pub max_frames: u32, pub beam: usize, pub hold: u8,
                     pub stall_steps: u32,      // steps without progress before exploring (§2.4)
                     pub explore_every: u32,    // 1 step in N explores while stalled; default 5
                     pub via: Vec<Cell>, pub avoid: Vec<(Cell, Cell)> }   // hints, DESIGN §2.10; empty when unaided
pub struct Planner { /* facts, map, choice, archive, learned/unreliable edges, rng(seed) */ }
impl Planner {
    pub fn new(start: Game, facts: Facts, map: MoveMap, opt: Options) -> Planner;   // never refuses on map evidence (D16)
    /// One unit of search work, split into `parts` for the shell's threads.
    pub fn step_parts(&mut self, n: usize) -> Vec<Part>;
    pub fn run_part(&self, part: &Part) -> PartResult;       // pure; called from any thread
    pub fn merge(&mut self, results: Vec<PartResult>) -> Progress; // in part order; Progress::{Working, Done(Route), Stuck(Reason)}
    pub fn best_partial(&self) -> Route;
}
pub struct Route { pub inputs: Vec<FrameInput>, pub engine_frames: u64, pub hearts_lost: u8,
                   pub learned_edges: Vec<Edge>, pub unreliable_edges: Vec<u32>, pub explore_steps: u64 }
pub fn plan_engine(start: Game, facts: Facts, map: MoveMap, opt: Options,
                   threads: usize, budget: Duration) -> Result<Route, (Route, Reason)>; // shell, run.rs
```

Every edge is played through `danger::play_safe` (T07); a heart lost after all retries is scored at
`heart_cost`. An edge that fails live replay (wrong landing node, or a heart lost after retries) 3
times in a run is marked unreliable and its cost ×4 for the rest of the run (D17). A node reached in
play that the map lacks, or reached by a cheaper input run than the map knows, is added as a learned
edge from the nearest map node (D16). The stall counter and the explore share count search steps, never
time (M8).

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
- [ ] A fixture map with one edge removed (the only way to a needed coin): the search explores, learns
      the edge, reaches the exit, and lists the edge in `learned_edges`. No early give-up.
- [ ] An edge made to fail live 3 times (fixture with a blocking object) is in `unreliable_edges` and
      the route avoids it when another path exists.
- [ ] A synthetic enemy crossing the path: the route waits (one of `danger::WAITS`) and loses no heart.
- [ ] `via` cells are passed in order before the remaining coin targets; a path entering an `avoid` box dies.

## Done when

- [ ] Tests green; W1-4 and W6-3 planned engine-only from their starts within 30 min each, times in a
      FINDINGS row (not gated here; the gate is T14).
