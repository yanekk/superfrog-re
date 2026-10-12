# T34 — game-over

**Phase:** 2 · **Depends on:** T15, T20; blocks T21 · **Weight:** medium

Added by T20 (owner, 2026-10-12): `game_over_seq` (h0+$74d0) is not a front-end screen. It spawns
a level object (`obj_spawn_at_xy_nodedup` with `obj_templates+$ae`, at the camera plus
`$82`, `$80`) and runs the level's object update (`obj_run_all`), bob drawing
(`build_bob_draw_list`, `bob_draw`) and scroll (`h0+$172b4`, `scroll_latch`) over the level the
frog died in, so it needs T14 and T15's object and bob code.

## Goal

Game over in the engine, from a level's last life lost to the return to `top_loop`:
`game_over_check` (h0+$749c, ESC or no lives), `music_fade_wait`, the GAME OVER object rising for
its frames, the wait for fire (up to 350 frames), and back to the title through T20's front end,
every frame matching the original.

## Design sections this implements

DESIGN §2.1; docs/08 "Scene flow"; docs/13.

## Files

- `engine/crates/sf-core/src/front/gameover.rs` (T20's `front` module, a routine like the title's)
- `engine/conform/{scripts,cases}/game-over-*`; `docs/13-front-end.md` — the game-over section.

## Interface

None new: game over is a routine on T20's `front` stack, entered from the level through `Game::step`.

## Tests

- [ ] game over entered from a transplanted state (the frame `game_over_seq` begins, after the last
      life in W1-1), untouched to the title, and the same with fire pressed during it.
- [ ] ESC in a level (`abort_flag`) to the title.

## Done when

- [ ] All cases green; one in the test block.
