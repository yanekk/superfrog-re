# T05 — level-readers

**Phase:** 1 · **Depends on:** T03, T04, T30 · **Weight:** medium

## Goal
Read level 1's files into Rust, and prove the formats against the original's memory and screen. This closes the gap that the renders were never compared with the game, and once were drawn transposed.
## Design sections this implements
§3.1, §3.2; `docs/05-file-formats.md`, `docs/06-graphics-and-rendering.md`.
## Reuses
`tools/render_levels.py`'s reading, `fd-tools` extract and T30's unpack, `sf-assets` as a shape, not code.
## Files
`games/forest-dumb/engine/crates/fd-assets/`, `fd-conform/tests/{purity.rs,level_files.rs}`, `games/forest-dumb/docs/06-graphics-and-rendering.md`, `smoke.toml`.
## Interface
```
pub struct Level { pub tiles: TileMap, pub gens: Vec<[u8;160]>, pub collision: CollisionMap, pub shapes: Vec<[u16;16]>, pub objects: ObjectMap }
impl Level { pub fn tile(&self, col: u16, row: u16) -> u16; pub fn shape(&self, col: u16, row: u16) -> u8; pub fn object(&self, col: u16, row: u16) -> u8 }
pub fn read_level(world: u8, files: &dyn Fn(&str) -> Result<Vec<u8>, AssetError>) -> Result<Level, AssetError>
pub struct Sprites { .. }  pub fn read_sprites(files: ..) -> Result<Sprites, AssetError>
```
Files come through a closure so the crate stays pure.
## Tests
- [ ] Each map equals the bytes at its hunk target in oracle memory at the sync point.
- [ ] A cell's tile matches the oracle's picture at that cell on the first play frame, at several cells.
- [ ] A short file and a wrong-size file give `AssetError`, not a panic.
- [ ] Column-order test: a seam check fails if read as rows.
- [ ] Purity test includes `fd-assets`.
## Done when
- [ ] All level 1 files and sprite sheets read and match the oracle, or a difference is named.
- [ ] `docs/06` states how far the render is now proven.
