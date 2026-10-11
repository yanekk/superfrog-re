# 09 — Object system

How `d1_c03` places, runs and draws enemies, items and the frog, for the mechanic tasks
(T14–T19). Read [`08-executable-map.md`](08-executable-map.md) first for the memory map and the
50 Hz / 25 Hz split. Addresses are `h0+$off` in `d1_c03`; names are in `re/symbols.toml`.
**(inferred)** marks what was not confirmed by the code or the oracle.

## Placement: the map holds the objects

**There is no separate object file.** Every level object is a *marker* in the T7MP BODY: the
6-bit field docs/04 calls "flags". In RAM the loader rewrites each cell as `tile << 6 | marker`
(confirmed: a coin cell reads `$3E09`, tile `$F8`, marker 9). The same 6-bit value means
different things in different worlds:

- **`obj_flag_jumptab`** (`$178ec`): 7 × 64 entries (`bra.w`), indexed by `cell & $3F`. The long
  `obj_world_offset` (`$9fa`) picks the world: `$000` W1 … `$500` W6 (also the final level), `$600`
  the bonus level. The per-world level-init routines (`level_init_w1` `$8cd8` …) set it.
- Each entry is a stub that loads a template (`a0`) and a pixel offset (`d0/d1`), then branches to
  a spawner.
- `map_spawn_scan_screen` (`$1777c`) runs the stubs for the whole first screen (18 rows × 24
  columns) at level start. `map_spawn_scan_edges` (`$17810`, from the VBL handler) runs them for
  the row or column scrolling into view. Edge scans only dispatch markers 1..`$39`.
- Collecting or killing an object usually clears its marker in RAM (`andi #$FFC0`), so it does not
  respawn.

The frog's own reactions to markers are separate tables, by body part: `$e5ba`, `$e6ba` and
`$e7ba`, plus `$f092`/`$f192` for markers 61 and 63. Known values: 6 deadly; 38 secret passage
(docs/07); 39–43 springs; 46/47 and 56/57 push or speed; 54 level exit (`level_state = 1`); 48–51
bump blocks; 55 world fruit. W3 and W4 use 24–28 as object markers, so docs/07's "solid iff
`flag & 0b11000 == 0b11000`" is not literally true for those worlds.

Other objects come from objects or from the frog: parents spawn children (bullets, debris, chain
segments) at pixel positions, and bump blocks spawn items.

## The slot record

`obj_slots` (`$1ba6c`): 20 slots of `$7c` bytes; `obj_count` (`$1ba6a`) is the number live.
`obj_run_all` (`$1b324`) calls each live slot's update routine once per 25 Hz pass.

| Off | Size | Meaning |
|---|---|---|
| `+$00` | L | init routine |
| `+$04` | L | update routine; a state change rewrites it |
| `+$08` / `+$0A` | W | x, y in level pixels (spawn: column·16 + d0, row·16 + d1) |
| `+$0C` / `+$0E` | W | vx (inferred), vy |
| `+$10` | L | touch handler: `a5` = slot, `a0` = frog, `d7` = 0 frog contact / 1 spud hit |
| `+$14` | L | hitbox list: 8-byte entries (w, h, dx, dy), ending at a negative word; 0 = none |
| `+$18` | L | hit or death animation |
| `+$1C` | W | hit points: the spud takes `$20`; `$7D00` = immune; negative = no contact |
| `+$1E` | W | score added to `score` (`$834`) |
| `+$26` | L | bob graphics/priority, copied to the bob |
| `+$2A` | W | evictable: when all slots are full, an off-screen one with this set is reclaimed |
| `+$2E` | W | stompable: landing on it calls the touch handler and bounces |
| `+$30` | W | passive: contact calls the handler instead of hurting (items, triggers) |
| `+$40` | L | the map cell it came from: non-zero = slot live; also the de-duplication key |
| `+$44` | L | spawn x,y (de-duplication for pixel spawns) |
| `+$4C` / `+$50` | L | its bob free-list entry / its bob record |
| `+$54`…`+$72` | | per-type scratch (`+$5C` timer, `+$64` state, …) |

`+$20`, `+$22`, `+$24`, `+$2C` and `+$32` are template constants of unknown use.

**Templates** (`obj_templates` `$1c6e2`): 160 blocks of `$AE` bytes. A spawner writes x/y into the
template, then copies its first `$34` bytes over the slot and calls `+$00`. What bytes
`$34`–`$AD` hold is unknown.

**Spawners:**
- `obj_spawn_cell` (`$1b3d8`): skips a cell that is already live. When the slots are full it evicts
  an evictable off-screen slot (`$1b95e`).
- `obj_spawn_cell_alt` (`$1b4e6`): a variant of `obj_spawn_cell`.
- `obj_spawn_coin_forced` (`$1b5d0`): coins only. It may free a visible slot.
- `obj_spawn_at_xy` (`$1b706`) / `obj_spawn_at_xy_nodedup` (`$1b822`): spawn at a pixel position.
- `obj_free` (`$1b90e`): frees a slot and its bob.

