# fd-routeview

The map page (plan `forest-dumb-level-1`, DESIGN §2.10): plays an input script on the engine from W1-1's
start (`Start::Level1`) and writes the toolkit's route page (`amiga-routeview`) over level 1's map, with
the character's path, the objects of the object map, the objects in play every other frame and the
events along the way. W1-1 only. It plays the engine, never the original, and gives no verdict on a
route (the page reads "No verdict", `NO_VERDICT`). The page holds disk graphics, so it is written under
`games/forest-dumb/build/` and never committed. `fd-plan` calls `write_page` to draw its routes.

## Start here

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

`fd-routeview --script FILE [--out DIR]`

- `--script FILE`: required; an input script (`0: -`, `10: R U`, `200: K19`; `U D L R F`, `K<hex>`, `-`; a line holds until the next). Leading `#` lines become the page's notes.
- `--out DIR`: default `default_out()`, `games/forest-dumb/build/route-viz`.
- Prints `<page>: N frames, ends at W1-1:c,r, M events`. Exit 2: bad arguments or a script path that cannot be read; exit 1: missing disks, a malformed script, or a write failure.

## API by job (`src/lib.rs`)

- `write_page(data: &Arc<GameData>, script_path: &Path, out: &Path) -> Result<(PathBuf, Dump), String>`: writes `index.html` and `data/W1-1.js` under `out`, returns the page's path and the replay.
- `parse_script(path: &Path, text: &str) -> Result<InputScript, String>`: errors name the path and line; a last line past `MAX_FRAMES` is refused.
- `replay(data, script: &InputScript, notes: Vec<String>) -> Dump`: plays to `TAIL` frames past the last line.
- `Dump`: `notes`, `palette`, `cols`, `body` (tile map row by row), `tiles`, `objects: Vec<Object>`, `track: Vec<(i32, i32)>` (`char_x`, `char_y` after each frame), `inputs`, `live` (objects in play every other frame), `events`, `items_needed`, `inputs_end` (the last line's frame). `end_cell() -> Location`, `to_json() -> String`.
- `Object { c, r, value, cat, name }`; `object_name(value: u8) -> (&'static str, String)`: its layer and name from `re/objects-W1-1.toml`, `other` when the file lacks it.
- `spec() -> PageSpec`: what the page knows of the game. `Route<'a> { dump, entry }`: a dump as `amiga_routeview::LevelDump`.
- `default_out() -> PathBuf`; `LEVEL` (`LevelId::W1_1`); `TAIL` (100); `MAX_FRAMES` (90 000, 30 minutes); `NO_VERDICT`.

Gotcha: the track runs `TAIL` frames past `inputs_end`, so a route's end is `track[inputs_end - 1]`,
not `track.last()`. `fd_plan::script_text` ends a route with a let-go line at its length for this reason.

## Where the details are

- `src/lib.rs` (replay, JSON, page), `src/main.rs`; `tests/page.rs` (the page's track against a replay; the page is git-ignored).
- `toolkit/crates/amiga-routeview` (page writer: `site`, `json`, `verdict`); `toolkit/crates/amiga-conform/src/script.rs` (input-script format).
- `games/forest-dumb/CLAUDE.md` "Seeing a run on the map"; `games/forest-dumb/re/objects-W1-1.toml`; `plans/forest-dumb-level-1/DESIGN.md` §2.10.
