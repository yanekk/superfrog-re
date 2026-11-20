# T28 — level-override

**Phase:** 3 · **Depends on:** T14, T24 · **Weight:** light

## Goal

Give the future map editor its target: export any disk level to a `.sflevel` file, and play a
`.sflevel` in place of the level it names (DESIGN §2.10). An unchanged export must play
frame-identically to the disk level, which proves the editor's round trip before the editor exists.

## Design sections this implements

DESIGN §2.10, §2.11.

## Files

- `engine/crates/sf-assets/src/sflevel.rs` — encode/decode (pure).
- `engine/crates/sf-app/src/levelfile.rs`; `menu.rs` — File → Export Level…, Play Level File…
- `docs/14-sflevel-format.md` — the format, for the editor.
- `engine/crates/sf-assets/tests/sflevel.rs`, `engine/crates/sf-e2e/tests/levelfile.rs`

## Interface

```rust
pub fn sflevel_encode(level: &LevelData) -> String;   pub fn sflevel_decode(s: &str) -> Result<LevelData, SflevelError>
pub enum SflevelError { NotJson, WrongFormat, Version(u32), UnknownLevel(String), MissingSection(String), BadT7mp(String) }
impl Game { pub fn new_with_override(data: &GameData, ov: LevelData) -> Game }
```

## Tests

- [ ] every level: `sflevel_decode(sflevel_encode(l)) == l`.
- [ ] an unchanged export played from `Start::Level` equals the disk level for 1 500 frames of the frog script.
- [ ] one changed cell (wall removed) makes the frog pass where it didn't, and nothing else differs before that frame.
- [ ] each `SflevelError` from a crafted file.

## End to end (the worker drives this)

- suite: `sf-e2e` · sizes: 2×
- [ ] export W1-1 via the menu, play it via the menu → frames equal the disk level's.
- [ ] a bad file → the refusal with its reason; choosing the disk level then plays it.

## Done when

- [ ] Tests and e2e green; docs/14 written.
- [ ] Exports land only where the player chose; none in the repo (no-disk-data check green).
- [ ] The format is versioned and its version-1 contents are fixed in docs/14.
