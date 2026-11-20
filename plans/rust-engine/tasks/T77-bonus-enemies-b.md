# T77 — bonus-enemies-b

**Split from:** T70. Carries T70's routine tests for the second half of the bonus level's object
types, and the end marker.

**Phase:** 2 · **Depends on:** T70, T76; blocks T21 · **Weight:** medium

## Goal

Markers 7–15 and 55 of the bonus level's row of `obj_flag_jumptab` (h0+$178ec + `$600`), and the
types they spawn. They are ported and proven routine by routine. All use `obj_spawn_cell` with d0 0
and d1 `$c`; every touch handler is `touch_bonus_enemy` (T78's). Templates (index: init, update):

- marker 7 → 124: $291a0, $2921c (path h0+$291e2, negated, ends `$80`).
- marker 8 → 125: $2924a, $29280.
- marker 9 → 126: $292a0, $292d2. Path at h0+$3b48a in 4-byte steps (ends on a negative word);
  every `$20` passes it fires a 123 (T76's) left.
- marker 10 → 127 at the cell, and 128 one row below: inits $29348 and $29388, update $293d0.
  Each follows a path through h0+$1c41c, a helper (`(+$54)` long → x,y, plus d0,d1; it wraps
  at −1 to `+$58`). It is freed at a `$fffe` word. When the flag word h0+$293ce is set, its
  update becomes h0+$29418, which flies it left.
- marker 11: sets h0+$293ce to 1 (the inits clear it).
- marker 12 → 129: $29434, $29460, and marker 13 → 130: $2949c, $294c8. Both use path h0+$3b48a
  in 8-byte steps, minus `$80`.
- marker 14 → 131: $29504, $29570 (path h0+$29536). Every `$28` passes it restarts its
  animation and fires three 132s.
- 132 (spawned): $29666, $2967c.
- marker 15 → 133: $2969c, $296ea (path via h0+$1c41c, a random y offset).
- marker 55 (h0+$18abc): `level_state` = 2, the level's end. T79 plays it.

Add h0+$293ce to `LAYOUT` and `re/symbols.toml`. Read the tables from the bus with their
addresses named, as `items` does.

## Files

- `engine/crates/sf-core/src/bonus/enemies.rs` (T76 makes it; this task adds to it), `layout.rs`, `re/symbols.toml`
- `engine/crates/sf-conform/tests/bonus_routines.rs` (the WB-1 rig; see T75)

## Interface

Kinds registered under templates 124–133; markers 7–15 and 55 for world table 6.

## Tests

- [ ] Routine conformance for each init and update above, h0+$1c41c, and the spawners (markers 11
      and 55 included).

## Done when

- [ ] Those routine tests are green; h0+$293ce is in `LAYOUT`.
