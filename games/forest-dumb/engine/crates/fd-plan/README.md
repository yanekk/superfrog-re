# fd-plan

The route planner on W1-1 (plan `forest-dumb-level-1`, DESIGN §2.11): the toolkit's whole-level explorer
(`amiga-plan`) stepping fd-core's `Game` from `Start::Level1`. It finds a route (a joystick input per
frame), writes it as an input script, draws it on the map page (`fd-routeview`) and plays it on the
original through `fd-conform` to say for how many passes the two agree. It plans W1-1 only. It is not
the explorer itself (that is `toolkit/crates/amiga-plan`) and not a case runner: `confirm` builds one
case and hands it to `fd-conform`. `game.rs` and `facts.rs` are pure; `confirm.rs` and `main.rs` are shell.

## Start here

All from `games/forest-dumb/engine/`. Every call needs the owner's disks; `confirm` also needs the Kickstart.

1. Plan from the command line (`tests/route.rs`, `the_budget_stops_the_search_and_it_reports_where`):
   `cargo run --release -p fd-plan -- level W1-1 --rounds 400 --threads 4 --out DIR/route.txt --page DIR/page`
2. Plan from a test with a fixed round count, so it is the same on any machine (`tests/route.rs`):
   ```rust
   let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
   let run = RunOptions { threads: 4, budget: Duration::from_secs(600), max_rounds: Some(100), ..RunOptions::default() };
   let p = fd_plan::plan(&data, Options { compass: Compass::Walk, ..Options::default() }, &run);
   ```
3. Replay a route on the engine and read where it ends: `fd_plan::replay(&data, &p.route.inputs).location()`.
4. Play it on the original: `fd_plan::confirm::confirm(&data, &p.route.inputs)` gives a `fd_conform::Verdict`;
   `v.outcome` is `Outcome::Same` or `Outcome::Differs { pass, var, .. }`.
5. Write it as a script `fd-routeview --script` and cases read: `fd_plan::script_text(&["note".into()], &inputs)`.

## Command

```
fd-plan level W1-1 [--guide FILE] [--budget-mins N] [--out FILE] [--page DIR] [--rounds N]
        [--threads N] [--seed N] [--frames N] [--compass walk|straight] [--no-confirm]
```

- `level W1-1`: required, first; any other level is exit 2 ("only W1-1 is built").
- `--guide FILE`: waypoints, one `col,row` or `col,row,coins_left` a line, `#` comments (`amiga_plan::guide_file::parse_waypoints`). Read before the disks.
- `--budget-mins N`: wall-clock budget, a number of minutes (fractions allowed, `0` runs no round); default 30.
- `--rounds N`: stop after N rounds (`RunOptions::max_rounds`); use this, not the budget, in tests.
- `--threads N`: default the machine's parallelism. The route is the same on any thread count.
- `--seed N` (default 1), `--frames N` (`Options::max_frames`, default 30000), `--compass walk|straight` (default walk).
- `--out FILE`: the route script; default `games/forest-dumb/build/routes/W1-1.txt`.
- `--page DIR`: the map page; default `fd_routeview::default_out()` (`build/route-viz`).
- `--no-confirm`: skip the original.

Exit 0: a route to the exit. Exit 1: budget or rounds ran out; the furthest route is still written and
drawn, and stderr says `stalled at W1-1:c,r (N items left, …)`. Exit 2: bad arguments, a bad guide, or
missing disks. Stdout prints `page: …`, `route: …`, the agreement line, and last
`PLAN_STATS frames= rounds= engine_frames= cells= wall_s= peak_rss_mb=`.

## API by job

Planning (`lib.rs`):
- `start(data: &Arc<GameData>) -> PlanState`: the level's start.
- `plan(data, opt: amiga_plan::Options, run: &amiga_plan::run::RunOptions) -> Planned`: one search.
- `Planned { route: amiga_plan::Route, stalled: Option<amiga_plan::Reason>, end: Location }`: `stalled` is `None` at the exit. `route` has `inputs`, `rounds`, `engine_frames`, `cells`; `Reason` has `coins_left`, `cells`, `exit_distance_with_coins_met`.
- `replay(data, inputs: &[FrameInput]) -> PlanState`: inputs played from the start.
- `script(inputs) -> InputScript` and `script_text(notes: &[String], inputs) -> String`: the route as a script. Gotcha: a final line at frame `inputs.len()` lets go, so anything played past the route idles; `notes` become `# ` lines.

The engine as the planner sees it (`game.rs`):
- `PlanState(pub Game)`: a newtype implementing `amiga_plan::PlanGame` (the orphan rule bars implementing it on `Game`). `game() -> &Game`, `location() -> Location` (the character's cell). Its `step` drains `take_events()`.
- `ACTS: [Joy; 7]`: the burst's joystick states; none holds fire (the game reads none in play).
- `bcd(w: u16) -> u16`: a BCD word's value (`items_left` is BCD; `collectibles_left` decodes it).

The level (`facts.rs`):
- `facts(level: &fd_assets::Level) -> amiga_plan::Facts`: size, exit cells, item cells, no gates.
- `COLS` (240), `ROWS`, `EXIT` (0xff), `is_item(v)`, `is_mechanic(v)`, `solid(level, c, r)`. `open` reads the level's file, not play's collision plane.

On the original (`confirm.rs`):
- `confirm(data, inputs) -> Result<Verdict, String>` and `confirm_excluding(data, inputs, exclude: &[(&str, &str)])`.
- `case(data, inputs) -> Result<Case, String>` and `case_excluding(...)`: the case, synced at `sync::LEVEL_1_SYNC`, `freeze = true`, goal the frozen engine's `char_x`/`char_y`.
- `VARS`: the compared variables. `SLACK` (8): idle frames past the route, because the original ends its passes about one behind.
- `EDGE_LATCH_RIGHT = ("edge_latch_right", "T73")`: pass it to `confirm_excluding` only for a route that reaches that gap; an exclusion that hid nothing fails the case.

Gotcha: the case runs the original frozen (creatures, animated cells, springs), so where a route meets
a creature the two sides can differ; `tests/route.rs` shows how a test handles that today.

## Where the details are

- `src/lib.rs`, `game.rs`, `facts.rs`, `confirm.rs`, `main.rs` as above.
- `tests/route.rs`: the measured end, frames and coins of the 100-round search, kept current.
- `toolkit/crates/amiga-plan` (`src/run.rs` `RunOptions`, `plan_engine`; `src/guide_file.rs`).
- `plans/forest-dumb-level-1/DESIGN.md` §2.11; `games/forest-dumb/CLAUDE.md` "Planning a route"; `games/forest-dumb/docs/09-play-measured.md`, `10-object-system.md`.
- A stall goes to the owner to play: `games/superfrog/docs/planner-stall-ask-owner.md`.
