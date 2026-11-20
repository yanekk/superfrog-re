# T78 — bonus-enemy-death

**Split from:** T70. Carries T70's routine tests for `touch_bonus_enemy` and what a shot enemy
turns into.

**Phase:** 2 · **Depends on:** T70, T75; blocks T21 · **Weight:** medium

## Goal

A bonus enemy hit by a shot, and the reward it leaves, ported and proven routine by routine:

- **`touch_bonus_enemy` h0+$238e2.** `+$1c` −= 1 + (`$7f12` >> 1). While `+$1c` is not negative,
  nothing else happens. Below 0, the bob plays h0+$32eb0, the update becomes h0+$23956, the hitbox
  list goes to 0 and the touch to h0+$7d4, and sfx 1 plays on channel 3.
- **h0+$23956**: the bob follows; once its `+$1a` is ≥ `$7530`, with d0 = 8, it continues in
  h0+$23c14.
- **h0+$23c14**, the routine a killed enemy runs. It is shared with `touch_spud_damage`
  (h0+$2360e, the enemies tasks'), so port both branches. It snaps to the ground under it through
  `map_rows` and the tile heights at h1+$31e20, and becomes a reward. In shooter mode the update
  is h0+$23e22 (path h0+$23de6, `$20` passes); otherwise it is h0+$23e6c. Its hitbox is h0+$233f2
  and its touch h0+$23eac. In shooter mode the reward's kind comes from the counters h0+$23de4
  (every `$28`th → 1) and h0+$23e20 (every 10th → 2); the RNG is drawn (`rng_range(2)`) either
  way. Animations: h0+$2cf58, $2cf64, $2cf70.
- **The reward's touch h0+$23eac.** It waits for its arc (h0+$23e90, ending `$7d00`). In shooter
  mode it plays sfx 2 on channel 3. Kind 1 adds a life (h0+$82e). Kind 2 calls h0+$8008 when
  `$7f12` is already set, then raises `$7f12`. Then sfx 5 on channel 3, 200 points, the frog's
  `+$7e` + 1, and the slot is freed.

Add h0+$23de4 and h0+$23e20 to `LAYOUT` and `re/symbols.toml`.

## Files

- `engine/crates/sf-core/src/bonus/death.rs`, `layout.rs`, `re/symbols.toml`
- `engine/crates/sf-conform/tests/bonus_routines.rs` (the WB-1 rig; see T75)

## Interface

`touch_bonus_enemy` and the reward's routines registered by address in the dispatch.

## Tests

- [ ] `collide_all` in shooter mode with a shot hitting a bonus enemy (T75 leaves this case).
- [ ] Routine conformance for h0+$238e2, h0+$23956, h0+$23c14 (shooter mode and not),
      h0+$23e22, h0+$23e6c and h0+$23eac.

## Done when

- [ ] Those routine tests are green; the two counters are in `LAYOUT`.
