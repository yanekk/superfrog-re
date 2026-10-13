# T93 — slot-attract-draw

**Split from:** T72

**Phase:** 2 · **Depends on:** T72, T91, T92 · **Weight:** medium

Carries T72's "Drawing on the beam" and "Jobs" items and the attract loop (T72 doc, "Then",
"Attract each frame"). T94 does the interrupt's length and the 300-frame case.

## Goal

The attract loop (h0+$4d24: raster, `slot_attract_frame` $6276, `slot_draw_lamps` $645e, fire or
mouse ends it) ported as `slot::draw::Draw`, its memory exact and its picture drawn as the beam
saw it, races excused through T91's `FrameOut::beam_races`.

## What T72 measured on the oracle (2026-10-13)

- Steady state per frame: after the raster wait (line 299) reel blits start at line 300 (7 of
  the 18, ~1.5 lines apart, the CPU in the blitter wait at the frame's end), the other 11 after
  the interrupt (lines ~18–36), then the 26 lamp blits (lines ~40–132), INTENA $10/$8010 and
  DMACON $8400 around each drawing (lamps $10 at ~27/40, $8010 at ~133; reels $10 at 300).
- Reels: three 64-line × 2-word blits a plane, six planes, FRUC (buffer 1) → buffer 0 at
  +$11b2/$11b8/$11be (picture lines 113–177): never raced. Lamps: on the sixth plane (+$c800)
  with FRUM (+$f000) as A, minterm $0c (lit, record word ≠ 0) or $fc; they race the beam.
- Fade-in frames (776–778 from the sync point) start the reels' second part at line ~52.
- Cycle costs and contention: T72's doc, "Drawing on the beam".

## Files

`engine/crates/sf-core/src/slot/{mod.rs,draw.rs}`, `front/blit.rs` if a minterm is missing;
`engine/conform/cases/slot-attract.toml` (sync `slot = "slot-none"`; start at 60 frames).

## Interface

`slot::draw::Draw` (a routine of steps yielding `Busy(line)`); fills T91's
`FrameOut::beam_races`.

## Tests

- [ ] `slot-attract` at 60 frames green on vars and picture (races excused, counted).
- [ ] sf-core unit test: one attract frame's lamp rows from the pattern (`$ff80` ends one; the
  10th and 30th turns).

## Done when

- [ ] `slot-attract` (60 frames) green on vars and picture; `front-*`, `ending`, `slot-load` green.
