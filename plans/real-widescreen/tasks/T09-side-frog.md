# T09 — side-frog

**Phase:** 2 · **Depends on:** T04 · **Weight:** light

## Goal

Draw the frog in the side strips when it stands there near a level's end (or flies there in the bonus
level), whole and in the right place.

## Design sections this implements

DESIGN §2.5, §2.9.

## Files

- `engine/crates/sf-core/src/render/objects.rs` (sprite placement over the widened window)
- `engine/crates/sf-core/src/level/camera.rs` (`place_sprites`, only if the control words cannot carry
  the position)

## Interface

Measure first: the largest and smallest frog screen x in widescreen (T04's clamps) and the HSTART they
need (window position = HSTART + 1 − left; widened `left` = $79 − 65). If HSTART stays within 9 bits the
renderer needs only to stop clipping sprites to the 4:3 window; otherwise the renderer places the frog's
sprites from the engine's frog screen x in widescreen. Record the measurement in FINDINGS.

## Tests

- [ ] at a level's right end in widescreen the frog drawn in the right strip equals the frog drawn in the
  centre (same animation frame), shifted.
- [ ] at the left end, likewise in the left strip.
- [ ] 4:3 sprites unchanged (existing render tests).

## End to end (the worker drives this)

- suite: `sf-e2e`, new `widescreen_frog.rs` (not T08's file: the two run in parallel) · size: 2× widescreen
- [ ] a level's right end reached in the app in widescreen: frames show the frog's pixels in the right
  strip and equal a headless render of the same state.

## Done when

- [ ] Tests green; the HSTART measurement is a FINDINGS row.
