+++
id = "slot-and-bonus"
title = "The slot machine and the bonus level"
tier = "subsystem"
depth = "deep"
kind = "game"
mix = { amiga = 21, game = 68, data = 11, shell = 0 }
reuse = "superfrog-only"
owns = [
  "engine/crates/sf-core/src/slot/",
  "engine/crates/sf-core/src/bonus/",
]
+++

# The slot machine and the bonus level

## 1. What it is

Two mini-games sit beside the levels. **The slot machine** is what the tally offers after a level
(up, then fire) for the coins the level's clock left: a fruit machine with three reels, a hold
menu, a prize ladder with a gamble, a hi-lo game and a jackpot that prints the next level's code.
The original's `slot_machine` (h0+$49e2) is the whole of it and returns to the tally.
[docs/13 "The slot machine"](../13-front-end.md#the-slot-machine-t71t74-code-oracle) has the
original's side: the round loop, the lines, the ladder, the gamble, the hi-lo game and the cases.
**The bonus level** is world 7 (the `LB*` files, between W5 and W6), played in shooter mode: the
camera creeps right on its own, the frog flies and fires shots instead of throwing the spud, and
shot enemies leave rewards.

The two are built on different machinery. The slot machine is a front-end routine: it runs on the
front end's routine stack (`front::Cx`, `Routine`, `Yield`, `engine/crates/sf-core/src/front/mod.rs`),
and its reels and lamps are blitted on the beam, so a third of its code is about when each blit
lands. The bonus level is a level: the level's main loop, object dispatch, camera and collision
run it, and `bonus/` only adds the object kinds shooter mode has and one change to the loop. It
has no timing of its own.

What each reuses:

- **The slot machine from `level`**: almost nothing. It is entered and left by the tally
  (`engine/crates/sf-core/src/level/tally.rs`: `Routine::Slot` is called from the tally's steps, and
  `after_slot` follows it), and `rounds::sound_back` puts the level's sound files back with
  `audio::files_for`. Everything else comes from the front end: `front::common` (`fade`,
  `MusicStart`, `frames`, `call`), the fade records (`front::fade::RECORDS`), the pointer's sprite
  (`front::sprite::attach`), the Accessing-disk overlay (`front::title::overlay_show`), the
  blitter's memory effect (`front::blit::Blit`), `front::attract::number_text`, `ending::work`
  (`CpuWork`), and the story's bus simulator (`front::story::sim`, `front::story::cpu`).
- **The bonus level from `level`**: nearly all of it. The dispatch (`level::register`,
  `register_routines`, `Dispatch::register_marker`, `ObjectKind`, `level::call`), the object
  routines (`obj_spawn_cell`, `obj_spawn_at_xy`, `obj_spawn_at_xy_nodedup`, `bob_anim`,
  `bob_follow`, `obj_free`, `obj_free_if_left_behind`, `obj_despawn_if_offscreen`, `obj_path_xy`,
  `overlapping`, all in `engine/crates/sf-core/src/level/objects.rs`), `LevelCtx::rng_range` and
  `LevelCtx::sfx`, the panel (`level::panel`) and the level start (`level::load::frog_first_step`,
  `drop_in`). Shooter mode itself is spread through code other chapters own: the camera's creep
  (`shooter` in `engine/crates/sf-core/src/level/camera.rs`), the column scroll (`shooter_scroll`
  in `engine/crates/sf-core/src/level/scroll.rs`), `level_init_lb` and its call to
  `shots::spawn_shots` (`engine/crates/sf-core/src/level/load.rs`), the shots' collision hook in
  `collide_all` (`engine/crates/sf-core/src/level/objects.rs`), the frog's flying
  (`shooter_input` in `engine/crates/sf-core/src/frog/physics.rs`), `spud_update`'s branch to
  `shots::shooter_spud` (`engine/crates/sf-core/src/items/powerups.rs`) and the loop's branch to
  `frame::back_on_screen` (`engine/crates/sf-core/src/level/frame.rs`). Of shooter mode, `bonus/`
  holds only what would not exist without it.

Files owned:

- `engine/crates/sf-core/src/slot/mod.rs` (536 lines): `Routine`, `slot_machine` from after its
  first raster wait: the overlay and the loads at once (`load`: `FRUT`, `FRUC`, `FRUM` into the two
  display buffers, from disk 1's or disk 2's copies by `wanted_disk`; the music faded and stopped;
  `SOUND_KEPT_LEN` bytes of the sound's chip RAM kept over the level's map at `SOUND_KEPT_AT`; the
  slot's `LFFX` and `LFMU` loaded), the music's start, `slot_count_win` at the start, `set_up`
  (`slot_init`, `slot_digits`, the pointer, the six plane MOVEs, the pointer's sprite), the fade in,
  the attract loop until fire or the mouse button, then the rounds; at their end `SlotOutcome`
  (`Award { points, lives }`, the score and lives added). `SlotOver` returns to the tally
  (`level::tally::after_slot`), `resume` rebuilds the stack from the original's call chain. Also
  `draw_lamps` (the lamps blitted with the screen blank), `count_win_lamps`, `count_text`,
  `print_text` (the text printer h0+$6f3a, returning its cycles), `slot_digits` (two-digit counts
  copied a byte at a time into six planes), the measured CPU costs (`LAMPS_OFF_CYCLES`,
  `DIGITS_CYCLES`, `CHAR_LOOK_CYCLES`, `GLYPH_CYCLES`, `P41_INIT_LINES`), `FRAME_LINES` (312) and
  `Handler` with `interrupt_lines`, the slot screen's vertical-blank handler length fitted to the
  oracle (T94).
- `engine/crates/sf-core/src/slot/draw.rs` (636 lines): `Draw`, the reels and lamps drawn on the
  beam (`Part::Attract`, `Part::Lamps`, `Part::Reels`): one blit a step, its memory at once, the CPU
  time between steps spent on the beam (`spend`, `done`: a line the display fetches on loses 1.4
  cycles a word from the CPU, 1.9 from the blitter), a blit due on line 312 or later cut by the
  frame's interrupt and resumed after it (`cut`, `REEL_RESUME`, `LAMP_RESUME`). Lamps drawn after
  the picture are timed on the story's bus simulator instead (`plan`, `LampBus`, `lamps_prog`: the
  68000's bus accesses for `slot_draw_lamps`, `IRQ_SEEN`). `Work` is CPU work on the beam (the
  digits' byte copies). `reel_blit` and `lamp_blit` build each blit's registers. `attract_row` is
  the attract pattern (a row three frames, the later pattern from the 10th pass, the count reset
  at the 30th).
- `engine/crates/sf-core/src/slot/rounds.rs` (1 380 lines): the rounds as front-end routines.
  `Rounds` (h0+$4d4a and the end: 100 frames or fire, fade out, the score taken, the music faded,
  `sound_back` and the level's music restarted), `HoldMenu` and `hold_lists`, `Menu` (the pointer
  between its stops on the joystick, the flashing and lit lists, a pick by fire or the mouse
  button), `Spin` (the reels' counters, the three-bars spin-on, `reel_stop`), `payout`, `Flash`
  and `Lights` (the seven flash patterns), `Award` (the ladder, the gamble, the hi-lo game, the
  bust), `award_rung`, `ladder_rung`, `rung`, `collect`, `HiloSpin`, `Jackpot` and
  `password_for_level`, `Count` (`slot_count_win`: 1 000 a frame, at once from 160 000). `sfx`
  places `sfx_play`'s Paula writes on the line (`SFX_WAIT`, `SFX_GAP`). `rng` is `rng_range` on
  `State`.
- `engine/crates/sf-core/src/bonus/mod.rs` (20 lines): `register_all`, the bonus level's kinds
  into the level's dispatch (templates 110–133, the markers of row `$600`, the death routines).
- `engine/crates/sf-core/src/bonus/shots.rs` (197 lines): the frog's five shots (`spawn_shots`
  with the original's y slip, `shooter_spud`, `shot_update`, `shots_faster`, `shot_touches`).
- `engine/crates/sf-core/src/bonus/enemies.rs` (833 lines): the enemies of markers 1–15
  (templates 113–133) and marker 55, the level's end (`level_end`, `level_state` 2), with their
  paths read by address from hunk 0 (`PATH_113`, `PATH_WAVE`, `PATH_CHAIN`, `BURST_SPEEDS`, ...).
- `engine/crates/sf-core/src/bonus/death.rs` (234 lines): `touch_bonus_enemy`, `death_fall`,
  `become_reward` (shared with `touch_spud_damage` in worlds 1–6, through
  `engine/crates/sf-core/src/enemies/common.rs`), `reward_float`, `reward_arc`, `touch_reward`,
  and the two reward counters.
- `engine/crates/sf-core/src/bonus/frame.rs` (40 lines): `goes_on`, `back_on_screen` and
  `bonus_frame`: after a death with a life left `top_loop` does not set the level up again but
  puts the frog back where the camera is.

## 2. Kind mix

`amiga` 21, `game` 68, `data` 11, `shell` 0. Judged by reading each file and splitting its lines
(tests included) between the parts below, then weighting by file size (`draw.rs` about 70% `amiga`,
`mod.rs` about 35%, `rounds.rs` about 12%, `bonus/` none). It is an estimate, not a count.

- **`amiga`.** In `draw.rs`, everything but `attract_row` and its tests: the beam clock, the fetch
  losses, the cut at the interrupt, `plan` and `lamps_prog` on the bus simulator, `write_at`,
  `Work`, the blits as register sets. In `mod.rs`, `Handler` and `interrupt_lines` with their test,
  `draw_lamps`, the plane MOVEs written into the copper list, `FRAME_LINES`, and the sound's chip
  RAM kept over the level's map in `load`. In `rounds.rs`, `sfx`'s placing of the Paula writes on
  the line, the `busy` charges, the leads passed to each draw, the digits' `Job`s and the chip-RAM
  copy back in `sound_back`.
- **`game`.** The rest of `rounds.rs` (every rule of a round), the flow of `Routine`, `slot_init`,
  `slot_digits`, `count_text`, `print_text` and `count_win_lamps` in `mod.rs`, `attract_row`, and
  all of `bonus/` but its tables: each enemy's movement, shots, damage, rewards and the loop's
  change after a death.
- **`data`.** The measured cycle and line constants (in `mod.rs`, `draw.rs` and `rounds.rs`: about
  forty, from T72, T73, T93, T94 and T129); the addresses the bonus enemies read their paths, speeds
  and animations from in hunk 0, and the routines' addresses the dispatch names them by.

Every variable both games use is at its original address in `State` (`slot_state` h0+$49c6, the
lamp records at h0+$6842, the shots at h0+$80dc, the reward counters and toggles inside the code).
That is the memory-model coupling, left uncounted here where it is only a label, as in the other
subsystem chapters.

## 3. Mechanisms it uses

- **Byte-exact state at original addresses** ([memory-model](memory-model.md)). Both games keep
  every variable where the original does, and the code reads lists the way the original's code
  does, overruns included: the gamble's bust test reads ten records of the flashing list even past
  its -1 (`Award::run`, step 2). Several bonus variables are words inside the original's code
  (`TOGGLE_113`, `TOGGLE_114`, `FLY_OFF`, `REWARD_LIFE_COUNT`, `REWARD_POWER_COUNT`, `bonus_visit`):
  `layout::LAYOUT` names them so conformance compares them.
- **The loaded hunk image** ([copied-in-data](copied-in-data.md)). The bonus enemies' paths, burst
  speeds, animation frames and the reward's wave and arc are read by address from hunk 0 as loaded
  (`c.bus.w(PATH_...)`). The slot machine reads the lamp rectangles (h0+$670a), the reel symbol
  positions (h0+$668a), the attract patterns (h0+$3b2e8, h0+$3b1ae), the prize ladder (h0+$57b6),
  the font and character table, the stops' x and `password_table` the same way. `sf_assets::Tables`
  decodes the attract patterns too (`slot_attract_first`, `slot_attract_later`), but the engine does
  not read them from there (FINDINGS T03).
- **Cycle charging and the beam** ([cpu-time-and-beam](cpu-time-and-beam.md)). The slot machine's
  draw is the fourth beam model in the engine, beside the level's `Clock`, the front end's `Cx` and
  the story's. It keeps CPU cycles from the frame's start and spends them line by line, slower on
  displayed lines (`draw::spend`). Outside the draw, its work is charged with `Cx::busy` and, for
  the sound copy, with `ending::work` (`CpuWork`, owned by [story-and-ending](story-and-ending.md)).
- **The story's bus simulator** ([story-and-ending](story-and-ending.md)). Lamps drawn after the
  picture run as a 68000 program of bus accesses (`lamps_prog`) on `sim::Sim`, against the frame's
  fixed DMA (`LampBus`: refresh, the copper's cycles from `copper::run_bus`, the bitplanes' fetch,
  the audio slots), to find which lamps start before the CPU takes the frame's interrupt.
- **The display hardware** ([display-hardware](display-hardware.md)). The reels and lamps are
  blits (`front::blit::Blit`), on six planes, through the slot's own copper list (`SLOT_COPPER`,
  h2+$65e4); the plane pointers are written into its MOVEs. The front end renders the picture part
  by part from each blit's start line (`cx.reveal`), and a lamp written within
  `game::RACE_LINES` of the beam is a beam race (`cx.races`, `FrameOut::beam_races`). The front
  end's sprite DMA follows the list's DMACON writes on the slot screen (`follows_dma` in
  `front/mod.rs`), which hides the pointer.
- **The vertical-blank interrupt** ([frame-interrupts-input](frame-interrupts-input.md)). The slot
  screen's handler is the front end's (`front::vbl`), but its length is the slot's own fit
  (`interrupt_lines`, read in `Front::step`), and a draw cut by the frame's end resumes after it.
  The rounds read fire and the mouse button (`Cx::fire`, `Cx::mouse_button`) and the joystick
  (`Cx::joy1dat`). The bonus level runs under the level's handler.
- **Paula writes** ([sound-chip](sound-chip.md), [sound-driver](sound-driver.md)). The slot
  machine's frames are 312 lines for the audio DMA (`FRAME_LINES`, passed to `AudioDma::frame` in
  `Front::step`). `rounds::sfx` places each `sfx_play`'s writes on the line with the original's
  raster wait between the channel's off and on, so Paula restarts the effect. Under conformance the
  Paula writes are anchored to the original's (`Game::anchor_writes`, `cx.schedule`) to time the
  audio slots and the lamps' start.
- **The RNG** ([game-flow](game-flow.md) owns `engine/crates/sf-core/src/rng.rs`). Not an Amiga
  imitation, but its state (`rng_state`, h0+$160d4) is compared every frame, so every draw has to
  come in the original's order and number, and only where the original draws (`rng(n)` is
  `rng_range`, 0 to `n`). The slot machine draws in: the hold menu (`rng(8)`, only after a round
  that neither won nor paid; 6 allows holds), the spin (`rng($64)` for the three-bars spin-on,
  then three `rng($40)` for the reels' counters), the free spin (`rng($64) < 6`, only after a win
  with coins left), the gamble (`rng($64) < $32`) and the hi-lo spin's length (`rng($32) + $32`
  frames). The bonus level draws in `dart_init`, `burster_init`, `bomb_init`, `swooper_init`,
  `glider_init`, `tracer_init` and `become_reward`, which draws `rng_range(2)` in shooter mode too
  and then discards it. W1-1 never moves the RNG, so the slot cases start from
  `start-w1-1-late` or `start-w1-1-blink1` to reach other draws (docs/13).

