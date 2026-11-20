# T08 — asset-decode

**Phase:** 1 · **Depends on:** T04, T07 · **Weight:** medium

## Goal

Decode every kind of game content in Rust, from chunks to typed data in `GameData`: levels (T7MP and
whatever T07 found beside it), tilesets, palettes, sprite and bob banks, fonts, full-screen pictures,
the IFF ANIM sequences, and the numeric tables the mechanics need, located by executable and offset
(DESIGN §2.3). Sound data is T11's. Verified against the Python renders where they exist and against
the original's memory where they do not.

## Design sections this implements

DESIGN §2.3, §3.2 (`sf-assets`), docs/04, docs/06.

## Files

- `engine/crates/sf-assets/src/{lib.rs,gamedata.rs,t7mp.rs,tiles.rs,palette.rs,sprites.rs,font.rs,picture.rs,anim.rs,tables.rs,exe.rs}`
- `engine/crates/sf-assets/tests/{levels.rs,graphics_vs_python.rs,graphics_vs_oracle.rs}`
- `docs/10-asset-formats.md` — every format decoded here that docs/04 and docs/06 don't cover.

## Interface

```rust
pub struct GameData { pub levels: BTreeMap<LevelId, LevelData>, pub worlds: Vec<WorldGfx>, pub sprites: Vec<SpriteBank>,
                      pub fonts: Vec<Font>, pub pictures: BTreeMap<PictureId, Picture>, pub anims: BTreeMap<AnimId, Anim>,
                      pub tables: Tables, pub exes: Vec<ExeImage> }
pub struct LevelData { pub id: LevelId, pub t7mp: T7mp, pub extra: BTreeMap<String, Vec<u8>> /* per T07 */ }
pub struct T7mp { pub width: u16, pub height: u16, pub body: Vec<u16>, pub tags: Vec<(Tag, Vec<u8>)> /* all, in order, lossless */ }
pub struct Tables { /* one typed field per table the mechanics need, each with its exe+offset in a doc comment */ }
pub fn decode(chunks: &[(ChunkId, Vec<u8>)]) -> Result<GameData, AssetError>
pub fn t7mp_encode(t: &T7mp) -> Vec<u8>   // lossless: encode(decode(x)) == x, for T28
```

`Tables` starts with what T07 named and grows in each mechanic task; this task fixes the pattern
(typed field, decoded from `ExeImage` at a named offset, offset documented).

## Tests

- [ ] all 29 levels decode; `t7mp_encode(decode(x)) == x` byte for byte for each.
- [ ] each world's tileset rendered with its palette equals `tools/render_levels.py`'s PNG pixel for pixel.
- [ ] sprites, fonts and pictures equal the original's decoded copies in memory at a sync point where
      the game has them loaded (oracle).
- [ ] each ANIM decodes to the frame count and size its header claims; frame 0 equals the oracle's screen
      at the moment the intro shows it.
- [ ] a truncated or corrupt chunk returns `AssetError` naming the chunk, no panic.

## Done when

- [ ] `decode` builds a complete `GameData` from the real disks in the test block.
- [ ] Every decoded format is documented in docs/04, 06 or 10.
- [ ] The no-disk-data check and purity test still pass.
