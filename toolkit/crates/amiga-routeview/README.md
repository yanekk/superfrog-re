# amiga-routeview

The route viewer's game-free half: it writes a static web page (`index.html` plus `data/<id>.js` per route) that draws a game's routes over its level maps, with a switcher over the routes and their verdicts. A game's own tool replays each route on its engine and hands every level's data over as a `LevelDump`; this crate writes the page around them and decides each route's verdict from plain facts the game reads off its replay, so no game type enters it. A shell crate (it writes files) with no dependencies (its tests use `amiga-browser`): its JSON is written by hand. It runs nothing itself, neither engine nor oracle, and a shipped app has no use for it.

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
    live: None,                                             // or Some(LiveSpec::new(..)), below
};
write_site(&root, &spec, &[&level_a, &level_b])?;
```

The page loads data by script tag, not fetch, so `index.html` opens straight from disk.

`write_js` must write the keys the page reads: `level`, `cols`, `rows`, `seed`, `timing`, `palette`, `tiles`, `body`, `objects`, `track`, `inputs`, `live`, `events`, `notes`, `coins_needed`, `end`, `inputs_end`, and optionally `guide`. Build it with `json::Json`.

A map-only level (no route replayed) writes `"track": []` and its entry's `verdict: None`. Its data needs only `level`, `cols`, `rows`, `palette`, `tiles`, `body`, `objects` and `track`, and optionally `coins_needed` and `notes`. The page shows "Map only" in the verdict's place, draws the map and its object layers with no character, route line, timeline, Follow button or events list, hides its own route layers' chips (route, guide, heat, live, events), and its readout names the cell under the pointer.

Live mode: with `live: Some(LiveSpec::new(&[("items_left", "Items left")]))` the page re-reads `live.js` beside `index.html` every 250 ms (a script tag `live.js?t=…`, since a page opened from disk cannot fetch) and draws where play is. The running game writes it with `write_live(&dir, &frame)` about four times a second; the shell owns the clock (`t_ms`), the file is written to `live.js.tmp` and renamed. Chrome still reads it half replaced now and then (about 1 read in 950 under a writer replacing it thousands of times a second): such a read raises "Script error." and sets no frame, and the page keeps the last frame until it is stale. The page then shows a status line ("Playing <level> · <actor> at cell c,r" and the counters; "Not playing"; "App not running" when the file is missing or its `t` is more than `stale_ms` old by the page's clock; "Playing a level this map does not have"), draws the character's dot, each creature by its cat's shape and each changed cell with its value's look (0: the object hidden; a value no object of the level holds draws as `other`) on its own layer `play`, switches level when `level` changes (a hand pick holds until it changes again; nothing of play is drawn until the new level's data is in, then the view zooms in on the dot), and its Follow button, on at load, keeps the dot in view; a drag or a wheel turn on the map turns it off. With `live: None` the page never asks for the file.

Verdict: fill `verdict::Outcome` (and `Bench` when the route is the planner's), call `verdict::verdict`, then add `progress(&events, &facts)` stalls' `fact` strings to `info` for a route that reaches no exit.

## API by job

Page (`site`, re-exported)
- `PageSpec { title, cell_size, categories, actor, anchor, cell_origin, live_box, live: Option<LiveSpec> }`: `anchor` is added to a track position to draw the line; `cell_origin` is added before dividing by the cell size to get the planner's cell.
- `Category::new(key: &str, name: &str, color: &str, on: bool, cats: &[&str]) -> Category`: `key` is `[a-z]+` and not `route`, `guide`, `heat`, `live`, `events` or `play`; an object goes on the first layer listing its cat, else the first with empty `cats`. Cats `coin`, `enemy`, `pickup`, `hazard`, `secret` get their own shapes. An object of value 0 (a `secret`) has no object-map value, so a live cell change never hides it; while the layer holding `secret` is off, the map-only heading count and summary leave secrets out. A block of `exit` objects named `level exit` is labelled EXIT once.
- `Entry { id, label, group, route_file, verdict: Option<Verdict>, wall_s, ref_frames }`: `None` is a map-only level, the manifest's `status` `map`, with no faults.
- `trait LevelDump { fn level_id(&self) -> &str; fn write_js(&self, out: &mut dyn Write) -> io::Result<()>; fn entry(&self) -> Entry }`, `entry` defaulting to a clean verdict named by the id.
- `write_site(root: &Path, spec: &PageSpec, levels: &[&dyn LevelDump]) -> io::Result<()>`; parts: `site::write_level`, `site::write_data(out, id, json)`, `site::write_index(out, spec, entries, bare: bool) -> io::Result<PathBuf>` (`bare` drops the `<!doctype>`/`<head>` skeleton for a host that adds its own), `site::id_of(name) -> String`.

Live (`live`, re-exported)
- `LiveSpec { file, poll_ms, stale_ms, counter_labels: Vec<(key, label)> }`; `LiveSpec::new(&[(key, label)])` is `live.js`, 250 ms, 3 000 ms.
- `LiveFrame { t_ms, playing, level: Option<String>, actor: Option<(i32, i32)>, cell: Option<(u16, u16)>, actors: Vec<(x, y, cat)>, cells: Vec<(c, r, value)>, counters: Vec<(key, i64)> }`, `Default`: `actor` and `actors` in level pixels as drawn (no `anchor` added), `level` a manifest `id`, `cells` only those differing from the level's data. `to_js() -> String` is `window.LIVE = {"t":…,"playing":…,"level":…,"actor":[x,y]|null,"cell":[c,r]|null,"actors":[[x,y,"cat"]],"cells":[[c,r,v]],"counters":{"key":n}};`.
- `write_live(dir: &Path, f: &LiveFrame) -> io::Result<()>`: `dir/live.js.tmp`, renamed to `dir/live.js` (`live::LIVE_FILE`); the folder must exist. A failed write is the caller's to drop and retry.

Verdict (`verdict`)
- `verdict::verdict(o: &Outcome, bench: Option<&Bench>) -> Verdict` (not re-exported at the root): `Status::{Clean, Faults, Failed}` (`word()`), `faults`, `info`. Faults: health lost, more than `LONGER_THAN_REF` (25%) over the reference route, planning over `SLOW_PLAN_S` (15 min).
- `Outcome<'a> { has_route, ended, exit_open, coins_left_at_end, lives_lost, hearts_lost, timing: &'a str }`; `Bench { completed, reason, wall_s, route_frames, ref_frames }`.
- `progress(events: &[Event], facts: &Facts) -> Vec<Stall>`: last new ground and closest approach to the open exit; empty when `events` hold an `end`. `Event { k, x, y, kind: &'static str, text }` with kinds `marker`, `hurt`, `death`, `jump`, `land`, `fire`, `end`, `stall`; `Facts { track, cells, inputs_end, exits, exit_open_at }`. This `Facts` is not `amiga_plan::Facts`.

Helpers
- `json::Json::object()`, `str`, `num`, `opt`, `bool`, `raw` (value already JSON), `finish() -> String`; `json::esc`, `json::list`, `json::strs`.
- `guide::parse(text) -> Vec<Waypoint>` (`c`, `r`, `t`, `est`): a planner's waypoint file with optional `t <secs>` and `est` in its comments.

## Where the details are

`src/site.rs` (the page and its embedded viewer), `src/verdict.rs`, `src/json.rs`, `src/guide.rs`; `tests/made_up_level.rs`; `tests/page_chrome.rs` drives the page in headless Chrome (`amiga-browser`, a dev-dependency) at 1280×800 and 800×600 and leaves its screens in `target/tmp/routeview-chrome/`; `tests/live.rs` checks the live file's text and that a reader beside 1 000 writes sees only whole frames; `tests/live_chrome.rs` plays made-up live frames into the page at 1280×800, 1920×1080 and 800×600 (screens in `target/tmp/routeview-live-*/`), and `window.ROUTEVIEW.view()` gives it the view's scale and offset. Routes come from `amiga-plan`.