## 4. What depends on it

- **[game-flow](game-flow.md).** `Game::enter_slot`, `Game::slot_outcome` and `Scene::Slot
  { coins_in }` (`engine/crates/sf-core/src/game.rs`, `engine/crates/sf-core/src/scene.rs`) expose
  the slot machine; `SceneKind::BonusLevel` names the bonus level. `front::chain` runs the bonus
  level's start and its `bonus_frame` (`exit::with_level`) and sets `REWARD_POWER_COUNT` to 9 on
  entry.
- **[front-end](front-end.md).** `front::Routine` holds the thirteen `Slot*` routines and runs
  them; `Front::step` times the slot screen's handler with `slot::Handler` and
  `slot::interrupt_lines`, its fades on `SLOT_COPPER`, its sprite DMA and its audio DMA on
  `slot::FRAME_LINES`; the front end's `resume` calls `slot::resume`.
- **[level](level.md).** The tally enters the slot machine and runs `after_slot` when it returns.
  `level::Dispatch` calls `bonus::register_all`; `level_init_lb` calls `shots::spawn_shots`;
  `collide_all` calls `shots::shot_touches`; the frame loop calls `frame::goes_on` and
  `back_on_screen`.
- **[frog-and-items](frog-and-items.md), [enemies-and-bosses](enemies-and-bosses.md).**
  `spud_update` hands over to `shots::shooter_spud` in shooter mode; `enemies::common` uses
  `death::become_reward` for every spud-killed enemy in worlds 1–6.
