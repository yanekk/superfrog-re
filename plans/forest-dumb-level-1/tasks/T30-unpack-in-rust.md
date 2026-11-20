# T30 — unpack-in-rust

**Phase:** 1 · **Depends on:** — · **Weight:** medium

## Goal
Unpack the game's `ESL!` files in Rust, because the app holds no Python or 68k runner and 46 files on disk 2, level 1's among them, are packed. Only `tools/unpack.py` does this today, under Musashi. Numbered T30 to spare a renumbering; it runs before T05.
## Design sections this implements
§3.2, §3.4, §2.12; `docs/08-packers.md`.
## Reuses
`amiga-disk`'s zip and ADF reading through `fd-tools`; the second decoder in `tools/test_unpack.py` and the format table in `docs/08-packers.md`. The crack intro's StoneCracker stays out: the game does not need it.
## Files
`games/forest-dumb/engine/crates/fd-tools/src/{lib.rs,unpack.rs}`, `fd-tools/tests/unpack.rs`, `games/forest-dumb/docs/08-packers.md`, `games/forest-dumb/smoke.toml`.
## Interface
```
pub fn unpack(packed: &[u8]) -> Result<Vec<u8>, String>      // unpacked files pass through unchanged
pub fn game_file(disks: &Path, name: &str) -> Result<Vec<u8>, String>   // in memory, unpacked, nothing written
```
`game_file` is the closure T05's `read_level` takes, so the app needs no `build/` folder.
## Tests
- [ ] Every `ESL!` file's output has the SHA-1 in `docs/08-packers.md`.
- [ ] The Rust output equals `unpack.py`'s on level 1's files.
- [ ] A truncated file, a wrong size field and a bad leftover-bit count give `Err`, not a panic.
- [ ] A file not on the disks names the file and path.
## Done when
- [ ] All 46 files unpack and match, in a smoke-labelled test.
- [ ] Time to read level 1's files is measured in the commit.
- [ ] `docs/08` says the Rust decoder exists and is what the engine uses.