**Collision** runs in each pass. `obj_build_hitboxes` (`$d2f6`) rebuilds a list at `$d350` (12-byte
entries: x1, x2, y1, y2, slot; ends with `$FFFF`). `collide_all` (`$d0cc`) then tests the spud against
objects (`d7` = 1), then the frog (`d7` = 0), then 5 extra frog parts in the bonus level.

**Touch handlers** by kind:

| Handler | Meaning |
|---|---|
| `touch_kill_1hit` `$23582` | dies in one hit |
| `touch_spud_damage` `$2360e` | spud damage |
| `touch_hurt` `$236ac` | always hurts |
| `touch_boss_hit` `$236b4` | boss hit counter |
| `touch_bonus_enemy` `$238e2` | bonus-level enemies |
| `touch_coin` `$239f6` | coin |
| `$7d4` | nothing (scenery) |

## Types per world

110 distinct templates are spawned from maps. Five more are referenced by nothing: four moving
platforms and one other.

Shared by worlds 1–6:
- coin (`$1c6e2`; init `$23f60`, update `$23f86`; markers 8/9),
- exit lock (`$1ca48`; marker 29; spawns while `coins_left_for_exit` is non-zero),
- hidden coin spring (`$1c99a`; marker 45).

| World | Types | New (not shared) | Enemies / hazards | Items | Other |
|---|---|---|---|---|---|
| W1 Magic Woods | 15 | 12 | 10 | 1 | 4 |
| W2 | 20 | 17 | 11 | 1 | 8 |
| W3 | 21 | 18 | 12 | 1 | 8 |
| W4 | 26 | 23 | 15 | 1 | 10 |
| W5 | 20 | 11 (6 shared with W2) | 13 | 1 | 6 |
| W6 + final level | 14 | 11 | 11 | 1 | 2 |
| Bonus level | 15 | 15 | 15 | 0 | 0 |

Two notes on the table:
- "Enemies / hazards" are the types whose contact hurts (`+$30` = 0).
- "Other" covers scenery that only animates, crushers, and solid blocks that write marker `$3E`
  into the map.

Per-marker templates, with their init, update and touch routines, can be regenerated by walking
`obj_flag_jumptab`.

**Bosses:** there is no per-world boss scene or routine. Each world ends with its `LnET` picture
(`end_world_pic_w1` …). The one boss-like type is W6/final-level marker 1: template `$221f6`, init
`$259ae`, update `$25a16`, touch `touch_boss_hit`, 128 hit points. It spawns 8 rows above its marker
cell, flashes the palette on defeat and sets `level_state = $64` (inferred: the final witch).

**For T16–T18** (owner, 2026-10-11: re-split at plan review): about 47 new types in W1–3 and 45 in
W4–6. That is several sessions each, while T18 has only the witch. The proposed split is one task per
world; T18 becomes the final level and the witch; the bonus level's 15 types go to T19.

## The frog, the spud, bobs and sprites

**Frog** (`frog` `$7d6`): a record of about `$B4` bytes (inferred). It is updated in the VBL handler
at 50 Hz.
- Position and motion: `+0`/`+2` x, y; `+4`/`+6` vx, vy.
- State flags: `+$24` on ground; `+$3A` invulnerable or hurt; `+$3E` dying; `+$54` on a platform;
  `+$66` crouch.
- Counters: `+$58` lives; `+$5C` HUD coins.
- Spud: `+$72` spud in flight; `+$74`/`+$76` spud x, y; `+$7C` spud enabled.
- Level clock digits: `+$A0`–`$A3`.
- Routines: `frog_hurt` `$1c5bc`, `frog_die` `$de0`, `frog_reset` `$902e`.

**Spud:** one projectile, `spud_update` (`$80f0`) in the pass. It fires on CIA-A PRA bit 7, and its
bob is at `$297b6`.

**Bobs:**
- `bob_free_list` (`$1ba16`) points at 20 `bob_records` (`$298d2`, `$11C` bytes each).
- Each record has 4 sub-records of `$46` bytes (one per buffer, inferred).
- A free bob is parked at `$7D00,$7D00`.
- `build_bob_draw_list` (`$19b42`) draws the spud first, then the objects.

**Hardware sprites:** the HUD and score pop-ups use `$1C`-byte sprite records (`$1aff4`, `$1b010`,
`$1b128`, `$1b144`), which `$1b208`/`$1b23e`/`$1b308` animate.

**Sound:** `sfx_play` (`$12a72`) plays an effect from `LnFX`. `sample_play` (`$148b8`) streams a
sample on channel 3, for example the frog's random idle sound.

## Busy scene **(oracle)**

The `play` run in `exe_map_claims.rs` walks W1-1 right with a jump every 30 frames. It peaks at **15
live slots** at frame 7 376 (14 coins and one enemy). A probe run of the same walk also reached 15
at frame 7 813 (3 bees, 10 coins, 2 others). Lag clusters in these stretches (docs/08).
