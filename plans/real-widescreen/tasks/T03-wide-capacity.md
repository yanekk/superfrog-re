# T03 — wide-capacity

**Phase:** 1 · **Depends on:** T00, T01 · **Weight:** heavy

## Goal

Give the widescreen game room for more live objects than the original's 20, so the wider view never
evicts or refuses what 4:3 would keep, while the 4:3 game never touches the new room.

## Design sections this implements

DESIGN §2.4, D4.

## Files

- `engine/crates/sf-core/src/layout.rs` (a new range: extra slots, bob records, free-list entries,
  draw-list room, hitbox entries)
- `engine/crates/sf-core/src/level/{objects,bobs}.rs` (every slot loop; `take_bob`, `park_free`,
  `build_bob_draw_list`, `obj_build_hitboxes`, `find_free`, `obj_spawn_cell`'s dedup)
- the literal `(0..20)` loops: `enemies/common.rs` (`cell_live`), `enemies/w1/mod.rs`,
  `enemies/w2/spikes.rs`, `enemies/w3/scenery.rs`, `enemies/w3/swing.rs`, `enemies/w4/mace.rs`
- `front/gameover.rs` (`PASS_WRITES`)
- `engine/crates/sf-core/src/state.rs` if the range needs a reader

## Interface

```rust
// level/objects.rs
pub const SLOTS: u32 = 20;                 // the original's
pub const WIDE_SLOTS: u32 = /* from T00 */;
/// Every slot address in the order the original walks them, then the extras while `screen.wide`
/// or any extra slot is live (DESIGN §2.4); `find_free` hands out an extra only when `screen.wide`.
pub fn slot_addrs(b: &Bus, screen: Screen) -> impl Iterator<Item = u32>;
```

The extra range sits at an address the original never maps (DESIGN §2.4), in LAYOUT address order;
its bytes are zero and unread in 4:3. Each extra slot's +$4c/+$50 point into the extra free list and
extra bob records, so every pointer the code follows stays a plain address. The bob free list stays
terminated by 0 after the extras. Every loop goes through `slot_addrs` (or an equivalent for bobs and
hitboxes): a reviewer greps `0x7c` and `0..20` and finds none left outside it.

## Tests

- [ ] 4:3: a run of W1-1, W3-2 and the bonus level for 2 000 steps gives byte-identical states before and
  after this task (a stored hash, or the conformance cases in the workspace run).
- [ ] widescreen: filling all 20 original slots, the 21st object takes an extra slot and is updated,
  hit-tested and drawn (draw list holds it); freeing returns it to the extra free list.
- [ ] widescreen: an object in an extra slot can be a chain link's ball (+$64) and a bonus `SHOTS` entry
  resolves.
- [ ] `find_free` in widescreen evicts only when every slot, extras included, is full.
- [ ] T00's heaviest scene (named in its README) stays below `WIDE_SLOTS` (an engine-only replay test).
- [ ] `PASS_WRITES` covers the extras in widescreen and not in 4:3.

## Done when

- [ ] Tests green; `cd engine && cargo test -q --workspace` run once, recorded.
- [ ] No slot loop left with a literal count (grep in the commit body).