- **[app](app.md).** The slot machine and the bonus level are played in the app; the recorder
  names the slot machine as a screen (`engine/crates/sf-e2e/tests/recorder_app.rs`).
- **[verification](verification.md).** The slot and bonus cases, the routine-oracle tests
  `engine/crates/sf-conform/tests/slot_routines.rs`, `engine/crates/sf-conform/tests/bonus_routines.rs`
  and `engine/crates/sf-conform/tests/bonus_enemy_routines.rs`, the sync point `slot-none`, and the
  examples `engine/crates/sf-conform/examples/slot_lamps_sim.rs`,
  `engine/crates/sf-conform/examples/slot_plan.rs` and `engine/crates/sf-conform/examples/plan_bonus.rs`
  call into `slot` and `bonus`.

### What the checks compare

- **The slot cases** (`engine/conform/cases/slot-*.toml`, sync point `slot-none`, reached by
  forcing W1-1's exit and the tally's choice) compare `LAYOUT` every frame, which holds every slot
  variable (`slot_state`, `slot_count`, `slot_prize`, `slot_win`, `slot_hilo`, `slot_spin`,
  `slot_menu`, `slot_attract`, `slot_lamps`, `slot_palette`, `text_plane`) and `rng_state`; the
  picture under check 2's tolerance (rust-engine DESIGN §2.1), with the beam races excused; and the
  Paula writes. The digits' and the printer's writes are compared once the original has left them
  (`Job`). The tests in `engine/crates/sf-conform/tests/front.rs` then assert the draws' DMACON
  `$8400` and INTENA `$0010`/`$8010` lines.
