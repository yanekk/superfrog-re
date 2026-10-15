# T156 — attract-demo-init-timing

**Split from:** T33. Carries the "every frame matching" part of T33's goal over `demo_init`.

**Phase:** 2 · **Depends on:** T155 · **Weight:** medium

## Goal

`demo_init` on the beam as the original's, in the first demo: its Paula writes in the frames
and order of the original's (case `front-attract`, frames 3300–3314).

## What T33 found (oracle, 2026-10-15)

With `init_on_beam` and today's `BLANK_COSTS` the engine's `DMACON $8400` (the sheet's masks)
came a frame late, 53 lines. Two model gaps, both fixed in T33's lost work and checked against
`frog-pit-w1-1`, `bonus-death-wb-1`, `game-over-w1-1` and `items-checkpoint-w1-1`:

1. **A restart's first frame runs under the level's five planes.** `level_exit_fade` sets the
   blank list, but COP1LC changes at the next blank, so `level_init`'s first frame loses CPU to
   the fifth plane (DMA owners: `4c2c351c`, copper every other slot). The fitted `FrameRecord`
   (337) had absorbed it; the demo's first frame shows the four-plane overlay (nothing taken).
   Fix: `Timeline::run` takes `ending::stolen_per_line` (CpuWork's rule, factored out) on lines
   `cx.fetch` shows, unless the frame's list is the blank one (new `Cx::list`, the list the frame
   started with). Refit `FrameRecord` to 240 (192 records; the demo's profile: 89 lines).
2. **Blitter time runs on through the vertical blank.** `MaskClear10`/`MaskOr10` and
   `Tick::Blit` are the blitter's: `Timeline` keeps them as blit stretches, and after a `Poll`
   credits the lines from the frame's top to `cx.line`. The demo's mask blit spans two blanks
   (512 lines), the restart's one (515). Refit `MaskClear10` 44, `BLANK_COSTS` `MaskOr10` 64.

Result then: 8400 at restart 297/67 (orig 69), demo 3303/301 (308); 0400 268 (271) and 3305/190
(194); 9c=0010 300/287 (281) and 3307/259 (263).

## Interface

None new, as T33: the demo runs inside `Game::step`, reached from T20's attract cycle under
`Start::PowerOn`.

## Files

`level/timeline.rs`, `level/beam.rs`, `front/ending.rs`, `front/mod.rs` (`Cx::list`).

## Tests

- [ ] `front-attract` green to case frame 3314.
- [ ] `frog-pit-w1-1`, `bonus-death-wb-1`, `game-over-w1-1`, `items-checkpoint-w1-1` green.

## Done when

- [ ] The above.
