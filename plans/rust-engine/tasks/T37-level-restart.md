# T37 — level-restart

**Phase:** 2 · **Depends on:** T36; blocks T34 · **Weight:** heavy

Added by T15 (owner, 2026-10-13). What follows a level's loop, as `top_loop` runs it: the loop's
end (`level_state` 0, `player_died`, ESC), `level_exit_fade` (h0+$75f2: the palette faded out,
the window closed a line a frame on line `$d0`/`$d1`), `game_over_check` (h0+$749c), and after a
death with lives left `level_init_wN` again and `level_start` (h0+$6952) frame by frame. T34's
game over is entered from here.

A working draft is on the local branch `t15-flow-wip`: `level/{exit,death,timeline}.rs`,
`Front::after_level`/`finish_frame`, the copper-interrupt branch of the VBL. On `frog-run-w1-1`'s
death (frame 655, loop end 712) it matched variables and Paula through frame 743. Measured there
(registers at frame ends): the clear loop 30.4 cycles a byte, the mask blits 4.2/6.3 cycles a word
in blitter-priority mode (holding the VBL off to line 84), frame records 337 and blit parameters
418 cycles each, the panel clear 28.8 a byte, first-screen tiles 1 383 each, its copies 37 lines a
row. Left there: the front VBL's length while a fade steps (~38 lines for 32 colours; the slide's
first step waits a frame when it ends past line `$40`), the slide's picture at line `$40`.

## Goal

A death in a level, the time up and the last life lost play as the original from the loop's end
to the level's loop again (or to `game_over_seq`), every frame.

## Interface

`LevelOutcome { Completed { tally }, Died, TimeUp, GameOver }` in `sf_core::level`, set when the
loop ends (T15's task doc named it; the draft on `t15-flow-wip` has it); `Game::level_outcome()`.

## Done when

- [ ] `frog-pit-w1-1` runs past the restart (its `unreached` exclusion gone) and a time-up case
      reaches `game_over_seq`; one in the test block.
