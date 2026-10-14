# T131 — story-last-fade-out

**Split from:** T56. Carries "that case is green" for engine frames 9268–9282.

**Phase:** 2 · **Depends on:** T56 · **Weight:** light

## Goal

The last scene's fade out and the "insert disk 1" page match the original to the case's end
(engine frame 9282, the last before the engine hands over to disk 1).

## Found in T56

- Engine frame 9267 (oracle 12 672): the fade out's set-up (h0+$b906) runs with interrupts off
  from line 112 to 172 in the original, 110 to 163 in the engine (`fade_setup_lines`, out:
  `5 + 3/2` a colour, 53 lines for 32).
- From 9268 the whole picture differs: the engine's fade is a step ahead (colour 1 original `$fff`,
  engine `$eee`), every frame to 9282.
- `frame_descriptors` also differed there before T56 fixed `frame_prepare`'s add; check it is gone.

## Interface

None new.

## Files

`engine/crates/sf-core/src/front/story/intro.rs` (`fade_setup_lines`), `scenes.rs` (`END`).

## Tests

- [ ] `sf-conform scan` of `story-intro-full`: 9267–9282 match.

## Done when

- [ ] As the tests.