- **The bonus cases** (`bonus-enter-wb-1`, `bonus-death-wb-1`, `bonus-end-wb-1`,
  `chain-w5-4-to-bonus`, `chain-bonus-to-w6-1`) compare the level's state, picture and sound as
  every level case does, with no re-sync. The routine-oracle tests run each bonus routine on Moira
  and the port from the same memory over generated states and compare every byte written, the
  sound driver's state and the Paula writes.

## 5. What a native rewrite must preserve

Faithful stays the bar. Each row is behaviour the original shows, and the check that holds the
engine to it today. Tests named without a file are in `engine/crates/sf-conform/tests/front.rs`;
`case!` entries are in `engine/crates/sf-conform/tests/cases.rs`.

| Behaviour to keep | Check that proves it |
|---|---|
| The loads at once, with the level's music faded and stopped inside them, the slot's sound loaded, the level's sound kept over its map; the scene `Slot { coins_in }` from the first frame. | `the_slot_machine_loads_as_the_original_does` (case `slot-load`); `the_music_stopped_inside_the_slot_machines_load_is_silent_after_it` (`engine/crates/sf-conform/tests/channels.rs`). |
| The attract loop: the reels at 0, 9, `$18` and 18 reel blits from the raster wait on line 299/300, seven before the interrupt and the rest after it, then the pattern's row and 26 lamps, every DMACON `$8400` and INTENA `$0010`/`$8010` on the original's line. | `the_slot_machine_attract_loop_draws_as_the_original_does` (case `slot-attract`: at least 2 500 of 3 120 on the line, at most 4 frames 2 lines off). |
| A line the display fetches on slows the CPU by 1.4 cycles a word and the blitter by 1.9; blits run at two cycles a word a channel. | `a_displayed_line_slows_the_cpu_and_more_the_blitter` (`engine/crates/sf-core/src/slot/draw.rs`); the line assertions of the attract and rounds tests. |
| The slot screen's handler length: 17.6 lines, plus the keyboard poll, each DMACON write, a music row read and each effect, less a quiet tick; a draw resumes where it ends. | `the_handler_acknowledges_on_the_originals_line` (`engine/crates/sf-core/src/slot/mod.rs`); the attract test above. The effect's two lines (T72) are not seen in the attract loop. |
| Lamps drawn after the picture: which start before the CPU takes the frame's interrupt, on the bus with the audio DMA's slots, the interrupt seen by a poll from line 0 cycle 0. | `the_jackpot_flash_cuts_its_lamps_where_the_original_does` (cases `slot-hold-jackpot-40`, `slot-hold-jackpot-50`). Conformance anchors the Paula writes to the original's; normal play uses the engine's lines, and 2–4 of about 70 flashes cut a lamp differently, which no check covers (rust-engine FINDINGS, T129). |
| Lamps racing the beam: a lamp written within 4 lines of the beam is excused, no wider than measured. | `the_slot_machine_attract_loop_draws_as_the_original_does` (at most 45 frames in 300 excused). |
| The rounds' draws on the beam (the menu's lamps, the spin's reels, the digits' byte copies through the interrupt), within a line of the original's. | `the_slot_machine_plays_its_rounds_as_the_original_does` (case `slot-fire`: at least 3 600 of 4 576 writes within a line). |
| A paying line's two `sfx_play`s run past the frame's end, so its flash starts in the next frame. | Case `slot-hold-20000` (`case!(slot_hold_20000, ...)`). |
| `sfx_play`'s channel off, the raster wait, then on: Paula restarts the effect. | `the_slot_machines_effects_restart_as_on_the_original` (`engine/crates/sf-conform/tests/channels.rs`, case `slot-gamble`). |
| The payout: the six lines in the original's order, the last match winning, each line's lamp and sounds. | `every_reel_state_pays_as_the_original_pays` (`engine/crates/sf-conform/tests/slot_routines.rs`, every reel state). |
| A reel stops on 0, 9 or `$18`, its hold set and its sound on channel 3. | `every_reel_stops_as_the_original_stops_it` (same file). |
| The ladder: each rung's win, life or lamp, its scan to the bust, the jackpot or the gamble's offer, the collect. | `every_prize_is_awarded_as_the_original_awards_it`, `the_ladder_is_scanned_as_the_original_scans_it`, `every_rung_is_collected_as_the_original_collects_it`, `every_rung_a_line_pays_is_collected_as_the_original_collects_it` (same file). |
| The gamble: half the time the rung above; else three below, or the bust when the flashing list's ten records (read past its -1) hold the bust lamp. | Case `slot-gamble` (won, then lost to the bust). A gamble lost above the bust is reached by no case (docs/13). |
| The hi-lo game: eight turns from rung 8, the number turning `rng(50) + 50` frames never stopping where it began, right climbing, wrong busting, take collecting. | Cases `slot-hilo-1`..`8`, `slot-hilo-wrong`; `the_slot_machine_pays_its_jackpot_as_the_original_does` (case `slot-hilo-0`, 155 000 given). |
| The jackpot: each coin left won as 5 000 and counted, 150 000, the next level's code printed and flashed until fire. | `the_slot_machine_pays_its_jackpot_as_the_original_does`; case `slot-hold-jackpot`. After a level 5 or in world 7 the engine stops as `unported` (`password_for_level`); no case reaches it. |
| The count: 1 000 a frame into the score shown, at once from 160 000; the score taken at the end. | Every slot case (`slot_count`, `score`). A win of 160 000 or more at the slot machine's start is `unported` in `count_win_lamps`, reached by no case. |
| The free spin after a win with coins left (`rng(100) < 6`). | No check: no case reaches it at these seeds (docs/13). |
| The RNG drawn in the original's order and number in every round. | `rng_state` in every slot case. |
| The end: 100 frames or fire, the fade out, the music faded, the level's sound copied back as CPU work, the level's music restarted; the outcome the score and lives added. | `the_slot_machine_plays_its_rounds_as_the_original_does` (to the frame before the return, then `SlotOutcome::default()`); `a_slot_machine_entered_from_a_level_returns_its_outcome`. No case compares the level's sound after the slot machine has returned. |
| Pausing during a spin holds the reels and the sound. | `pause_during_a_slot_spin_holds_the_reels_and_the_sound_then_spins_on` (`engine/crates/sf-e2e/tests/drill.rs`). |
| Widescreen on the slot screen shows the border at the sides. | `widescreen_on_the_title_story_intro_and_slot_machine_shows_the_border_at_the_sides` (`engine/crates/sf-e2e/tests/view.rs`). |
| The five shots: spawned with the original's y slip (x plus `$130`), parked off the screen, the cycle's three every fourth pass of fire, the extra two together once the power is on, each hit thrown `$280` right, a reward passed through. | `spawn_shots_matches_the_original`, `spud_update_in_shooter_mode_matches_the_original`, `shot_update_matches_the_original`, `shots_faster_matches_the_original`, `collide_all_in_shooter_mode_matches_the_original`, `collide_all_with_a_shot_hitting_a_bonus_enemy_matches_the_original` (`engine/crates/sf-conform/tests/bonus_routines.rs`). |
| A hit costs one point and half the shot power; below none the enemy falls, then becomes a reward set on the ground; in shooter mode it floats on the wave and its kind comes from the two counters (every `$28`th a life, every tenth other a power-up), with the RNG drawn anyway. | `touch_bonus_enemy_matches_the_original`, `death_fall_matches_the_original`, `become_reward_matches_the_original`, `reward_float_matches_the_original`, `reward_arc_matches_the_original`, `touch_reward_matches_the_original` (same file). |
| Every enemy's spawner, init and update, the paths' ends, and the animation toggles inside the code. | `spawners_match_the_original`, `inits_match_the_original`, `updates_match_the_original`, `the_paths_end_where_the_listing_says` and their `the_second_half_s_` counterparts, `obj_path_xy_matches_the_original` (`engine/crates/sf-conform/tests/bonus_enemy_routines.rs`). |
| A death with a life left puts the frog back on the screen without leaving the loop; the last life lost is game over. | `bonus_frame_matches_the_original` (`bonus_routines.rs`); `bonus_death_wb_1` (`engine/crates/sf-conform/tests/frog_cases.rs`, green to frame 2448; 2449 diverges, pinned there). |
| The bonus level played whole: marker 55 ends it, the exit fade, the end picture, on to W6-1. | `bonus_enter_wb_1`, `bonus_end_wb_1` (`frog_cases.rs`); `w5_4_goes_on_to_the_bonus_level`, `the_bonus_level_goes_on_to_w6_1` (`engine/crates/sf-conform/tests/chain.rs`). |
| The shots' `sfx_play` wait of three lines on channel 2. | `sfx_play_waits_three_lines_and_turns_the_dma_on_when_the_original_does` (`engine/crates/sf-conform/tests/beam.rs`, over `bonus-enter-wb-1`). |

