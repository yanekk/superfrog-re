# T76 — bonus-enemies-a

**Split from:** T70. Carries T70's routine tests for the first half of the bonus level's object
types (its "routine conformance for each bonus object type's update").

**Phase:** 2 · **Depends on:** T70; blocks T21 · **Weight:** medium

## Goal

The bonus level's markers 1–6 and the types they spawn, ported and proven routine by routine. The
bonus level's row of `obj_flag_jumptab` (h0+$178ec + `$600`) holds only markers 1–15 and 55. Each
spawner is `obj_spawn_cell` with d0 0 and d1 `$a` (markers 1–5) or `$c` (from marker 6 on). Every
type's touch handler is `touch_bonus_enemy` (h0+$238e2, T78's). Templates (index: init, update):

- marker 1 → 113: $28af2, $28b4c. y from camera y, plus a path at h0+$28b8a (only the first word
  of each pair is used; it ends at `$ff80`). Two animations alternate by the toggle word h0+$28b4a.
- marker 2 → 114: $28a7a, $28ada. Random y (`rng_range($e0)`) and speed −(4..10); animations
  alternate by the toggle h0+$28ad8.
- marker 3 → 115: $28d52, $28d80. It bursts after 30..60 passes into twelve 116s
  (`obj_spawn_at_xy_nodedup`, speeds from the table at h0+$28dec), then frees itself.
- 116 (spawned): $28cd4, $28cf0. Its speeds decay toward 0 every 4th pass; it is freed when its
  bob's `+$1a` reaches `$7530`.
- marker 4 → 117: $28e3c, $28e52. When its speed reaches 0 it spawns a 118 and a 119, and its
  update becomes h0+$28eaa. 118 and 119 have updates $28ec6 and $28ef0, and switch to $28eaa at
  speed 0.
- marker 5 → 120: $28f1a, $28f36. Every 12 passes it spawns a 121 at (+`$20`, +`$1c`).
- 121 (spawned): $28f7a, $28fae.
- marker 6 → 122: $28fd8, $29044. A bobbing path at h0+$2900a (ends at `$80`). Every `$28`
  passes it fires four 123s (template 123 is T77's).

Add the toggles h0+$28ad8 and h0+$28b4a to `LAYOUT` and `re/symbols.toml`. The listing
mis-disassembles both as code. Read the tables from the bus (the executable is resident), with
their addresses named in the source, as `items` does.

## Files

- `engine/crates/sf-core/src/bonus/enemies.rs` (T75 makes `bonus/mod.rs`; if T75 has not merged
  yet, create the module and `register_all` here), `layout.rs`, `re/symbols.toml`
- `engine/crates/sf-conform/tests/bonus_routines.rs` (the WB-1 rig; see T75)

## Interface

Kinds registered under templates 113–122; markers 1–6 registered for world table 6.

## Tests

- [ ] Routine conformance for each init and update above, and for the six spawners.

## Done when

- [ ] Those routine tests are green; the two toggles are in `LAYOUT`.
