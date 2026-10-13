# T50 — story-objects-init-display

**Phase:** 2 · **Depends on:** T49; blocks T56 · **Weight:** medium

**Split from:** T46.

## Goal

Scene 4's set-up (frames 3175–3192) matches: `objects_init` runs while the scene's five-plane
display still fetches (no black list after `scene_out`), which slows its CPU and its blits.

## Found in T46 (oracle)

- The masks' clear loop (`clr.b (a0)+; subq.l #1,d0; bne.b`, h0+$8408, `$a000` turns) ran 4 009
  turns a frame under the display: 16.97 cycles a turn on the bus model (15.133 with refresh only).
  Splitting `INIT_CPU` into the loop (timed on the bus, `BlitClock` stepping the ops `rpw pnn npp`)
  and the rest (`656 124 - 619 850` plus 206 a frame, unslowed) put the masks' DMACON $8400 at
  line 154 against the original's 146: refine.
- The engine's five mask blits then took the same total as the original's.
- Order at the masks' end: the original writes `DMACON = $400` before the vertical blank's handler
  the last mask blit held off when that blit started on a line the display fetches (frames 3191,
  3760, 4238), else after it (536, 1755, 2863, 3438, 4580, 4974, 5452).
- The first draw after them ends ~17 lines early in the engine: draw timing is T52–T54's.

## Interface

None new.

## Files

`front/story/intro.rs`, `front/story/clock.rs`.

## Tests

- [ ] A case to frame 3195 (sync `power-on-disk3`): `sf-conform scan` lists nothing before the
  first draw-timing miss (outside the test block if slow).

## Done when

- [ ] `sf-conform scan` to frame 3195 lists nothing before the first draw-timing miss; docs/13
  notes the set-up under the display.
