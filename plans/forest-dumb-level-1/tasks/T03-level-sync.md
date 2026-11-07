# T03 — level-sync

**Phase:** 1 · **Depends on:** T00 · **Weight:** medium

## Goal
Reach the start of level 1 on the original in seconds rather than the 35 s boot, by caching a snapshot, so level cases and routine tests are cheap.
## Design sections this implements
§2.3, §3.5, §3.6.
## Reuses
`sf-conform/src/sync.rs:1460-1523` (`at_snapshot`, `cache_key`, encode and decode), moved to the toolkit with magic and root as parameters; `Amiga::snapshot`, `restore`.
## Files
`toolkit/crates/amiga-conform/src/sync_cache.rs` or `amiga-oracle`, whichever does not add a cycle; `games/superfrog/engine/crates/sf-conform/src/sync.rs`; `games/forest-dumb/engine/crates/fd-conform/src/{lib.rs,sync.rs}`; `fd-conform/tests/level_sync.rs`; both `smoke.toml`.
## Interface
```
pub fn at_snapshot(root: &Path, magic: &[u8], key: [u8; 20], build: impl FnOnce() -> Result<Amiga>) -> Result<Amiga>
pub fn cache_key(format: u32, kickstart: &[u8], disks: &[&[u8]], drives: &str, what: &str) -> [u8; 20]
pub fn reach_level_1() -> Result<Amiga>                          // fd-conform
```
## Tests
- [ ] Superfrog's cache-key pins at `sync.rs:1819-1825` still pass unchanged.
- [ ] Cached and uncached reach give identical chip and slow RAM.
- [ ] A changed script, disk or build changes the key.
- [ ] A corrupt cache file rebuilds rather than panics.
- [ ] Missing disks and Kickstart name the path.
## Done when
- [ ] `reach_level_1` from cache takes under 5 s, measured in the commit.
- [ ] Superfrog's `sf-conform` suite is green.
- [ ] Toolkit code names no game.
