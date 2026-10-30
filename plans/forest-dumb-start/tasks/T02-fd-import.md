# T02 — fd-import

**Phase:** 1 · **Depends on:** T01 · **Weight:** medium

## Goal

Every file of both disks comes out of the owner's zips into `build/`, with a committed inventory,
and the game's test command becomes real. This is playbook step 1's import half: the disks are
AmigaDOS OFS, so import is zip, ADF, OFS reader, inventory, not the trackloader Superfrog needed
(DESIGN §2.3).

## Design sections this implements

DESIGN §2.3, §2.10 (the import's unhappy paths), §5.2 (the write-target seatbelt).

## Files

- `games/forest-dumb/engine/` — a new cargo workspace shaped like Superfrog's: `Cargo.toml`
  (workspace, resolver 3, the dev profile overrides for the amiga-* crates copied from
  Superfrog's, because a build reads them from the workspace root), `rust-toolchain.toml` pinned
  to 1.99.0, member `crates/fd-tools/`.
- `games/forest-dumb/engine/crates/fd-tools/` — `src/lib.rs`, `src/locate.rs`, `src/main.rs`,
  `tests/`.
- `games/forest-dumb/test.sh` — real now: `cargo test -q --workspace` in the engine. The Python
  tools line arrives with the first Python tool in T03; adding a discover on a `tools/` folder
  that does not exist yet would fail.
- `games/forest-dumb/docs/02-disk-format.md` — the two disks' layout: OFS structure, the volume
  name, block sizes, the files each disk holds.
- `games/forest-dumb/docs/04-file-inventory.txt` — the inventory: one line a file (disk, name,
  size, SHA-1), with a header naming the two zips and their SHA-1s.
- `games/forest-dumb/CLAUDE.md` — index lines for the new docs and the inventory.

## Interface

```
/// Where the two zips live: FOREST_DUMB_DISKS, else games/forest-dumb/ in the main checkout,
/// else its root. Pure: the lookup inputs arrive as arguments.
pub fn locate(env: Option<&str>, checkout: &Path) -> Result<PathBuf, String>;

/// One extracted file of the inventory.
pub struct InventoryFile { pub disk: u8, pub name: String, pub size: u32, pub sha1: String }

/// Extract both disks into `out` (created if missing): the two ADFs and every file, one file
/// per entry in `out/files/disk<N>/<name>`. Never writes outside `out`.
pub fn extract(dir: &Path, out: &Path) -> Result<Vec<InventoryFile>, String>;
```

The binary: `cargo run -p fd-tools -- extract [--dir DIR] [--out build/forest-dumb]`, printing
one line a file and writing the inventory text to `docs/04-file-inventory.txt` under the repo.
The inventory is committed; `build/` is not.

## Tests

- [ ] `locate`: the env var wins, then the game folder, then the root; nothing found names every
      path it looked at (the same contract as Superfrog's).
- [ ] `extract` on the real disks (fails loudly naming where it looked when they are absent,
      matching Superfrog's real-disk tests): the volume name is `FOREVER` on both; the main
      program `FOREST_DUMB_FOREVER` is 521 640 bytes; there are 48 data files (`_Mapa*`,
      `_Gen*`, `_MapaBobow*`); `!EAGLE_SOFT!` sits on disk 2.
- [ ] A corrupt zip and a truncated ADF are reported by name, never silently extracted.
- [ ] Extraction is repeatable: two runs produce the same SHA-1s.

## Done when

- [ ] `cargo run -p fd-tools -- extract` lists every file of both disks, and the worker's
      cross-check against amitools' `xdftool` (names and sizes of both disks, bytes of a few
      files) agrees exactly, recorded in FINDINGS.
- [ ] `sh games/forest-dumb/test.sh` is green with the disks present.
- [ ] The inventory and the docs are committed; `git status --porcelain` shows nothing derived
      (extracted files and ADFs under `build/` are ignored).

## Outside actions

- Fetch a crates.io dependency — `worker` (DESIGN §5.3), only if fd-tools needs one; the toolkit
  zip and OFS readers are expected to cover it.