Only the slot machine has Amiga-shaped parts of its own (the first eight rows); the bonus level's
are the level's, and the [level](level.md) chapter lists them. The rows after are game rules, kept
because conformance compares every byte they write.

## 6. Native sketch (non-binding)

This sketch is non-binding: designing the rewrite is the later plan's job.

The rules of both games are already plain code over a few dozen variables, and would carry into a
native rewrite as they are, the variables becoming fields. The bonus level needs nothing else: its
enemies are object kinds like any world's, and it changes when the level machinery changes.

The slot machine's drawing is the hard part. A native slot machine would draw the reels and lamps
each frame from its state. But where each blit lands decides what a frame shows: the lamps cross
the picture while it is drawn, and whether a lamp starts before or after the frame's interrupt
moves the next round's timing by a frame. A rewrite would need that outcome as a schedule: per
frame, which lamps and reels are drawn by which line and on which frame each change shows. The
bus simulator could compute it once, offline, as the [story-and-ending](story-and-ending.md)
sketch proposes for the story, or the rewrite could keep a small beam model for this screen alone.
The checks that assert the draws' lines (`slot-attract`, `slot-fire`, the jackpot flash) would
then be met by the schedule, or changed to compare the picture only; that choice belongs with the
[verification](verification.md) and [memory-model](memory-model.md) decision.

