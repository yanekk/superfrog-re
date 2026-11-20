# T05 — wide-special-cases

**Phase:** 1 · **Depends on:** T01 · **Weight:** medium

## Goal

The objects that test the screen's edge themselves rather than through the shared helpers follow the
same rule in widescreen: Spud, the bonus level's shots and chain, the two watchers' eyes, the witch's
loop, and `set_cell_tile`'s redraw window.

## Design sections this implements

DESIGN §2.6, D10.

## Files

- `engine/crates/sf-core/src/items/powerups.rs` (`spud_update`, h0+$80f0)
- `engine/crates/sf-core/src/bonus/shots.rs` (`spawn_shots` h0+$7e3e, `shooter_spud` h0+$7f14,
  `shot_update` h0+$8070)
- `engine/crates/sf-core/src/bonus/enemies.rs` (`chain_update` h0+$293d0)
- `engine/crates/sf-core/src/enemies/common.rs` (`direction` h0+$da62) and its callers
  `enemies/w1/watcher.rs`, `enemies/w5/watcher.rs`
- `engine/crates/sf-core/src/bosses/witch.rs` (`witch_loop` h0+$25b40)
- `engine/crates/sf-core/src/level/pass.rs` (`set_cell_tile` h0+$7a40)

## Interface

Each horizontal bound moves by DESIGN §2.2: Spud parked outside `[cx − 65, cx + $150 + 65)`; shots spawn
at `cx + $160 + 65 + $10·k`, count as gone past `cx + $150 + 65`, move while `x < cx + $200 + 65`;
`direction` clamps screen x to `−65 .. $154 + 65` (callers' x − cx is signed); the chain's path offset
follows its (moved) spawn x unchanged; the witch's loop keeps its cx-relative path (moves only if T04
moves cx). `set_cell_tile`: leave as is (it only feeds the buffers; the strips draw from the map), and
say so in the commit, or widen it if a test shows a strip cell stale.

## Tests

- [ ] Spud thrown right in widescreen flies until `cx + $150 + 65`; in 4:3 until `cx + $150`.
- [ ] bonus: five shots spawn and re-fire at the widened bounds; a shot in the strip still hits an enemy
  there.
- [ ] a watcher's eye looks toward a frog in the strip (direction from the unclamped widened x).
- [ ] each in 4:3: unchanged (existing routine tests).

## Done when

- [ ] Tests green; `cd engine && cargo test -q --workspace` run once, recorded.
- [ ] A grep for `0x150`, `0x154`, `0x160`, `0x200` horizontal tests in these files finds only
  `screen.rs` calls (commit body).
