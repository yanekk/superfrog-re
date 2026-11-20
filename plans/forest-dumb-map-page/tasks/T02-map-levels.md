# T02 — map-levels

**Phase:** 1 · **Depends on:** T00, T01 · **Weight:** medium

## Goal

A new crate `fd-map` that turns each world's level files into what the page draws, with no engine run: the
tile map, the tiles it uses, the palette, every object with its layer, the exit cells and the items needed.
It is separate from `fd-routeview` so `fd-app` can link it (DESIGN §7).

## Design sections this implements

DESIGN §2.1, §2.2, §2.9, §3.1, §3.2.

## Files

- `games/forest-dumb/engine/crates/fd-map/` (new): `Cargo.toml`, `README.md`, `src/lib.rs`, `src/level.rs`
  (pure), `tests/levels.rs`, `tests/oracle.rs` (in `fd-conform` instead if it needs the oracle crate as a
  dev-dependency there; keep `fd-map`'s normal dependencies to `fd-assets` and `amiga-routeview`).
- `games/forest-dumb/engine/Cargo.toml` (member); `games/forest-dumb/smoke.toml`;
  `games/forest-dumb/engine/crates/fd-conform/tests/purity.rs` (the file-level check);
  `games/forest-dumb/engine/crates/fd-routeview/src/lib.rs`, `Cargo.toml` (`object_name` from `fd-map`).

## Interface

```rust
pub const WORLDS: [u8; 6] = [1, 2, 3, 4, 5, 6];
pub fn level_id(world: u8) -> String;                 // "W2-1"; `fd_core::recording::LevelId::name` once
                                                      // plan forest-dumb-title's six-world LevelId is merged
pub struct MapObject { pub c: u16, pub r: u16, pub value: u8, pub cat: &'static str, pub name: String }
pub struct MapLevel {
    pub world: u8, pub cols: u16, pub rows: u16,
    pub palette: Vec<[u8; 3]>, pub body: Vec<u16>, pub tiles: Vec<(u16, [u8; 256])>,
    pub objects: Vec<MapObject>, pub exit: Vec<(u16, u16)>, pub items_needed: u16,
}
// Pure: the level's bytes and the program's, nothing read here.
pub fn map_level(world: u8, level: &fd_assets::Level, program: &[u8]) -> Result<MapLevel, String>;
pub fn object_name(value: u8) -> (&'static str, String); // both tables; `other` for neither
pub fn items_needed(world: u8, program: &[u8] /* or map, per T00 */) -> Result<u16, String>;
impl amiga_routeview::LevelDump for MapLevel { .. }  // map-only entry (T04), empty track
pub fn default_out() -> std::path::PathBuf;          // games/forest-dumb/build/map of the main checkout
```

`object_name` moves here from `fd-routeview` (which then calls it), so one table reader serves both; it
reads every `re/objects-*.toml` (DESIGN §2.1). If T00
found the count is only produced by a run, `items_needed` becomes a shell function in `src/lib.rs` running
the translated program headless; say so in the README.

## Tests

- [ ] Made-up level bytes: objects, layers, exit cells, `other` for an unknown value, W6-1's 320 columns.
- [ ] The pure-side guard: `fd-conform/tests/purity.rs` extended to scan `fd-map/src/level.rs` (DESIGN §3.1).
- [ ] Each real world (disks): object counts per value equal the `_MapaBobowN` file's over the used columns.
- [ ] Oracle (full): each world's items needed and its `$ff` cells equal `items_left` and the RAM object map
  (`h0+$cb38`) after `level_init`, through the `world-N-start` way in.

## Done when

- [ ] `fd-map` builds with only `fd-assets` and `amiga-routeview` as normal dependencies (plus
  `fd-translated` and `pcode-machine` only if T00 found the count needs a run; never `fd-conform` or the
  oracle, so `fd-app/tests/deps.rs` holds).
- [ ] The file test passes in smoke; the oracle test passes and is labelled full.
- [ ] `fd-routeview` uses `fd_map::object_name`; its tests stay green.
