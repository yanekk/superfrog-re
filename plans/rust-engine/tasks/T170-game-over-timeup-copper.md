# T170 — game-over-timeup-copper

**Phase:** 2 · **Depends on:** T34; blocks T21 · **Weight:** small

Added by the T120 review (owner, 2026-10-15): `game-over-timeup-w1-1` is red in
`sf-conform -- all`, the same without T120, and no task owns it.

## Goal

`game-over-timeup-w1-1` green in `sf-conform -- all` with W1's enemies live, to its full frames.

## What is known

- It diverges at frame 271 on `level_copper` ($727c): from +$2ac on the copper's buffer pointers
  are a flip apart (original `$e00002`, engine `$e00001`), in game over's first pass (FINDINGS 🐞
  2026-10-14, "Not T42's, open"; the T79-review fault). Hover costs do not move it.
- `bonus-death-wb-1` met a like fault at 2448 after a lag (FINDINGS, T79 review, "likely T101's
  class") and `enemies-w6-4` at 1120 (T146). Check the three are not one fault before fixing
  this one alone; fix only this case's, and name in `FINDINGS.md` any shared cause.

## Interface

None new.

## Files

- wherever the game-over pass's buffer flip lives in `engine/crates/sf-core/src/`
- `engine/conform/cases/game-over-timeup-w1-1.toml` (only its comment, if needed)

## Tests

- [ ] `game-over-timeup-w1-1` green in `sf-conform -- all`.
- [ ] `game-over-w1-1`, `game-over-esc-w1-1` and `game-over-fire-w1-1` still green.

## Done when

- [ ] The above, and the case in the test block if it runs within the test block's time.
