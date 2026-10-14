# T128 — story-setup-under-music

**Split from:** T56. Carries "that case is green" for engine frames 4562–4581 and 8216–8217.

**Phase:** 2 · **Depends on:** T56 · **Weight:** medium

## Goal

A scene's set-up job (`intro::SimJob`: `bobs::clear_buffers`, `bobs::init`, run on `story::sim`
and re-timed each frame with the original's fed acknowledges) times `objects_init` to the
original's line in the scenes where the music plays, and its memory is left out of the comparison
for as long as the original is inside it.

## Found in T56

- Engine frame 8216 (oracle 11 621, scene 13/14's set-up): the clear job matches the original to
  the cycle (second `$FA16` at line 30, cycle 114), but the sim's first mask blit starts on line
  117 where the original's starts on 132: `objects_init`'s CPU part (h0+$96d4 to the masks'
  DMACON) is about 15 lines short, the handlers' lengths fed. So the last mask blit's held handler
  lands on line 307 (original 291) and the first draw's `$8400` slips to the next frame. The
  original's CPU loses about one cycle a line to audio in that loop (`dma_story` 11 575): check
  the engine's `AudioDma` slots against it, then the instruction model (measure each segment:
  frames_load, the frames' blits, objects_reset, the clearing, the masks).
- Engine frames 4562–4563: `intro_vars` at h0+$1e, the scene table pointer (a long at h0+$1c)
  that the set-up writes; `SETUP_WRITES` covers h0+$1a for 4 bytes only. Widening it to 6 was
  tried and discarded uncommitted at T56's split; it should clear these frames.
- Engine frame 4581: `vbl_count` (h0+$1a) original `$12`, engine `$11`, at scene 8's set-up end;
  not looked at.

## Interface

None new.

## Files

`engine/crates/sf-core/src/front/story/` (`intro.rs`, `bobs.rs`); `examples/init_probe.rs`
(per-scene segment times) if extended.

## Tests

- [ ] `sf-conform scan conform/cases/story-intro-full.toml`: no divergence at 4562–4581 or
  8216–8217.
- [ ] The story tests and the `story-*` ignored cases stay green.

## Done when

- [ ] Both checks hold; the per-segment numbers that explained the 15 lines are in FINDINGS.