The three copies of the same mechanism here could each be merged into one: the slot's beam clock
with the other three, `rounds::rng` with `LevelCtx::rng_range`, and `mod.rs`'s `draw_lamps` with
`Draw`'s `Part::Lamps`.

## 7. Reuse

Rating: **`superfrog-only`**. 3 876 lines (`python tools/architecture.py totals`).

Both games are this game's content: the slot's lines, ladder, gamble, hi-lo game and jackpot, its
lamp records and screens; the bonus level's enemies, shots and rewards. Only the method carries:
port each routine against the routine oracle over generated states (`slot_routines.rs`,
`bonus_routines.rs`), and play each path by a planned script (`slot_plan.rs`, `plan_bonus.rs`).

The slot's draw on the beam (`draw::spend`, `done`, the cut at the interrupt, `plan` on the bus
simulator) is generic in method but tied to this screen's constants and blits; another game would
reuse `sim` through [story-and-ending](story-and-ending.md), not this file. About 450 lines of
`draw.rs` are that timing.

Another Team 17 game of the period may share the slot machine's text printer and its fruit-machine
code, or the shooter-mode object routines. This is unchecked.

## 8. Sources

- [docs/13 "The slot machine"](../13-front-end.md#the-slot-machine-t71t74-code-oracle): the round
  loop, lines, ladder, gamble, hi-lo game, jackpot, drawing on the beam, and the cases.
- [docs/08](../08-executable-map.md) (the bonus level in the chain, `level_state`, the password
  table) and [docs/09](../09-object-system.md) (`touch_bonus_enemy`, the object system).
- [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.1 (the checks and the tolerance).
- `plans/rust-engine/FINDINGS.md` rows for T74, T129, T203, T213; `plans/engine-architecture/FINDINGS.md`
  rows for T03, T04, T09, T10.
- Code: `engine/crates/sf-core/src/slot/`, `engine/crates/sf-core/src/bonus/`,
  `engine/crates/sf-core/src/front/mod.rs` (`Routine`, `Front::step`),
  `engine/crates/sf-core/src/level/tally.rs`, `engine/crates/sf-core/src/layout.rs`,
  `engine/crates/sf-conform/tests/front.rs`, `engine/crates/sf-conform/tests/cases.rs`,
  `engine/crates/sf-conform/tests/channels.rs`, `engine/crates/sf-conform/tests/slot_routines.rs`,
  `engine/crates/sf-conform/tests/bonus_routines.rs`,
  `engine/crates/sf-conform/tests/bonus_enemy_routines.rs`,
  `engine/crates/sf-conform/tests/frog_cases.rs`, `engine/crates/sf-conform/tests/chain.rs`.
