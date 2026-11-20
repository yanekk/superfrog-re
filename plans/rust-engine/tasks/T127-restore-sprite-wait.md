# T127 — restore-sprite-wait

**Phase:** 2 · **Depends on:** T101; blocks T21 · **Weight:** medium

Added by T86's review (owner, 2026-10-14). T86 ported h0+$19c9c, the original's wait before a frog
sprite's copper words are written: with VPOS's low byte on `$1d`..`$21` (compared signed) the CPU
waits for line `$22`. It is called from `frog_anim_tick` (h0+$1027a) and `place_frog_sprites`
(h0+$19c24, h0+$19c92). The copper fetches the frog's sprite pointers on line `$20`, so a write
that waits misses them and the frame shows the pose before.

Merged with T43's live W2 enemies, the calls turned `items-switch-w2-1` and `enemies-w2-1` red
at frame 442. There the frog's tile collision starts two sound effects; the original's last write
(DMACON) is on line 26, hpos `$b7`, and its frog reaches the wait before line `$1d`. After that
write the engine's clock jumps to line 27, column 0, then charges `FrogAir` and `FrogAnim` (about
2.3 lines), so it reaches the wait on line 29.2 ($1d), waits, and misses the pose. The calls are
left out; `LevelCtx::sprite_wait` and its test (`frog_routines.rs`
`the_frog_sprites_wait_out_lines_1d_to_21`) are kept.

## Goal

1. Find why the engine reaches `frog_anim_tick` late on frame 442: the anchoring after the sound
   effect's last write (`LevelCtx::scheduled`, `anchor_to`), or the fitted `FrogAir`/`FrogAnim`
   costs (`examples/fit`). Fix it in the beam model, not around the wait.
2. Put `c.sprite_wait()` back at both call sites in `sf-core/src/frog/anim.rs` (the comments there
   name this task).

## Interface

No new interface: `LevelCtx::sprite_wait` exists; only its two calls come back.

## Tests

- `frog_routines.rs`: the test asserts again that `place_frog_sprites` and `frog_anim_tick` called
  on line `$1e` end on line `$22` or later.
- `items-switch-w2-1`, `enemies-w2-1` and `items-launcher-w2-2` green with the calls in; every
  other level case still green.

## Files

`engine/crates/sf-core/src/frog/anim.rs`; `engine/crates/sf-core/src/level/{mod.rs,beam.rs,frame.rs}`;
`engine/crates/sf-conform/tests/frog_routines.rs`.

## Done when

- [ ] Both calls to `sprite_wait` are back, with the call-site test.
- [ ] `items-switch-w2-1`, `enemies-w2-1` and `items-launcher-w2-2` green, and every case green
      before stays green.
