# T08 — world-start-recording

**Phase:** 0 · **Depends on:** — · **Weight:** medium

## Goal

Start Level… and `--level N` (T09) start any world with or without the cheat, and a bug recording must
say so, or its replay starts somewhere else. Extend the recording format with world starts and the
cheat, the shared start path that both the app and the replay use, and `fd-conform bug replay` for
those starts.

## Design sections this implements

DESIGN §2.5.

## Files

- `games/forest-dumb/engine/crates/fd-core/src/recording.rs`: `LevelId` W1-1 to W6-1, `StartPoint`,
  version 3, its tests.
- `games/forest-dumb/engine/crates/fd-core/src/recording.rs` again: the start's input and writes,
  beside `to_level_1_input`, and re-exported from `fd-app/src/lib.rs` as it is, so the app and
  `fd-conform` (which does not depend on `fd-app`) start alike.
- `games/forest-dumb/engine/crates/fd-conform/src/bug.rs` (`replay_translated`).
- `games/forest-dumb/engine/crates/fd-conform/tests/recording_compare.rs` or a new test, labelled.

## Interface

```rust
pub enum LevelId { W1_1, W2_1, W3_1, W4_1, W5_1, W6_1 }   // ALL: all six; name() "W2-1" …
pub enum StartPoint { Level1, PowerOn, World { level: LevelId, cheat: bool } }
// header fields, version 3: start = "world" , level = "W3-1", cheat = "on"|"off"
// versions 1 and 2 read as today; "level-1" stays Level1 (world 1, cheat off, the old path)

/// The writes made as load_world is entered for a world start: (symbol, value).
pub fn world_start_writes(level: LevelId, cheat: bool) -> Vec<(&'static str, u32)>;
```

`world_index` is `level`'s index (0 to 5); `cheat_flag` is written only when `cheat`, with the value
T01 or `level-end.toml` measured (1).

## Tests

- [ ] Header round trip for every world, cheat on and off; a version 2 and a version 1 zip still read.
- [ ] An unknown level name, a `world` start without `level`, a cheat value other than on/off: errors.
- [ ] `replay_translated` of a world 3 cheat-on recording (made headless by the same start path) reaches
      world 3 with `cheat_flag` set and the recording's final state.

## Done when

- [ ] Version 3 reads and writes world starts; old zips replay unchanged.
- [ ] `bug replay` of a world start reaches the recorded state; tests labelled.
