+++
id = "frame-interrupts-input"
title = "The frame, interrupts and input"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 70, game = 16, data = 0, shell = 14 }
reuse = "big-changes"
owns = [
  "engine/crates/sf-core/src/level/frame.rs",
  "engine/crates/sf-core/src/level/clock.rs",
  "engine/crates/sf-core/src/front/vbl.rs",
  "engine/crates/sf-core/src/input.rs",
]
+++

# The frame, interrupts and input

## 1. What it is

The original has no game loop in the modern sense. The game's work is split between two
programs that run against each other on the beam. The first is the **vertical-blank interrupt**
(`vbl_handler`, h0+$11bf8), which runs every frame at 50 Hz, plus the **copper interrupt** it asks
for (the same handler's other half, h0+$11d1a). The second is the **main program**: in a level,
`level_frame_loop` (h0+$aa6), whose **25 Hz pass** runs once the interrupt has counted two
blanks. Outside a level the main program paces itself on raster waits, one pass a frame.
[docs/08 "Main loop and interrupt"](../08-executable-map.md#main-loop-and-interrupt) has the
original's side. This chapter covers how the engine copies that order and what the order decides.

The game also has no input routine. Each piece of code reads the hardware itself: the joystick
(`JOY1DAT`, CIA-A PRA bit 7 for fire) wherever it is needed, and the keyboard through CIA-A's
serial register, polled every second blank
([docs/08 "Input"](../08-executable-map.md#input),
[docs/13 "The keyboard"](../13-front-end.md#the-keyboard-oracle)). The engine gives the game one
`FrameInput` per frame and turns it into what those registers would read.

Files owned:

- `engine/crates/sf-core/src/level/frame.rs` covers one frame of a level, in the original's order.
  `vbl` is the vertical blank and `copper_irq` the copper interrupt. `main_program` runs the main
  program on the beam for one step, including a pass left pending from the last step. `pass` and
  `pass_tail` are `level_frame_loop`'s body and its end conditions. `pass_due` is the
  `vbl_count` rule.
- `engine/crates/sf-core/src/level/clock.rs` has `level_clock_tick` (the level clock, a second
  per 50 blanks, flashing red in its last ten seconds) and `kbd_poll`, the level's CIA keyboard
  poll.
- `engine/crates/sf-core/src/front/vbl.rs` is the same handler outside a level (`logic_running`
  clear). It covers sound, the mouse pointer (`pointer`), `kbd_poll`, `display_timer`, the fades
  and the title's palette copy. It also holds the cost model that places the handler's Paula
  writes and the start of its fades on the beam (`place`, `fade_start`, module `cost`).
- `engine/crates/sf-core/src/input.rs` holds `Joy`, `KeySet` and `FrameInput`: one frame's input,
  taken by the engine and the oracle alike. It also holds the script token format
  (`format_tokens`, `parse_tokens`) that conformance scripts and bug recordings write.

A second instance of the handler, owned by [story-and-ending](story-and-ending.md), is
`engine/crates/sf-core/src/front/story/vbl.rs`. It is the story intro's own handler (`intro_vbl`,
h0+$b102), a different program's. It sets the frame's flag and count and the bitplane pointers of
the buffer to show, and steps the music stream and the fades. It reads fire or the mouse button
into `intro_skip`, then plays the music. It is timed instruction by instruction on the story's
bus simulator (`colour_writes`, `TAIL`), and split at its 8-line wait (`to_wait`, `after_wait`)
so the wait can run into the next frame. It has no keyboard poll and no main-loop counter of the
level's kind. A rewrite that unifies the handlers must keep this one's timing separate.

### One frame of a level, in order

`vbl` in `engine/crates/sf-core/src/level/frame.rs`, with `logic_running` (h0+$9d6) set:

1. `vbl_count` (h0+$9d8) is incremented, then `scroll_latch`: the camera into the scroll delays
   and the play field's bitplane pointers in the shown buffer.
2. `vbl_pass_flag` (h0+$9da) is set, then the sample stream ticks.
3. `frog_level_entry`. Then, while the frog's sprites show (or in shooter mode), the frog's 50 Hz
   work, in this order: `demo_record`, `demo_play`, `frog_tiles`, `frog_air_anim`,
   `frog_anim_tick`, `frog_walk_input`, `frog_input_anim`, `place_frog_sprites`,
   `level_clock_tick`, `frog_move`, `camera_follow`.
4. `music_tick`, `scroller`, and `scan_edges` (the edge spawner).
5. Whether running or paused: `frog_exit_walk`. When paused (`logic_running` clear, T191), the
   mouse pointer, the music, and a wait of eight lines run.
6. `music_dma_tick`, `sfx_tick`, then `kbd_poll`, then `display_timer` counted down, then the
   fades (`fades_tick`, every frame unless the title's every-other-frame toggle holds them).
7. The handler acknowledges (INTREQ `$0020`) and requests the copper interrupt (`request_copper`).
   The copper interrupt runs as the handler returns or, while the pass's blit has it masked
   (INTENA `$0010` cleared), once the pass unmasks it (`LevelCtx::custom`). It runs the
   scroller's queued blits (`copper_irq_blits`).

Each step charges its 68000 cost (`Cost::*`) first, so its writes land where the original's do
(see [cpu-time-and-beam](cpu-time-and-beam.md)).

The pass (`pass`) clears `vbl_count`, then runs `spud_update`, `obj_run_all`,
`obj_build_hitboxes`, `collide_all`, `build_bob_draw_list`, `bob_draw`, `buffer_flip`,
`tile_anim_draw`, `panel_update` and `pause_check`. After the pause check, `pass_tail` runs
`passage_step`, `extra_life` and `time_up_check`, and counts `level_state` down. The loop ends
(`logic_running` cleared) on the exit, the frog's death, or ESC (`key_down` `$45`; in the attract
demo, fire too). Each call is a numbered phase (`enter_phase`, 0–13), so a frame can end in the
middle of one.

## 2. Kind mix

`amiga` 70, `game` 16, `data` 0, `shell` 14. Judged by reading each file and weighing its lines by
the parts below; it is an estimate, not a count.

- **`amiga`.** In `frame.rs`, the interrupt order, the 50/25 Hz split, `main_program`'s frames and
  their cuts, the pending pass and the resumed vertical blank, and the copper interrupt's masking
  and its run past the frame's end. All of it is imitating how a 68000 program and its interrupts
  share a PAL frame. In `clock.rs`, `kbd_poll`. Most of `front/vbl.rs`: the Paula write placement
  and its measured `cost` table, `fade_start`, the mouse pointer's `JOY0DAT` read and its sprite,
  `kbd_poll`, and the acknowledge. In `input.rs`, the raw key codes and the joystick bits as
  ports (`Hw::of`, `joy1dat` in `engine/crates/sf-core/src/level/mod.rs` turns them into Denise's
  encoding).
- **`game`.** `level_clock_tick` and its colours. What the pass calls and when the loop ends (the
  exit, death, ESC, fire in the demo). `display_timer`, the fades' dispatch and the title's
  palette copy in `front/vbl.rs`.
- **`shell`.** In `input.rs`, the token format scripts and recordings use, and its tests: test
  tooling, not the game.
- **`data`.** None. The `cost` constants were measured on the oracle, not read out of the
  executable.

## 3. Mechanisms it uses, and where else this one is used

This chapter documents the [glossary](glossary.md)'s **vertical-blank and copper interrupts**,
**25 Hz main-loop pass** and **CIA keyboard**. It leans on:

- **Cycle charging and beam position** ([cpu-time-and-beam](cpu-time-and-beam.md)). Every
  handler step charges a `Cost`. `main_program` ends frames by the beam clock or by
  `Pacing::Late` cuts. `wait_lines(8)` is the paused handler's wait.
- **Byte-exact state** ([memory-model](memory-model.md)). `logic_running`, `vbl_count`,
  `vbl_pass_flag`, `clock_ticks`, `display_timer`, `kbd_toggle`, `key_down`, `key_up` and `mouse`
  are original addresses in `LAYOUT`, compared pass by pass (none is a `presentation` variable).
- **Paula writes** ([sound-chip](sound-chip.md)). The handler's INTENA and INTREQ writes go out in
  order with the sound's (`LevelCtx::custom`, `wr`), because the comparison and Paula read them
  together.

Where else the mechanism appears:

- `engine/crates/sf-core/src/level/mod.rs` handles the frame boundary and interrupt plumbing.
  `LevelCtx::frame_end` runs `frame::vbl` at each boundary (and again for a blank held behind
  the handler, `vbl_held`). `irq_frame_end` is the copper interrupt past the end. `custom` and
  `request_copper` are the copper mask.
- `engine/crates/sf-core/src/level/beam.rs` holds `Frames`, `Pending` and `Cut`: the state of a
  pass that runs past a blank.
- `engine/crates/sf-core/src/front/mod.rs` runs the front end's handler (`vbl::run`, `vbl::end`,
  `vbl::load_blank` on frames spent loading). Its `Keyboard` is the front end's copy of the CIA
  register.
- `engine/crates/sf-core/src/front/gameover.rs` runs a level's `frame::vbl` while game over plays
  over the level (`in_level_from`).
- `engine/crates/sf-core/src/game.rs` turns `FrameInput` into the keyboard byte (`step_body`,
  `key_byte`) and hands the keyboard over between front end and level. Its `PassClock` counts
  passes by the same `vbl_count` rule for the faithful comparison.
- `engine/crates/sf-core/src/level/pass.rs` has the pause (`pause_check`, `pause_run`, T191),
  which clears `logic_running` so the handler takes its non-level path, and `buffer_flip`, which
  clears `vbl_pass_flag`.
- `engine/crates/sf-core/src/front/story/vbl.rs` is the story's handler (above).

### Where the 50 Hz interrupt and the 25 Hz pass disagree

These are the places where the two programs' timing shows in the game. A native loop that ran
"update everything, then draw" once a frame would lose each one.

- **The frog, camera and clock move every frame; enemies, items and their drawing every second
  frame.** The frog is in the handler, the objects in the pass. On the original, the frog's x
  changes on consecutive frames, and no object moves on two consecutive frames while the pass is
  on time.
- **A late pass sees the frog move under it.** When a pass runs past a blank, the handler's
  frog, camera and scroll work runs in the middle of it. The pass's later calls read the frog
  and the camera as the handler left them. The engine copies this only where conformance feeds
  the original's schedule (`Pacing::Late`). Normal play never lags; the owner accepted that on
  2026-10-15 ([docs/08 "Lag"](../08-executable-map.md#lag-oracle)). A pass that comes due at the
  frame's end waits for the next frame (`waits_out`, `Pacing::PassDue`).
- **Input sampled per frame, acted on in either program.** `Hw` (joystick, fire, the keyboard
  byte) holds still within a frame and is replaced at each blank (`frame_end_from` sets
  `self.hw = spec.hw`). The frog's handler routines read the joystick every frame. The spud reads
  fire in the pass (`spud_update`), so it sees only the frames a pass runs in. A late pass's calls
  after the blank read the new frame's buttons.
- **The keyboard is polled every second blank and acted on in the pass.** `kbd_poll` alternates
  on `kbd_toggle` and writes `key_down` / `key_up`. The pass's `pause_check` (P, `$19`) and
  `pass_tail` (ESC, `$45`) read `key_down` as the last poll left it. Which blank the poll falls on
  carries across scene changes: the front end after a level must not re-read the ESC the level's
  poll handled (`Front::after_level`, measured on `frog-esc-w1-1`).
- **The buffer the beam shows is decided by the handler.** `buffer_flip` in the pass writes
  `shown_buffer` in steps, and a frame may end between them. The next blank's `scroll_latch`
  latches whichever buffer is shown by then. `vbl_pass_flag` is set by the handler and cleared
  by the flip.
- **The copper interrupt can run past the frame's end.** Its blits may still be running when the
  next blank is due. The next frame's handler is then taken straight after it, and the
  interrupt's DMACON `$0400` and INTREQ `$10` belong to the next frame (T197, `irq_frame_end`).
  Outside a pass the engine keeps them in the old frame, because no pass runs again to make them.
- **Outside a level, the music decides when the fades start.** After `music_tick` the handler
  waits until eight lines past the line it reads (h0+$12a14). The fades therefore start on a line
  boundary set by how long the music took (`fade_start`, T100), and the colours the copper reads
  mid-fade depend on it. The DMA stage after the wait restarts a note the replayer turned off
  (`place`, T213).

## 4. What depends on it

- **Every subsystem chapter.** [level](level.md), [frog-and-items](frog-and-items.md),
  [enemies-and-bosses](enemies-and-bosses.md), [slot-and-bonus](slot-and-bonus.md),
  [game-flow](game-flow.md) and [front-end](front-end.md) are all called from the handler or the
  pass, and their behaviour is defined at the rate they are called.
- **[sound-driver](sound-driver.md).** Its ticks are called from the handler, and its writes are
  placed by the handler's cost model outside a level.
- **[display-hardware](display-hardware.md).** It relies on `scroll_latch` and the copper
  interrupt's blits happening where they do on the beam.
- **[verification](verification.md).** The faithful state check compares at pass ends counted by
  `PassClock`, the same rule on both sides. `Pacing::Late` cuts end frames inside a pass by
  `phase` (the numbered pass calls here). Every conformance script and recorded bug is a list of
  `FrameInput` in this chapter's token format.
- **[app](app.md).** It builds one `FrameInput` per PAL frame from the keyboard and pads.
  Its bug recorder writes the token format.

## 5. What a native rewrite must preserve

Faithful stays the bar. Each row is behaviour the original shows, and the check that holds the
engine to it today.

| Behaviour to keep | Check that proves it |
|---|---|
| The frog, camera, scroll and clock step every frame, the objects every second frame. | On the original: `the_frog_moves_every_frame_and_objects_every_other` in `engine/crates/sf-oracle/tests/exe_map_claims.rs`. On the engine: the faithful checks (state and picture) of `frog-run-w1-1` (`engine/conform/cases/frog-run-w1-1.toml`) and every level case. |
| A pass begins once `vbl_count` reaches 2 and clears it; a pass due at a frame's end on time waits for the next frame. | `the_pass_clock_counts_passes_ended` (`engine/crates/sf-core/src/game.rs`); `state_is_matched_by_pass_count` (`engine/crates/sf-conform/tests/faithful.rs`); `enemies_w6_4_alone_waits_out_a_pass_due_at_the_frames_end` (`engine/crates/sf-conform/tests/enemies_w6_cases.rs`). |
| A late pass resumes after the blank, with the handler's work run in the middle of it. | `frog_run_w1_1` (`engine/crates/sf-conform/tests/frog_cases.rs`: its case is late on 320 frames, per `the_originals_late_frames_are_counted` there, and green); `a_late_cut_the_engine_cannot_reach_ends_the_frame_as_normal_play_does` (`engine/crates/sf-conform/tests/alone.rs`); the completion cases under `engine/conform/cases/` fed `Pacing::Late`. Normal play never lags, by owner decision (above). |
| The copper interrupt runs the scroller's blits; masked by the pass's blit, it waits for the unmask; run past the frame's end, its tail is the next frame's. | `copper_irq_blits_match_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`); `complete_w4_3` and `coper_tail_outside_a_pass_kept` (`engine/crates/sf-conform/tests/frog_cases.rs`). |
| `kbd_poll` every second blank: the register's byte, `!(code << 1 \| released)`, into `key_down` / `key_up`, with `$ff` written back once a new byte is handled, so a held key reads as down for one poll. | `kbd_poll_matches_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`); `the_keyboard_sends_a_byte_per_change_and_reads_ff_once_handled` (`engine/crates/sf-core/src/front/mod.rs`). |
| The poll's phase and the handled byte carry across the level/front-end boundary (ESC read once). | `frog_esc_w1_1` (`engine/crates/sf-conform/tests/frog_cases.rs`, case `engine/conform/cases/frog-esc-w1-1.toml`); `game_over_esc_w1_1` (`engine/crates/sf-conform/tests/cases.rs`). |
| P pauses at the pass's pause check, and P or fire resumes it where it stopped, with the handler's paused path running meanwhile. | `p_pauses_and_p_resumes_as_the_original_does`, `fire_ends_the_pause_as_the_original_does` (`engine/crates/sf-conform/tests/chain.rs`). |
| ESC, the exit, death, and fire in the attract demo end the loop at the pass's end. | `frog_esc_w1_1`; `fire_ends_the_attract_demo_as_the_original_does` (`engine/crates/sf-conform/tests/front.rs`); the completion and game-over cases. |
| The level clock: a second off every 50 blanks while the frog's work runs, not in the bonus level, red for the first half of each of its last ten seconds. | `level_clock_tick_matches_the_original` (`engine/crates/sf-conform/tests/level_routines.rs`); `frog-timeup-w1-1` (`engine/conform/cases/frog-timeup-w1-1.toml`). |
| Joystick directions as Denise encodes `JOY1DAT`; fire as CIA-A PRA bit 7 low. | `joy1dat_encodes_directions_as_denise_does` (`engine/crates/sf-core/src/level/mod.rs`); for fire, no dedicated test, only every case that holds fire. |
| Outside a level, the fades start eight lines after the line `music_tick` ends on, and the handler's Paula writes fall where the original's do. | `the_fades_start_eight_lines_after_the_music`, `the_keyboard_and_a_row_come_after_the_wait`, `the_dma_stage_is_placed_after_the_wait`, `a_note_retriggered_mid_sample_restarts` (`engine/crates/sf-core/src/front/vbl.rs`); case `frog-pit-w1-1`, whose exit fades T100 measured. |
| `display_timer` counts down in the handler on every frame, loads included, timing the progress map and the world pictures. | No dedicated test. The faithful state check of the cases through those screens (`game-whole`, `engine/conform/cases/game-whole.toml`) compares it. |
| The mouse pointer: `JOY0DAT` read and the pointer clamped to its box. | No check moves the mouse: `JOY0DAT` never changes on the oracle because no script moves it, so only the clamps are covered (state check on `mouse`). Mouse movement is unverified. |
| The story handler sets `intro_skip` on fire or the port-1 button, and its colour writes' timing. | `story-skip-*` cases under `engine/conform/cases/`, run by `engine/crates/sf-conform/tests/story.rs` (owned by [story-and-ending](story-and-ending.md)). |

## 6. Native sketch (non-binding)

This sketch is non-binding: designing the rewrite is the later plan's job.

A native shape keeps two schedulers, not one loop. One is a `per_frame` step (the handler's
work, in the listed order). The other is a `per_pass` step (the pass), run when a counter says two
frames have gone by. Both sit behind an explicit frame boundary, not hardware interrupts. The
beam-dependent parts (where a pass is cut, where the copper interrupt's tail falls, when the fades
start) would become an explicit "where in the frame" value passed to the steps. They would no
longer come from charging cycles. Conformance's `Pacing::Late` would keep its role as an input.
The keyboard could become an event queue with an adapter that reproduces the CIA's behaviour:
the last byte wins, a poll every second frame, and `$ff` once handled. That adapter is what the
pause and ESC behaviour depend on. The two `kbd_poll` copies (`level/clock.rs` and
`front/vbl.rs`, with their state in `State::kbd_flag` / `kbd_acked` and `Keyboard`) could be one.
Whether the frame-boundary cut points can be dropped from normal play is unknown. They are never
reached there, but the conformance runs need them, so the decision belongs with
[verification](verification.md)'s address coupling.

## 7. Reuse

Rating: **`big-changes`**. 1 098 lines (`python tools/architecture.py totals`).

The skeleton carries to another Amiga game: a vertical-blank handler every frame, a main loop
paced on a counter or a raster wait, a copper interrupt, frames cut mid-pass by the beam, a
per-frame `Hw` snapshot, and a polled CIA keyboard. The handler's contents are Superfrog's: which
routines run at 50 Hz and in what order, the pass's thirteen calls, the end conditions, the clock,
and the measured `cost` table (re-measured per game on its oracle). Another game may also use the
keyboard interrupt instead of polling it, or run its logic at 50 Hz. `input.rs` carries as-is,
since joystick bits and raw key codes are the Amiga's and the token format is generic. The
handler files would be rewritten around the same structure. Another Team 17 game of the period
may share the handler's shape and the 8-line wait after the music (Player 4.1A is common to
several). This is unchecked.

## 8. Sources

- [docs/08 "Main loop and interrupt", "Lag", "Input"](../08-executable-map.md)
- [docs/13 "Timing the engine keeps", "The keyboard"](../13-front-end.md)
- [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.1 (faithful, normal play never lags)
- Code: `engine/crates/sf-core/src/level/frame.rs`, `engine/crates/sf-core/src/level/clock.rs`,
  `engine/crates/sf-core/src/front/vbl.rs`, `engine/crates/sf-core/src/input.rs`,
  `engine/crates/sf-core/src/level/mod.rs` (`Hw`, `joy1dat`, `LevelCtx::frame_end`,
  `LevelCtx::custom`), `engine/crates/sf-core/src/game.rs` (`step_body`, `key_byte`,
  `PassClock`), `engine/crates/sf-core/src/front/mod.rs` (`Keyboard`, `Front::after_level`),
  `engine/crates/sf-core/src/level/pass.rs` (`pause_check`, `pause_run`, `buffer_flip`),
  `engine/crates/sf-core/src/front/story/vbl.rs`.
