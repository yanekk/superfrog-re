# T05 — map-cache

**Phase:** 1 · **Depends on:** T04 · **Weight:** light

## Goal

Save each level's map so everyday planning pays the build once, without ever letting a stale or
partial map be used, and with a switch that makes the benchmark build fresh.

## Design sections this implements

DESIGN §2.8, §2.9 (corrupt cache, two writers), D6.

## Files

- `engine/crates/sf-plan/src/cache.rs`, `engine/crates/sf-plan/tests/cache.rs`
- `engine/crates/sf-plan/Cargo.toml` (`build = "../sf-app/build.rs"`, as sf-conform has, for `SF_ENGINE_BUILD`)

## Interface

```rust
pub fn cache_dir() -> PathBuf;                                // ~/.cache/superfrog-re/maps, or $SF_MAP_CACHE
pub fn key(level: LevelId, start: StartKey, wings: bool) -> String; // includes env!("SF_ENGINE_BUILD")
pub fn load_or_build(key: &str, no_cache: bool, build: impl FnOnce() -> MoveMap) -> MoveMap;
```

`MoveMap` derives serde; files are bincode with a magic and version header; writes go to `*.tmp` then
`rename`.

## Tests

- [ ] Round trip equal; a truncated file is rebuilt; a file with another engine build key is ignored.
- [ ] `no_cache` builds even when a file exists and does not overwrite it.
- [ ] Two threads writing the same key leave one whole readable file.

## Done when

- [ ] Tests green; tests use a temp `SF_MAP_CACHE`, never the real cache.
