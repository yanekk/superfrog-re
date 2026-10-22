# T08 — search

**Phase:** 1 · **Depends on:** T04, T06, T07 · **Weight:** heavy

Rewritten by the 2026-10-22 amendment (D19): the search is a whole-level explorer, not map following.
The experiment it is built from is in `plans/smart-planner/spike-explore/` (`goexp.rs`, a reference
only, and its README with every number). Read both first.

## Goal

The planner proper, on the engine alone: one archive for the whole level, explored in rounds of
random bursts chosen by novelty, progress in coins, an exit pull and (T09) a guide, until the frog
leaves the level alive; deterministic for a seed; memory bounded. Phase 2 adds the original.

## Design sections this implements

DESIGN §2.1 (goal), §2.4, §2.5, §2.9 (budget, memory, reason line), §2.10 (`via`/`avoid` in the
search), §2.2 (the `Compass` hook, straight distance by default), §3.1 (threads as parts), D19, D22.

## Files

- `engine/crates/sf-plan/src/core/archive.rs` (cells, entries, counters, checkpointed states)
- `engine/crates/sf-plan/src/core/explore.rs` (rounds, choice, bursts, exits, `Compass`)
- `engine/crates/sf-plan/src/core/mod.rs`
- `engine/crates/sf-plan/src/run.rs` (shell: runs parts on `std::thread::scope`, enforces the
  wall-clock budget by asking the core to stop between rounds)
- `engine/crates/sf-plan/tests/explore.rs`
- `engine/crates/sf-plan/src/cache.rs` only if a new core module joins `cache::CORE_FILES` (T05 review
  FINDINGS row: a module `map.rs`, `facts.rs`, `graph.rs` or `moves.rs` uses). The explorer's modules
  are not map inputs and do not join it.

## Interface

```rust
pub struct Options {
    pub seed: u64, pub max_frames: u32, pub max_mem_bytes: u64,
    pub bursts_per_round: u32,                   // fixed by options, never by the machine (M8); default 80
    pub compass: Compass,                        // Compass::Straight default; Compass::Map(MoveMap) optional
    pub guide: Vec<GuidePoint>,                  // T09 fills it; empty = unguided
    pub seed_inputs: Option<Vec<FrameInput>>,    // T09's --follow seed; None here
    pub via: Vec<Cell>, pub avoid: Vec<(Cell, Cell)>,   // hints, DESIGN §2.10; empty when unaided
}
pub struct Explorer { /* facts, archive, rng(seed), counters */ }
impl Explorer {
    pub fn new(start: Game, facts: Facts, opt: Options) -> Explorer;
    /// One round's bursts, split into `n` parts for the shell's threads.
    pub fn round_parts(&mut self, n: usize) -> Vec<Part>;
    pub fn run_part(&self, part: &Part) -> PartResult;          // pure; called from any thread
    pub fn merge(&mut self, results: Vec<PartResult>) -> Progress;  // in part order
    // Progress::{Working, Exit(Route)}; the shell decides when to stop on time
    pub fn best_partial(&self) -> (Route, Reason);
}
pub struct Route { pub inputs: Vec<FrameInput>, pub engine_frames: u64, pub hearts_lost: u8, pub rounds: u64, pub cells: u32 }
pub struct Reason { pub coins_left: u16, pub exit_distance_with_coins_met: Option<u16>, pub cells: u32 }
pub fn plan_engine(start: Game, facts: Facts, opt: Options, threads: usize, budget: Duration)
    -> Result<Route, (Route, Reason)>;          // shell, run.rs
```

`GuidePoint` is declared here (`core::guide`, one struct: `cell: Cell, coins_left: Option<u16>`) so
T09 only fills it; the weight that reads it is T09's.

The key, choice weights, burst shape and exit test are DESIGN §2.4's, constants as there. A heart lost
is not a burst's end (§2.5): the state is archived and loses to its cell's more-hearted entries.

Memory (D22): states are stored for checkpoint entries only; any other entry's state is rebuilt by
replaying its inputs from its nearest checkpoint ancestor. Pick the checkpoint policy, measure it on
W3-4 (the experiment's biggest archive, 39 000 cells in 10 min), and record it and its RSS in
FINDINGS.

## Tests

- [ ] W1-1 from its start (engine only): `plan_engine` reaches the exit within the test's 2-minute cap.
- [ ] Same seed twice → byte-identical `inputs`, with 1 and with 8 threads (M8 at core level).
- [ ] An entry's rebuilt state equals the state it was stored from (`Game` state bytes equal) after
      checkpoints are dropped.
- [ ] A synthetic archive where a cell is reached with fewer and with more hearts keeps the more-hearted
      state; a burst that loses a heart still archives its state.
- [ ] The band key: `COINS_LEFT` 0 is band 0, 1–4 band 1, 5–8 band 2.
- [ ] `via` cells are passed in order before the search's own progress resumes; a burst entering an
      `avoid` box ends there.
- [ ] `Compass::Map` (distance in edge frames to the nearest exit node, `graph::shortest` over T04's
      map, straight distance where the map has no path) gives the explorer a finite pull on W1-1 and
      planning still exits; `Compass::Straight` needs no map.
- [ ] With the memory cap set low (fixture), resident archive bytes stay under it and planning W1-1
      still reaches the exit.

## Done when

- [ ] Tests green; engine-only, unguided, 10 threads, 10-minute cap, quiet machine: W3-2, W5-2 and W6-1
      reach the exit (the experiment did, in 340, 62 and 211 s), times, cells and peak RSS in one
      FINDINGS row. Peak RSS stays under `--max-mem-gb`'s default on all three.
