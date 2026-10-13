# T89 — story-copper-bus-cycles

**Split from:** T52. Carries T52's "Done when" for the masks (the fixed DMA half).

**Phase:** 2 · **Depends on:** T52; blocks T53 · **Weight:** light

## Goal

`copper::run_bus` gives the copper's bus cycles exactly as vAmiga takes them, so `story::sim::Fixed`
(T52) holds the blitter up on the original's cycles. Then the five scene-1 mask blits match.

## Found in T52 (measured)

- Mask frames (oracle 3 016–3 020, case 532–536; my oracle frame = story_dbg's `O` + 1): DMACON
  `$8400` (BLTPRI), BLTCON0 `$0dfc`, BLTSIZE `$0014` (1 024 rows × 20 words, ABD), begin = write + 3,
  free on the next frame near line 90. Refresh plus copper only: no bitplane, audio or sprite slots.
- COP1LC `$e092` (CPU writes `$80/$82` in oracle frame 3 005). List: `0180 0000, 0100 0200,
  0096 0020, ffff fffe`. `run_bus` gives 10 cycles on line 0 (h 2..20); the model's free/end came
  one cycle late (91 657 vs 91 656), so the original takes 9 there. Find which (likely the end WAIT's
  cycles; vAmiga `Copper/CopperEvents.cpp` `COP_WAIT1/2`) and fix `run_bus`, checking the copper's
  write beams stay as T46's tests have them.
- Copper WAITs without BFD (bit 15 of the second word) also wait for the blitter (vAmiga); `run_bus`
  ignores it. Note whether it matters here.
- Probe: the DMA profile (`Amiga::set_dma_profile`, `LineSample::owners`) reports each line's
  owners one sample late and stops while a BLTPRI blit starves the CPU.

## Files

`engine/crates/sf-core/src/render/copper.rs`; a test in `engine/crates/sf-conform/tests/` (new
`story_blits.rs`, shared with T90).

## Tests

- [ ] An oracle test: for oracle frames 3 016–3 020, `Sim` (fixed DMA = `Fixed` from `run_bus` over
  the original's list in its memory at each frame's start, this frame and the next), poked with the
  original's BLTCON0/BLTSIZE/DMACON writes, gives the original's begin/free/end (`next` events +
  `sim::FRAME`). Recipe: `sync::reach` the `story-intro` case, `step_inserting` with
  `input_at(of)`, follow COP1LC from `cpu_writes`, carry `Fetch` from frame to frame.

## Done when

- [ ] The masks' blits match to the cycle; T46's copper tests green.
