# amiga-plan

The route planner's game-free half: a whole-level explorer (archive of cells, random bursts, compasses, guides, shortening, repair) over any engine that implements `PlanGame`, plus the shell that runs a round on threads under a wall-clock budget. `core` (re-exported at the crate root) is pure: no clock, file, environment, process or network, enforced by `tests/purity.rs`, so a search is a function of its start, facts and options and gives the same route on any thread count. If the purity test fails, move the code to `run`; never relax the test. A game plugs in through a newtype around its engine in its own planner crate.

## Start here

Implement `PlanGame` and plan (from `tests/made_up_game.rs`):

```rust
impl PlanGame for MyEngine { /* step, state_bytes, cell, lives, dying, health, playing,
    collectibles_left, held_flag, landed, open, collectible_live, pressed, const ACTS */ }

let facts = Facts { cols, rows, exits, collectibles, gates };
let opt = Options { compass: Compass::Walk, ..Options::default() };
let run = RunOptions { threads: 4, budget: Duration::from_secs(120), ..RunOptions::default() };
match plan_engine(start, facts, opt, &run) {
    Ok(route) => route.inputs,              // Vec<FrameInput> from the start to the exit
    Err((best_partial, reason)) => ...,
}
```

Check a route: `shorten::replay_exit(&start, &route.inputs, &mut frames) -> Option<(usize, u8)>` (frames to the level's end, health lost).

Gotchas: `Clone` is the planner's snapshot and must step exactly as the original state would. `seed_inputs` replays joystick only. The same seed gives the same route at any `threads`; a run stopped by `budget` is not held to that.

## API by job

The game interface (`core`)
- `trait PlanGame: Clone + Send + Sync`: `step(&mut self, i: FrameInput)`, `state_bytes`, `cell() -> Cell`, `lives() -> i16` (signed: a game-over mark below zero reads as fewer), `dying`, `health() -> u16`, `playing`, `collectibles_left() -> u16`, `held_flag`, `landed`, `open(c: Cell)` (read once by the walk compass), `collectible_live(c: Cell)`, `pressed(&self, f: &Facts) -> u32`, `const ACTS: &'static [Joy]`.
- `Facts { cols, rows, exits, collectibles, gates: Vec<Vec<Cell>> }`; `type Cell = (u16, u16)`.

Searching (`explore`, `run`)
- `Options { seed, max_frames, max_mem_bytes, bursts_per_round, compass, guide, guide_pace, guide_ordered, seed_inputs, goal, via, avoid }`, defaults seed 1, 30 000 frames, 4 GiB, 80 bursts.
- `Compass::{Straight, Map(MoveMap), Walk}`; `Goal::{Exit, Key { key, within_frames }, Reach { point, within_frames, hearts }}`.
- `run::plan_engine<G: PlanGame>(start: G, facts: Facts, opt: Options, run: &RunOptions) -> Result<Route, (Route, Reason)>`.
- `RunOptions { threads, budget, shorten_share, shorten_min_rounds, shorten_min_over, max_rounds, dead_end, log }`; `RunOptions::shortening()` turns shortening on (`SHORTEN_SHARE`, `SHORTEN_MIN_ROUNDS`).
- `Route { inputs, engine_frames, hearts_lost, rounds, cells }`; `Reason { coins_left, exit_distance_with_coins_met, cells }`.
- Lower level: `Explorer::new(start, facts, opt)`, `round_parts(n)`, `run_part(&part)`, `merge(results) -> Progress` (`Working` or `Exit(Route)`), `best_partial()`, `found()`, `archive()`; `run::run_round(ex, threads)`, `run::shortcut_pass(...) -> PassResult`, `run::dead_end(rounds, cells)`, `run::over_slack(frames)`, `run::peak_rss_bytes()`.

Guides and repair
- `GuidePoint { cell, coins_left, switches }` (`covers(&CellKey)`); `guide_file::parse_waypoints(text) -> Result<Vec<GuidePoint>, String>` reads `col,row[,coins_left]` lines.
- `repair::points_from_trace`, `repair::follow_options` (an old route as `seed_inputs` and guide), `repair::Sampler` (`due`, `sample`, `take` for a machine that is not a `PlanGame`), `repair::trace_engine`.

Maps and compasses
- `MoveMap::new(nodes: Vec<Node>, edges: Vec<Edge>)`, `shortest(map, from, switches, wings) -> Vec<u32>`.
- `walk::WalkCompass::new(&g, &facts)`, `from_grid`, `exit_steps(&mut self, key: &CellKey) -> Option<u16>`, `walk::fill`.

Archive: `CellKey { col, row, band, switches, wings, via }` (`col`, `row` are the player's cell halved: a key is 2x2 cells), `cell_key(facts, g, via)`, `band(coins_left)`, `Archive`. Field names `coins`, `hearts`, `wings` mean collectibles, health and the held flag.

## Where the details are

`src/lib.rs` and `src/core/mod.rs` (`PlanGame`, `Facts`), `src/core/explore.rs`, `src/run.rs`, `src/core/shorten.rs`, `src/core/repair.rs`; `tests/made_up_game.rs` (a whole game in 250 lines); `toolkit/docs/explorer-go-explore-idea.md`, `toolkit/PLAYBOOK.md`. A planner's routes are drawn by `amiga-routeview`.
