# fd-assets

Pure readers for Forest Dumb Forever's level files, sprite sheets and the main program's hunks, from
their unpacked bytes (`docs/05-file-formats.md`, `docs/06-graphics-and-rendering.md`). Files arrive
through a closure the caller supplies, so the crate touches no disk and has no dependencies. None of the
files has a header, so a file's size is its whole check: a wrong size, or a value the format cannot hold,
is an `AssetError` naming the file, never a panic. It does not find, unzip or unpack files (`fd-tools`),
and it does not interpret objects or play (`fd-core`). Most callers get a `Level` already read, as
`data.level` of `fd_core::GameData`.

## Start here

1. Read world 1 from the disks (`fd-conform/tests/level_files.rs`):
   ```rust
   fn files(name: &str) -> Result<Vec<u8>, AssetError> {
       let dir = fd_tools::disk_dir().map_err(|e| AssetError::missing(name, e))?;
       fd_tools::game_file(&dir, &format!("disk2/{name}")).map_err(|e| AssetError::missing(name, e))
   }
   let level = read_level(1, &files).unwrap_or_else(|e| panic!("{e}"));
   let sprites = read_sprites(1, &files).unwrap_or_else(|e| panic!("{e}"));
   ```
2. Read a cell: `level.tile(col, row)`, `level.shape(col, row)`, `level.object(col, row)`. Maps are stored a column at a time, index `col * 96 + row`.
3. Test without disks: hand `read_level` a closure of made-up files of the right sizes (`src/tests.rs`, `files(tiles, shapes)`).

## API by job (`src/lib.rs`)

Common:
- `Files<'a> = dyn Fn(&str) -> Result<Vec<u8>, AssetError> + 'a`: hands over one file by its disk name, unpacked.
- `AssetError { file: String, why: Why }`; `AssetError::new(file, why)`, `AssetError::missing(file, reason)` (for the closure). `Why::Missing(String)`, `Size { expected, got }`, `NotWhole { unit, got }`, `Malformed { at, what }`. `Display` is `<file>: <why>`.
- `PLANES` (5): interleaved bitmaps, one line of each plane after the other.

Levels (`src/level.rs`):
- `read_level(world: u8, files: &Files) -> Result<Level, AssetError>`: world 1 to 6.
- `level_file_names(world: u8) -> [String; 5]`: tile map, tile pictures, collision map, shapes, object map.
- `Level { tiles: TileMap, gens: Vec<[u8; TILE_BYTES]>, collision: CollisionMap, shapes: Vec<[u16; 16]>, objects: ObjectMap }`. Each map has `columns: u16` and `cells`, column by column.
- `Level::tile(col, row) -> u16`, `shape(col, row) -> u8`, `object(col, row) -> u8`: panic past the map, as a slice index does. `tile_pixel(tile: u16, x: u8, y: u8) -> u8`: a colour number 0 to 31.
- Gotcha: the tile map is wider than the collision map (280 and 240 columns outside world 6; the extra columns are filler rock), and the object map always has `OBJECT_COLUMNS` (400). Bound a loop by the map you read.
- `LEVEL_ROWS` (96), `TILE_BYTES` (160), `SHAPE_BYTES` (32, 16 big-endian words, set bit solid, top line first), `OBJECT_COLUMNS` (400).
- The object map holds the file's bytes; the game writes its own copy at run time (fd-core's `state().object_map`).

Sprites (`src/sprites.rs`):
- `read_sprites(world: u8, files: &Files) -> Result<Sprites, AssetError>`; `sprite_file_names(world: u8) -> [String; 7]`.
- `Sprites { creatures, elements, flame, vanishing, knocker }`, each a `Sheet`.
- `Sheet { file, picture, mask: Option<Vec<u8>>, line_bytes, frame_width, frame_height }`; `lines()`, `pixel(x, y) -> u8`. World-specific sheets have no mask.

Program (`src/program.rs`):
- `loaded_hunk(exe: &[u8], file: &str, hunk: usize, bases: &[u32]) -> Result<Vec<u8>, AssetError>`: one hunk as the loader leaves it, zero-padded, `RELOC32` longs relocated by `bases`. Pass the oracle's bases (`fd_conform::HUNK_BASES`) so pointers match the original's.

## Where the details are

- `src/level.rs`, `src/sprites.rs`, `src/program.rs`, `src/error.rs`; `src/tests.rs` (made-up files).
- `games/forest-dumb/docs/05-file-formats.md` (byte layout), `06-graphics-and-rendering.md` (tiles, sprite sets, palette), `LEVEL-INDEX.md` (level ids and files), `07-executable-map.md` (hunks).
- `fd-conform/tests/level_files.rs`: the real files checked against the original's memory.
