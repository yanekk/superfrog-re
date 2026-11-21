# fd-routeview

The map page. `--all` writes the six levels, W1-1 to W6-1, map only and with live mode on, to
`games/forest-dumb/build/map/` (plan `forest-dumb-map-page` T08; the page is `fd-map`'s). `--script`
(plan `forest-dumb-level-1`, DESIGN §2.10) plays an input script on the engine from W1-1's
start (`Start::Level1`) and writes the toolkit's route page (`amiga-routeview`) over level 1's map, with
the character's path, the objects of the object map, the objects in play every other frame and the
events along the way. W1-1 only. It plays the engine, never the original, and gives no verdict on a
route (the page reads "No verdict", `NO_VERDICT`). The page holds disk graphics, so it is written under
`games/forest-dumb/build/` and never committed. `fd-plan` calls `write_page` to draw its routes.

## Start here

0. The six-level page, from `games/forest-dumb/engine/`: `cargo run --release -p fd-routeview -- --all`, then open
   `games/forest-dumb/build/map/index.html` (of the main checkout, from any worktree). It says "App not running"
   until `fd-app` writes `live.js` beside it.
1. From `games/forest-dumb/engine/`: `cargo run --release -p fd-routeview -- --script conform/scripts/stretch-run.txt`,
   then open `games/forest-dumb/build/route-viz/index.html` (it opens from disk, no server, no network).
2. From a test (`crates/fd-plan/tests/route.rs`):
   ```rust
   let (page, dump) = fd_routeview::write_page(&data, &script_path, &dir.join("page")).unwrap();
   assert_eq!(dump.track[n - 1], (char_x as i32, char_y as i32));
   ```
   `data` is `fd_conform::game_data()`.
3. Without writing a page: `parse_script(path, &text)?` then `replay(&data, &script, notes) -> Dump`.

## Command (`src/main.rs`)

```
fd-routeview --all [--out DIR]                  # six levels, map only, live mode on
fd-routeview --level W2-1 [--out DIR]           # one level, map only, live mode on
fd-routeview --script FILE [--out DIR] [--all]  # W1-1's route; with --all the other five map only
```

- `--all`, `--level ID`: default out `fd_map::default_out()`, `games/forest-dumb/build/map` of the main checkout.
  Prints `<page>: 6 levels` (`1 level`). An unknown id exits 2 naming the six. `--level ID --script` replays
  from W1-1's start only, so it is refused for any other level (plan forest-dumb-level-2 adds W2-1's).
- `--script FILE`: required; an input script (`0: -`, `10: R U`, `200: K19`; `U D L R F`, `K<hex>`, `-`; a line holds until the next). Leading `#` lines become the page's notes.
- `--out DIR`: with `--script` alone, default `default_out()`, `games/forest-dumb/build/route-viz` (live mode off, as before).
- Prints `<page>: N frames, ends at W1-1:c,r, M events`. Exit 2: bad arguments or a script path that cannot be read; exit 1: missing disks, a malformed script, or a write failure.

## API by job (`src/lib.rs`)

- `write_page(data: &Arc<GameData>, script_path: &Path, out: &Path) -> Result<(PathBuf, Dump), String>`: writes `index.html` and `data/W1-1.js` under `out`, returns the page's path and the replay.
- `play(data, script_path) -> Result<(Dump, Entry), String>`: the replay and its switcher row, unwritten.
- `write_map(out, worlds: &[u8], route: Option<&dyn LevelDump>) -> Result<PathBuf, String>`: `fd_map::write_levels`
  on the owner's disks (`fd_tools::disk_dir`). `world_of(id)` (error names the six), `level_ids()`.
- `parse_script(path: &Path, text: &str) -> Result<InputScript, String>`: errors name the path and line; a last line past `MAX_FRAMES` is refused.
- `replay(data, script: &InputScript, notes: Vec<String>) -> Dump`: plays to `TAIL` frames past the last line.
- `Dump`: `notes`, `palette`, `cols`, `body` (tile map row by row), `tiles`, `objects: Vec<Object>` (the hidden passages first, cat `secret`, value 0, as `fd-map` writes them), `track: Vec<(i32, i32)>` (`char_x`, `char_y` after each frame), `inputs`, `live` (objects in play every other frame), `events`, `items_needed`, `inputs_end` (the last line's frame). `end_cell() -> Location`, `to_json() -> String`.
- `Object { c, r, value, cat, name }`; `object_name(value: u8) -> (&'static str, String)`: re-exported from `fd-map`, its layer and name from every `re/objects-*.toml`, `other` when none groups it.
- `spec() -> PageSpec`: `fd_map::spec()` with live mode off, for the route page. `Route<'a> { dump, entry }`: a dump as `amiga_routeview::LevelDump`.
- `default_out() -> PathBuf`; `LEVEL` (`LevelId::W1_1`); `TAIL` (100); `MAX_FRAMES` (90 000, 30 minutes); `NO_VERDICT`.

Gotcha: the track runs `TAIL` frames past `inputs_end`, so a route's end is `track[inputs_end - 1]`,
not `track.last()`. `fd_plan::script_text` ends a route with a let-go line at its length for this reason.

## Where the details are

- `src/lib.rs` (replay, JSON, page), `src/main.rs`; `tests/page.rs` (the page's track against a replay; the page is git-ignored);
  `tests/levels_chrome.rs` (`--all`'s data against the disks' object files, bad arguments in smoke; `--level`,
  `--script --all` and the six levels in headless Chrome at 1280×800, 1920×1080, 800×600, about 25 s, full).
- `toolkit/crates/amiga-routeview` (page writer: `site`, `json`, `verdict`); `toolkit/crates/amiga-conform/src/script.rs` (input-script format).
- `games/forest-dumb/CLAUDE.md` "Seeing a run on the map"; `crates/fd-map` (object tables, level data); `plans/forest-dumb-level-1/DESIGN.md` §2.10.
