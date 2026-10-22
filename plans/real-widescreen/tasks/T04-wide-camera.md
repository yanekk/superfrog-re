# T04 — wide-camera

**Phase:** 1 · **Depends on:** T01 · **Weight:** medium

## Goal

In widescreen the camera stops earlier at a level's left and right ends, so the wide view never shows
past what the original's view can show and nothing is black; the frog is no longer hidden at the old
screen edge; the bonus level's frog may fly into the strips.

## Design sections this implements

DESIGN §2.5, D3.

## Files

- `engine/crates/sf-core/src/level/camera.rs` (`keep_inside`, `shooter` frog clamp)
- `engine/crates/sf-core/src/level/load.rs` (`start_point`'s copy of the clamp)
- `engine/crates/sf-core/src/frog/anim.rs` (sprite hide at $150 / −$20)
- `engine/crates/sf-core/src/level/screen.rs`

## Interface

Widescreen clamps: `cx ≥ $10 + 65`, `cx ≤ level_width − $168 − 65` (both copies); bonus frog
`cx − 8 − 65 .. cx + $12a + 65`; frog sprite hidden at screen x ≥ $150 + 65 or x + $20 < −65. Vertical
clamps unchanged.

## Tests

- [ ] at W1-1's start in widescreen the camera is at ≥ 81 and the wide view's first column shows level
  x ≥ 16 (no colour 0 from off the map in a render of that frame).
- [ ] driving right to a level's end in widescreen, the camera stops at `level_width − $168 − 65` and the
  frog keeps walking to its own clamp; the wide view's last column is level x `level_width − 26`.
- [ ] 4:3 clamps unchanged (both copies).
- [ ] bonus level: the frog can be steered to `cx − 73` and `cx + $12a + 65` in widescreen, not in 4:3.
- [ ] the frog's sprites stay shown at screen x $150..$150+64 in widescreen.
- [ ] final level: record in FINDINGS whether the widescreen clamp moves its camera from the original's
  start point and by how much; the witch fight still starts (engine-only run of 1 000 steps, no panic).

## Done when

- [ ] Tests green; `cd engine && cargo test -q --workspace` run once, recorded.
- [ ] If the final level's camera moves and the witch's path leaves the arena, the owner was asked
  before anything else is changed (DESIGN §2.5).
