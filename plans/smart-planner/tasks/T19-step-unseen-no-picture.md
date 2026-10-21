# T19 — step-unseen-no-picture

**Phase:** 1 · **Depends on:** T00; blocks T04 · **Weight:** light

Added by T00 (owner, 2026-10-21): the spike's map build was over ~5 min a level, and the profile
showed why.

## Goal

`Game::step_unseen` stops drawing the level picture nobody reads, so a movement map builds about
three times faster. Performance only: the game state after every frame stays exactly what `step`
leaves, and `step` (conformance, the app) is untouched.

## What T00 found

- `Scene::step` (`sf-core/src/scene.rs`) calls `render::level_as_drawn` on every level frame,
  whatever `step_unseen`'s `draw` flag; the flag only skips the fallback `self.render(view)` in
  `Game::level_frame`. A one-thread `sample` of `step_unseen` on W1-4: 72% of the time in
  `level_as_drawn` (`render::panel::fetch` 45%, `render::objects::compose` 18%).
- A local patch skipping `level_as_drawn` (never committed) left the state identical to the drawing
  run over 3000 frames on W1-4 and W6-3, and cut a map build's CPU time 3.2×
  (`plans/smart-planner/spike/q3-profile.txt`).

## Design sections this implements

DESIGN §5 (the measured speeds), the T00 gate in PLAN Phase 0. No game mechanic or faithful-bar
change (§1 Stance).

## Files

- `engine/crates/sf-core/src/scene.rs`, `engine/crates/sf-core/src/game.rs`: pass `draw` into
  `Scene::step`; with `draw` false, no `level_as_drawn` (the frame's picture stays empty, as
  `step_unseen`'s doc already promises).
- `engine/crates/sf-conform/tests/step_unseen.rs`: the existing equality test, plus W1-4 and W6-3.

## Interface

None new. `Game::step_unseen`'s doc comment says the picture is skipped in `Scene::step` too.

## Tests

- `step_unseen_leaves_the_game_as_step_does` keeps passing (whole `Game` and `probe()` equal on every
  frame), and gains a W1-4 and a W6-3 route (`enemies-w1-4`, `enemies-w6-3` scripts, ~2000 frames
  each); under ~30 s together, else `#[ignore]` with the time noted (DESIGN §5).
- Every conformance and render test is unaffected (they use `step`); the task's regression set
  includes `sf-conform`'s render cases for one level.

## Done when

- [ ] `step_unseen` skips `level_as_drawn`; `step` unchanged.
- [ ] The equality test passes on W6-1, W1-4 and W6-3.
- [ ] One-thread `step_unseen` frames/s on W1-4 and W6-3 before and after, measured from `Start::Level`
      and written as a FINDINGS row with the machine's load average.
