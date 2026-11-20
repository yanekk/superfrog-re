# fd-tools

The shell side of the owner's disks (plan `forest-dumb-level-1`, DESIGN §2.9, §3.1): finds the two zips,
reads the ADF inside each, extracts both disks into `build/` and writes the committed inventory, and
reads one game file unpacked, in memory. It also unpacks `ESL!` files (PowerPacker's format under another
magic) in Rust, and stamps the engine build a bug recording carries. The zips are only ever read, and
nothing is written until both disks have been read whole. It holds no game logic and parses no game
format beyond the packer; level and sprite files are `fd-assets`.

## Start here

1. Find the disks in a test or tool: `fd_tools::disk_dir()` returns the folder or an error naming every
   place tried. A test that needs the disks fails with that error, never skips (`tests/locate.rs`).
2. One file, unpacked, nothing written (`fd-conform/tests/level_files.rs`):
   `fd_tools::game_file(&fd_tools::disk_dir()?, "disk2/_Mapa1-1")`. A bare name works when it is on one disk only.
3. Extract both disks, from `games/forest-dumb/engine/`: `cargo run -p fd-tools -- extract`. It writes
   `games/forest-dumb/build/disk1.adf`, `disk2.adf`, `build/files/disk<N>/<name>`, prints
   `disk<N> <name> <size> <sha1>` a file, and rewrites `games/forest-dumb/docs/04-file-inventory.txt`.

## Command (`src/main.rs`)

`fd-tools extract [--dir DIR] [--out DIR]`

- `--dir DIR`: the folder holding both zips; default `disk_dir()`.
- `--out DIR`: default `games/forest-dumb/build`, anchored on the crate, so the worktree it was built in is written to. It must be inside a folder named `build`, with no `..`, or it is refused.
- Anything else, or no `extract`, prints the usage; failures exit 1 with `fd-tools: <why>`.

## API by job (`src/lib.rs`)

Finding the disks (`src/locate.rs`):
- `disk_dir() -> Result<PathBuf, String>`: `FOREST_DUMB_DISKS`, else `games/forest-dumb/` in the main checkout, else its root.
- `locate(env: Option<&str>, checkout: &Path) -> Result<PathBuf, String>`: the same with its inputs passed in, for tests.
- `main_checkout() -> Result<PathBuf, String>`: the git common dir's parent, from any worktree (runs `git`).
- `zip_paths(dir: &Path) -> Result<[PathBuf; 2], String>`: each disk's zip by the `disk N of 2` in its name; two matches for one disk is an error.

Reading:
- `read_adf(path: &Path) -> Result<Vec<u8>, String>`: the one ADF in a zip, checked to be exactly a disk's size; errors name the zip and its SHA-1.
- `game_file(disks: &Path, name: &str) -> Result<Vec<u8>, String>`: `disk1/<name>`, `disk2/<name>` or a bare name, read and unpacked in memory.
- `unpack(packed: &[u8]) -> Result<Vec<u8>, String>`: an `ESL!` file unpacked; an unpacked file comes back unchanged; a StoneCracker executable is refused. A damaged file is an `Err`, never a panic.

Extracting:
- `extract(dir: &Path, out: &Path) -> Result<Vec<InventoryFile>, String>`: both disks into `out`, sorted by disk then name.
- `InventoryFile { disk: u8, name: String, size: u32, sha1: String }`.
- `inventory_text(dir: &Path, files: &[InventoryFile]) -> Result<String, String>`: the inventory file's text, tab-separated.
- `entry_path(name: &str) -> Result<PathBuf, String>`: a disk name as a safe relative path; refuses `..`, `.`, empty parts.

Build stamp:
- `ENGINE_BUILD: &str`: the version and a hash of the engine's sources (fd-core, fd-assets, the toolkit crates they use, the lock file), set by `build.rs`. `fd-app` stamps recordings with it and `fd-conform` checks it on replay.

## Tests

All need the disks except `locate.rs` and `bad_disks.rs`, which build fake zips. `real_disks.rs`
(the inventory), `unpack.rs` (every `ESL!` file against the SHA-1s in the docs), `boot_chain.rs`,
`exe_map_claims.rs` (checks `docs/07-executable-map.md`). All smoke.

## Where the details are

- `src/locate.rs`, `src/unpack.rs`, `src/lib.rs`, `build.rs`.
- `games/forest-dumb/docs/01-disk-location.md` (lookup, file names), `02-disk-format.md`, `04-file-inventory.txt`, `08-packers.md` (the format; `tools/unpack.py` is the reference this is checked against).
- `toolkit/crates/amiga-disk` (OFS images, zip, SHA-1).
