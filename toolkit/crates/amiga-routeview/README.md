# amiga-routeview

The route viewer's game-free half: it writes a static web page (`index.html` plus `data/<id>.js` per route) that draws a game's routes over its level maps, with a switcher over the routes and their verdicts. A game's own tool replays each route on its engine and hands every level's data over as a `LevelDump`; this crate writes the page around them and decides each route's verdict from plain facts the game reads off its replay, so no game type enters it. A shell crate (it writes files) with no dependencies: its JSON is written by hand. It runs nothing itself, neither engine nor oracle, and a shipped app has no use for it.

## Start here

Write a site (from `tests/made_up_level.rs`):

```rust
impl LevelDump for MyLevel {
    fn level_id(&self) -> &str { &self.id }                 // [A-Za-z0-9._-], see site::id_of
    fn write_js(&self, out: &mut dyn Write) -> io::Result<()> { /* one JSON object */ }
    fn entry(&self) -> Entry { /* optional: label, group, route file, verdict */ }
}
let spec = PageSpec {
    title: "My Game".into(), cell_size: (16, 16),
    categories: vec![Category::new("gems", "Gems", "--accent", true, &["coin"]),
                     Category::new("rest", "Everything else", "#ffffff", true, &[])],
    actor: "hero".into(), anchor: (1, -1), cell_origin: (1, -2), live_box: (0, -2, 2, 2),
};
write_site(&root, &spec, &[&level_a, &level_b])?;
```

The page loads data by script tag, not fetch, so `index.html` opens straight from disk.

`write_js` must write the keys the page reads: `level`, `cols`, `rows`, `seed`, `timing`, `palette`, `tiles`, `body`, `objects`, `track`, `inputs`, `live`, `events`, `notes`, `coins_needed`, `end`, `inputs_end`, and optionally `guide`. Build it with `json::Json`.

Verdict: fill `verdict::Outcome` (and `Bench` when the route is the planner's), call `verdict::verdict`, then add `progress(&events, &facts)` stalls' `fact` strings to `info` for a route that reaches no exit.

## API by job

Page (`site`, re-exported)
- `PageSpec { title, cell_size, categories, actor, anchor, cell_origin, live_box }`: `anchor` is added to a track position to draw the line; `cell_origin` is added before dividing by the cell size to get the planner's cell.
- `Category::new(key: &str, name: &str, color: &str, on: bool, cats: &[&str]) -> Category`: `key` is `[a-z]+` and not `route`, `guide`, `heat`, `live` or `events`; an object goes on the first layer listing its cat, else the first with empty `cats`. Cats `coin`, `enemy`, `pickup`, `hazard`, `secret` get their own shapes.
- `Entry { id, label, group, route_file, verdict, wall_s, ref_frames }`.
- `trait LevelDump { fn level_id(&self) -> &str; fn write_js(&self, out: &mut dyn Write) -> io::Result<()>; fn entry(&self) -> Entry }`, `entry` defaulting to a clean verdict named by the id.
- `write_site(root: &Path, spec: &PageSpec, levels: &[&dyn LevelDump]) -> io::Result<()>`; parts: `site::write_level`, `site::write_data(out, id, json)`, `site::write_index(out, spec, entries, bare: bool) -> io::Result<PathBuf>` (`bare` drops the `<!doctype>`/`<head>` skeleton for a host that adds its own), `site::id_of(name) -> String`.

Verdict (`verdict`)
- `verdict::verdict(o: &Outcome, bench: Option<&Bench>) -> Verdict` (not re-exported at the root): `Status::{Clean, Faults, Failed}` (`word()`), `faults`, `info`. Faults: health lost, more than `LONGER_THAN_REF` (25%) over the reference route, planning over `SLOW_PLAN_S` (15 min).
- `Outcome<'a> { has_route, ended, exit_open, coins_left_at_end, lives_lost, hearts_lost, timing: &'a str }`; `Bench { completed, reason, wall_s, route_frames, ref_frames }`.
- `progress(events: &[Event], facts: &Facts) -> Vec<Stall>`: last new ground and closest approach to the open exit; empty when `events` hold an `end`. `Event { k, x, y, kind: &'static str, text }` with kinds `marker`, `hurt`, `death`, `jump`, `land`, `fire`, `end`, `stall`; `Facts { track, cells, inputs_end, exits, exit_open_at }`. This `Facts` is not `amiga_plan::Facts`.

Helpers
- `json::Json::object()`, `str`, `num`, `opt`, `bool`, `raw` (value already JSON), `finish() -> String`; `json::esc`, `json::list`, `json::strs`.
- `guide::parse(text) -> Vec<Waypoint>` (`c`, `r`, `t`, `est`): a planner's waypoint file with optional `t <secs>` and `est` in its comments.

## Where the details are

`src/site.rs` (the page and its embedded viewer), `src/verdict.rs`, `src/json.rs`, `src/guide.rs`; `tests/made_up_level.rs`. Routes come from `amiga-plan`.
