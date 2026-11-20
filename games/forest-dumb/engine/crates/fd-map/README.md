# fd-map

Each world's level as the map page draws it, read from the level files and the main program with no
engine run (plan `forest-dumb-map-page`, DESIGN §2.1, §2.2, §3.2): the tile map, the tiles it uses, the
palette, every object with its page layer, the hidden passages, the exit cells and the items the exit needs. Separate from
`fd-routeview` so `fd-app` can link it: its normal dependencies are `fd-assets` and `amiga-routeview`
only. `src/level.rs` is pure (`fd-conform/tests/purity.rs` scans it); `src/lib.rs` is the shell side.

## Start here

1. One world, from a test or tool (`tests/levels.rs`):
   ```rust
   let level = fd_assets::read_level(3, &files)?;           // files: disk 2's, unpacked
   let m = fd_map::map_level(3, &level, &program)?;          // program: disk 1's FOREST_DUMB_FOREVER
   assert_eq!(m.items_needed, 676);
   ```
2. On the page: `MapLevel` is an `amiga_routeview::LevelDump` (a map-only entry, empty track), so
   `amiga_routeview::site::write_level(out, &m)` writes `data/W3-1.js`.
3. An object's layer and name alone: `fd_map::object_name(0x34)` is `("flame", "flame $34")`.

## API (`src/lib.rs`, `src/level.rs`)

- `WORLDS` (`[1, 2, 3, 4, 5, 6]`), `level_id(world) -> String` (`"W2-1"`).
- `map_level(world, &Level, program: &[u8]) -> Result<MapLevel, String>`: errors name the file
  (`_MapaBobowN: no exit`, `FOREST_DUMB_FOREVER: hunk 1 +$398: ... is not colour 5's copper move`).
- `MapLevel { world, cols, rows, palette, body, tiles, objects, secrets, exit, items_needed }`; `to_json()`.
  `secrets` is `fd_assets::secrets(level)` as `MapObject`s: cat `secret`, value 0, named by kind
  (`secret_name(SecretKind)`, `"hidden passage"`). Only hidden passages: T07 confirmed no other kind on
  the original (docs/17), so there is no invisible floor to list. `to_json` writes them into `objects`
  before the real objects, because the page keys a cell's hover by the last object there and a coin
  in a passage must stay the coin; a test counting the file's objects skips value 0.
  `cols` is the used columns (the collision map's: 240, W6-1 320); `body` is row by row.
- `MapObject { c, r, value, cat, name }`.
- `object_name(value) -> (&'static str, String)`: the layer (`coin`, `enemy`, `pickup`, `hazard`,
  `spring`, `block`, `flame`, `exit`) and name from every `re/objects-*.toml`; `other` and
  `object $vv` for a value no table groups, never a guess. `groups()`, `groups_of(tables)`, `TABLES`.
- `items_needed(file, objects: &[u8]) -> Result<u16, String>`: the coins (`$20`-`$27`) over all
  38 400 bytes less 50, as every `load_list_world_N` computes it (T00, docs/10, "Items"). It is a pure
  function of the object map: no run of the program is needed.
- `palette(program) -> Result<Vec<[u8; 3]>, String>`: hunk 1's level colours at `$384`, registers checked.
- `spec() -> PageSpec`: the game's page, a Secrets layer (cat `secret`, off at load, DESIGN §2.3), live mode on (`live.js`, "Items left"); `offline(html)` drops the toolkit's
  web-font line and shows a clean route as "No verdict".
- `write_site(out, files, program, route) -> Result<PathBuf, String>`: the six levels' page (`index.html`,
  `data/W?-1.js`); `write_levels(out, files, program, worlds, route)` for some. `route` (a `LevelDump`) takes its
  level's place; every level is read before anything is written. `MapLevel`'s switcher row has no verdict ("Map only").
- `default_out() -> PathBuf`: `games/forest-dumb/build/map` of the main checkout (from any worktree,
  read from git's files, git not run); `out_in(dir) -> Option<PathBuf>` the same for a given folder,
  `None` outside a checkout (the app then uses `<home>/map`, DESIGN §2.9).

Gotcha: the object tables are gathered by `build.rs` at build time, every `re/objects-*.toml` sorted by
name; a new table is picked up by the next build. A value two tables group differently keeps the first,
and `fd-conform/tests/object_tables.rs` fails on it.

## Tests

- `tests/levels.rs` (smoke, about 2 s, needs both disks): made-up levels (layers, exit, unknown value,
  coins past the used columns, W6-1's 320 columns, a hidden passage, errors), then each real world's
  object counts per value against its `_MapaBobowN` bytes, its 2×3 exit, T00's items needed, and its
  secrets against `fd_assets::secrets` (W1-1 133, W3-1 292, W4-1 63, the rest 0).
- `fd-routeview/tests/levels_chrome.rs`: the page in Chrome; Secrets off at load, drawn at each of
  W1-1's cells when on and named on hover, W2-1's count 0.
- `fd-conform/tests/map_oracle.rs` (full, about 76 s): each world's items needed and `$ff` cells against
  `items_left` and the RAM object map (`h0+$cb38`) when `main_loop` first runs, by the
  `world-N-start` cases' way in.
