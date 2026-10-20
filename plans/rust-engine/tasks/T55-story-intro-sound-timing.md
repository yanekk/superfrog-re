# T55 — story-intro-sound-timing

**Phase:** 2 · **Depends on:** T50; blocks T56 · **Weight:** medium

**Split from:** T46.

## Goal

The intro's Paula writes match where the scan (T46, before T49–T54) showed them early, late or a
step apart: DMACON $8400 / INTENA $c000 and the sound's writes (frames ~3185–3631, 3741–3832) and
the effects' volume ramps one step apart (`AUD0VOL` 3816, 3872–3990, 4551, 5056).

## Notes

Re-scan first (`sf-conform scan` on a case to the intro's end): many of T46's misses were one
frame-phase slip from frame 3438 that never re-synced, so what is left after T49–T54 is the work.

## Interface

None new.

## Files

`front/story/` (`sound.rs`, `intro.rs`).

## Tests

- [ ] `sf-conform scan` on a case to the insert-disk-1 page lists no Paula-only divergence.

## Done when

- [ ] No Paula-only divergence left in the scan to the insert-disk-1 page.
