# T75 — bonus-shots

**Split from:** T70. Carries T70's routine tests for the frog's side of the bonus level (the shots
and `collide_all`'s extra frog parts) and its "bonus variables in `LAYOUT`" item for those
variables.

**Phase:** 2 · **Depends on:** T70; blocks T21 · **Weight:** medium

## Goal

The frog's shots in shooter mode, ported and proven routine by routine against the original. All
offsets are h0; read them in `~/.cache/superfrog-re/re/d1_c03.lst` (`tools/re_listing.py`).

- **h0+$7e3e** (called by `level_init_lb`; T14's `level::load::level_init` leaves it out, see its
  comment): spawns five shots with `obj_spawn_at_xy`, templates 110, 110, 110, 111, 112, at x
  `$7f0a` + 0/$10/$20/$30/$40, y `$7f0e`. Their slots go into `$80dc`, `$80e0`, `$80e4`, `$80e8`,
  `$80ec`. `$7f0a` = camera x + `$160`. Quirk to keep: `$7f0e` = camera x + `$160` + `$130`. The
  code adds `$130` to d0, not d1.
- **spud_update's shooter branch h0+$7f14** (`spud_update` h0+$80f0 jumps there in shooter mode;
  today `items::powerups::spud_update` asserts it never runs there). It parks the spud
  (`$639c`, mirrored into h0+$297b2). With fire held and power `$7f12` ≠ 0, the two extra shots
  (`$80e8`, `$80ec`) are re-placed at the frog when both are past camera x + `$150`. Then comes
  the fire delay `$80c6` (3), sfx 0 on channel 2, and the cycle pointer `$80c8` walking the table
  at `$80cc` (pointers to `$80dc`, `$80e0`, `$80e4`, then 0, which wraps it).
- **The shot update h0+$8070** (templates 110–112; their inits at h0+$802e, $8044 and $805a only
  set animations). It moves while x ≤ camera x + `$200`. A shot off screen (y − camera y outside
  `$ffe0..$120`) is parked at camera x + `$200`.
- **h0+$8008**: each shot's `+$c` speed is raised by one, up to `$1e`. T78's reward calls it.
- **`collide_all`'s shooter branch h0+$d1d0**: each of the five shots is checked against the
  hitbox list. The first three use a `$20`×`$10` box, the last two `$20`×`$20`. A reward
  (update h0+$23e22) is skipped. Any other hit runs its touch handler with d7 = 0, a0 = the frog
  and a1 = the shot, and pushes the shot `$280` right. Remove `objects::collide_all`'s debug
  assertion.

Add `$7f0a`, `$7f0e`, `$7f12`, `$80c6`..`$80f0` to `LAYOUT` (and `re/symbols.toml`). `Bus` panics
on a write to an unmapped address.

## Files

- `engine/crates/sf-core/src/bonus/{mod.rs,shots.rs}` (new module), `level/objects.rs`,
  `items/powerups.rs`, `layout.rs`, `re/symbols.toml`
- `engine/crates/sf-conform/tests/bonus_routines.rs`: a rig whose memory is WB-1's first frame
  (`sync::reach` of `AtLevel("WB-1")`, as `common::level` does for W1-1). Not `sf-core/tests/`: the
  purity test keeps the emulator out of the pure crates' tests.

## Interface

`sf_core::bonus::register_all(&mut Dispatch)`, called from `level::standard_dispatch`. The
shots' kinds are registered under templates 110–112.

## Tests

- [ ] Routine conformance on generated states, as `items_routines.rs` does it, for h0+$7e3e, the
      shooter branch of `spud_update`, h0+$8070, h0+$8008 and `collide_all` in shooter mode.

## Done when

- [ ] Those routine tests are green; the shot variables are in `LAYOUT`.
