# T194 — w6-2-sfx-double-blank

**Phase:** 2 · **Depends on:** T147; blocks T21 · **Weight:** medium

Added by T147 with the owner's yes, 2026-10-16. `enemies-w6-2-complete` (T147) reaches W6-2's exit
and plays alone green (check 4, 334 milestones), but checks 1–3 fail on one frame of 8430: a sound
event on channel 3. T146, which owned W6 completion faults, was dropped.

## Goal

First re-run `sf-conform -- run conform/cases/enemies-w6-2-complete.toml` on the current feature
branch: a pass spanning two blanks may be the family T184 (alone-pass-drift) and T185
(w6-4-lag-frog-fall) fixed. If it is green, close at once: move the case into the test block. Note:
T147 measured it red with both already merged. Otherwise find why the engine's channel-3 sound
effect is a step off there, fix it in the engine, and put the case in the test block.

## What T147 measured

- Original, channel 3: a sample started at 6658 (period `$be`), then period `$be` 6659, 6660, `$80`
  6661, 6662, 6663, `$be` 6664 (`sf-conform record … --paula`). The engine writes period 152 (`$98`)
  at 6661.
- Checks 1–3 re-sync at 6665 on `sfx_chan0` (`$c13f92`): original `$98`, engine `$80`. The engine's
  sound-effect step is offset by one.
- Frame 6661 is `Pacing::Late { blanks: 2, line: 0, hpos: 7, … }`, inside a lag stretch 6597–6698;
  the last coin was taken at 6650. Not unique (T147 review, `record --every`): 54 of the case's
  frames have `blanks: 2` and 8393 also has `line: 0`; `scan` flags none of them, so `blanks: 2` alone
  does not explain it.
- `sf-conform scan` lists 6664 as the case's only differing frame.

## Interface

None new.

## Files

- The engine's sound-effect or pacing code the fault is in (`sf-core`), and a test that fails
  without the fix.
- `engine/crates/sf-conform/tests/enemies_w6_cases.rs`: `enemies-w6-2-complete` added.
- `engine/conform/cases/enemies-w6-2-complete.toml`: its comment updated.

## Tests

- [ ] `enemies-w6-2-complete` green in `sf-conform -- run` (checks 1–4).
- [ ] A test pinning the channel-3 event at 6661 (or the `blanks: 2` pacing it comes from).
- [ ] The case's test pins the exit (`level_state` 1 at 8429, as W5's `completes`): check 4 alone
  stays green when the script stops short of it (T147 review).

## Done when

- [ ] The case is green and in the test block, with no threshold changed (DESIGN §2.1).
