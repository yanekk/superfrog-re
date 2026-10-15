# T146 — attract-demo-entry

**Split from:** T33. Carries T33's "credits past frame 6416" test and the demo's place in the cycle.

**Phase:** 2 · **Depends on:** T33 · **Weight:** medium

## Goal

From the credits into the attract demo's level loop, and back to the title after it, as
`top_loop` h0+$a8 does. A T33 worker built all of this, then lost it uncommitted at pir's
context line; what it measured is below, so this is rebuilding, not discovery.

## What T33 found (oracle, 2026-10-15)

- **Power-on `level1_vector`** ($64): every power-on case diverged at frame 0 (FINDINGS 🐞
  2026-10-14). Fix: `State::default` sets `$64` to Kickstart 1.3's `$00fc0c8e` next to
  `EXEC_BASE`. With it `front-attract` is green with vars on.
- **Credits scroll at the original's frame 6416**: the blit and the beam meet in row 229 (about
  229.6 by arithmetic: the blit runs from line 198 to 287). The original's blit start moves
  198/2..198/11 and its end 287/116..287/184 from frame to frame (audio DMA slots), so a word of
  that row can show either way. Fix used: `front::attract::scroll` reports, through
  `cx.races`, each row whose first word the blit writes with the beam within
  `game::RACE_LINES` of that row's line (`0x2c + y`, `game::beam_race`): the owner's beam-race
  rule (2026-10-13). After it the credits are green to the demo's start (case frame 3285).
- **The demo path** (listing h0+$a8): `access_overlay_show`, `demo_world_load` (`world_load_w1`
  without FX/MU/intro), `demo_level_load` (`level_load_w1_l1` without the progress map),
  `overlays_end`, `demo_init` (byte for byte `level_init_w1`'s code at h0+$8f3a), `level_start`,
  `level_frame_loop`, `move.w h0+$9e8, opt_game_mode`, `level_exit_fade`, `bra top_loop` (no
  `game_over_check`). Every `demo_index` gives start W1-1 (h0+$1218) and `demo_init` always
  points the recording at h0+$3ad8c: the three entries play the same recording.
- Built as: `TopLoop` pc 3 in attract mode calls a `Demo` routine: overlay shown when
  `h0+$445a`, `overlay_up` are 0 and `loader_mode` is not (as `EndSeqLoad`); the load is
  `level::load::level_load` for W1-1 (made `pub(crate) demo_load`); `cx.loaded(Reached(h0+$b7e2))`;
  `OverlayEnd`; then `exit::init_on_beam` (Restart's body factored out) with `inside` =
  h0+$8f3a + 6, $2e, $34, $38, $4a; then `LevelStart`, which ends in `LevelLoop`.
- `LevelOver` in attract mode: at pc 0 copy h0+$9e8 to `opt_game_mode` (h0+$9e6); at pc 4 after
  the blank list, `Yield::Jump(TopLoop::default())`.
- When the level loop is entered `Game::step_paced` must copy the front end's keyboard register
  into `Game::sdr` (add `Front::keyboard_sdr`): the original's still holds the power-on `$7e`
  released, read by the level's `kbd_poll` (else `kbd_toggle` diverges at case frame 3340).

## Files

`front/attract.rs`, `front/mod.rs` (`Routine::Demo`, `keyboard_sdr`), `level/exit.rs`,
`level/load.rs`, `state.rs`, `game.rs`; `conform/cases/front-attract.toml` (its comment).

## Tests

- [ ] `front-attract` with vars on, run past case frame 2275 (the original's 6416) into the demo:
      green to case frame 3300 (`demo_init`'s start), its beam races counted.
- [ ] `frog-pit-w1-1`, `game-over-w1-1`, `front-boot` still green.

## Done when

- [ ] The above; `front-attract`'s frames raised to 3300 with its comment updated.
