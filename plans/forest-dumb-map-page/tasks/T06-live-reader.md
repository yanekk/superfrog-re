# T06 — live-reader

**Phase:** 1 · **Depends on:** — · **Weight:** medium

## Goal

What the live page shows comes from the game's memory at a frame's end. Write the pure reader that turns
the translated program's RAM, or `fd-core`'s `Game` under `--engine hand`, into one value for any world:
whether a level is in play, which, where the character is, items left, the creatures in view, and the
object cells that differ from the level file.

## Design sections this implements

DESIGN §2.5, §2.7, §3.1.

## Files

- `games/forest-dumb/engine/crates/fd-core/src/live.rs` (new), `src/lib.rs`, `src/layout.rs` (add
  `object_map` at `h0+$cb38`, 38 400 bytes, if not named).
- `games/forest-dumb/engine/crates/fd-core/tests/live.rs` (new, made-up RAM);
  `games/forest-dumb/engine/crates/fd-conform/tests/live_reader.rs` (new, headless runs).
- `games/forest-dumb/smoke.toml`.

## Interface

```rust
pub struct Live { pub playing: bool, pub world: Option<u8>, pub char_xy: Option<(u32, u32)>,
    pub items_left: Option<u16>,                 // decoded from BCD
    pub creatures: Vec<(i32, i32, u8)>,          // active slots: x, y, kind
    pub changed: Vec<(u16, u16, u8)> }           // cells differing from `start`
/// `start`: the level file's object map for that world (`fd_assets::Level`), column-major as RAM holds it.
pub fn live_from_ram(chip: &[u8], slow: &[u8], start: &dyn Fn(u8) -> Option<Vec<u8>>) -> Live;
pub fn live_from_game(g: &Game, start: &[u8]) -> Live;
/// The bytes `live_from_ram` reads, so the app's program thread copies only these (DESIGN §3.4).
pub fn live_ranges() -> Vec<(u32, u32)>;
```

`playing` is `irq_count != 0` (docs/09); `world` is `world_index + 1`. `char_xy` is `char_x + $e`,
`char_y + 9` (the cell rule's spot, `recording.rs`).

## Tests

- [ ] Made-up RAM: not playing; playing world 3 with two active slots; a taken coin and a written-back
  creature in `changed`; BCD items left; ranges cover every byte read.
- [ ] Headless translated run (full): power-on to W1-1's play, then T84's route's first 600 steps: `char_xy`
  equals `location_in_ram`'s spot each frame, items left falls with each coin, `changed` holds each taken cell.
- [ ] Headless `Game` (smoke): the same route steps give the same `Live` from `live_from_game` as from RAM of
  the translated run at matched frames, where the two agree today (W1-1's seam).
- [ ] World 2 (full): through the `world-2-start` way in, `world` 2, `playing` from its first pass.

## Done when

- [ ] `fd_core::live` is pure (fd-core's I/O guard passes) and its tests pass.
- [ ] The headless tests pass and are labelled.
