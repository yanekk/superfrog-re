# T188 — chain-e2e

**Split from:** T183 (its "e2e" test)

**Phase:** 2 · **Depends on:** T183; blocks T21 · **Weight:** small

## Goal

`engine/crates/sf-e2e/tests/smoke.rs`: the real binary under `--script` from power-on, fire on the
title and on START GAME, shows W1-1 play by frame 1 500 of the engine's: captured frames
non-black, the frog's sprite present. Engine-only measurement (T183, scratch run of
`Game::step`, seed (1,2), fire at frames 300, 600, 800): the loading picture at 913, the
progress map at 2109, W1-1's level copper list at 2440, the level scene at 2465 — so either
the frame budget in the task's wording moves (to about 2 600, recorded as a deviation) or the
script fires earlier; the walk and the map's 300 frames are the original's and stay.

## Design sections this implements

DESIGN §2.1.

## Files

- `engine/crates/sf-e2e/tests/smoke.rs`; a script under `engine/conform/scripts/` if one is needed.

## Interface

None.

## Tests

- [ ] The smoke test above: frames captured non-black and the frog's sprite present by the
      budget, run by the test command.

## Done when

- [ ] The smoke test green in the test block.
