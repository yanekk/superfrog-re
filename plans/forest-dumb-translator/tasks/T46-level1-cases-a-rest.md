# T46 — level1-cases-a-rest

**Split from:** T30 (owner, 2026-11-12); carries T30's "Every case above passes" for its four cases below.

**Phase:** 5 · **Depends on:** T30; blocks T36 · **Weight:** medium

## Goal

Make the four T30 cases still failing on the translated program pass from the level sync point, as the hand engine does (owner, 2026-11-12, split from T30). After T30 (pass-end bar, CLXDAT, a pass of slack for non-Paula writes) they differ in state:

- `camera-bottom-right`, `camera-right-left`: `creature_slots+$20` is `$01cb0050` on the engine, `$01cc0050` in the original at pass 93 (oracle frame 6395).
- `character-edge-pit`: `joy_up` is 1 on the engine, 0 in the original at pass 289 (oracle frame 6691).
- `character-run-slide-buttons`: `char_x` is `$1f3` on the engine, `$1f0` in the original at pass 79 (oracle frame 6481).

Find each cause first (docs/09 lists sub-frame timing the hand engine was handed: cut redraws, a creature start the next pass cuts, input read across a frame's end). Every fix is general, in the translator, `pcode-rt` or the machine, never for one address (DESIGN §2.2); a difference the bar admits is accepted only with its measurement in the case's notes (DESIGN §2.7). If a cause is the machine's missing DMA contention (the CPU about nine lines fast a frame), say so with the measurement and ask the owner before modelling it.

## Design sections this implements

DESIGN §2.2, §2.7, §2.8.

## Files

- Whatever the causes lead to in `tools/pcode_fix/`, `crates/pcode-rt`, `crates/pcode-machine`; case notes for accepted differences. Each fix with a test that fails without it.
- Case files only for notes; a case's checks are never loosened.

## Tests

- [ ] Each fix's own test (made-up program or machine unit test).
- [ ] Full: the four cases pass with `--engine translated`, and T30's 19 still do (`run-all --engine translated` over `camera-*`, `character-*`, `stretch-*`, `picture-*`, `w1-idle`, `stub-level`).

## Done when

- [ ] Every case above passes; any acceptance is in the case's notes with its measurement.
- [ ] If the causes need more than a session, the worker asks to add a task rather than stretching this one.
