# T06 — wide-switch

**Phase:** 1 · **Depends on:** T02, T03 · **Weight:** light

## Goal

Switching widescreen on mid-level makes whatever is in the strips live at once; switching off lets the
extras drain by the 4:3 rules. The same start, inputs and switch frames always give the same game.

## Design sections this implements

DESIGN §2.7, D2.

## Files

- `engine/crates/sf-core/src/game.rs` (a pending-switch flag applied at the next step's start)
- `engine/crates/sf-core/src/level/{frame,spawn}.rs` (the switch-on scan inside the level step)
- `engine/crates/sf-conform/tests/widescreen_switch.rs` (new; engine only)

## Interface

`Game::set_widescreen(on)` records the change; the next `step` applies it before the level's pass, and
on an off→on change inside a level runs `scan_screen` with the wide `Screen` (T02's widened scan) once.
Outside a level the flag just changes. No new public API beyond T01's.

## Tests

- [ ] switching on mid-W1-1 with an enemy cell in the right strip: the enemy is live after that step.
- [ ] switching off: extra-slot objects keep running and are freed by the 4:3 despawn once out of view;
  nothing panics when the 4:3 loops meet a live extra slot (they walk extras while any is live, or T03's
  rule says how).
- [ ] determinism: two runs with switches at the same frames give identical states for 3 000 steps.
- [ ] switching on while paused, during the level's fade-in, and on its first frame.

## Done when

- [ ] Tests green; `cd engine && cargo test -q --workspace` run once, recorded.
