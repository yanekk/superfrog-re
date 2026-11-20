# T97 — story-split-handler

**Phase:** 2 · **Depends on:** T54; blocks T56 · **Weight:** light

Added by T55 (owner's yes, 2026-10-13).

## Goal

A vertical-blank handler that a draw's blit holds partway through, across a frame's end, splits
its work between the two frames as the original's does. The engine runs the whole handler in
one frame today.

## Found in T55

- Oracle frame 10087 (engine frame 6682 of `sf-conform scan` from `power-on-disk3`): the
  interrupt is taken at line 300, the music's writes come at 305 and `intro_vars` counts. Then a
  blit holds the CPU inside the effects' tick (chain `h0+$38a10 < $b15e`), so `sfx_records` steps
  and `INTREQ = $20` is written only at line 59 of the next frame. That next frame's blank has
  no handler of its own.
- The engine is back in step after the two frames (`var sfx_records` at 6682, `paula write #0`
  `INTREQ` at 6683).
- Conformance already feeds which frames had a handler (`sync::handlers`, T55): a handler counts
  in the step it first writes in. This task is about where the handler's work stops when a blit
  holds it.

## Interface

None new, or a conformance feed beside `Game::set_blank_merged` if the engine cannot time the
hold itself (T54's handler-end work may give it).

## Files

`front/story/` (`intro.rs`, `vbl.rs`, `sound.rs`); `sf-conform/src/` (`lib.rs`, `sync.rs`) if fed.

## Tests

- [ ] `sf-conform scan` on `story-scene4-sound` extended past frame 6683 (or a new case to
  there): no divergence at 6682–6683.

## Done when

- [ ] Frames 6682–6683 match; no other split handler shows in the scan to the insert-disk-1 page.
