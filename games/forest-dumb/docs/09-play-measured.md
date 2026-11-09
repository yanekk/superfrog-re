# 09 — Play in W1-1, measured

What the level 1 plan's tasks rest on, measured on the oracle (plan `forest-dumb-level-1`, T00,
2026-11-05) instead of read from the listing. Each answer says **measured** (the original was run and
watched), **read** (from the listing only) or both. Names are in [`re/symbols.toml`](../re/symbols.toml);
addresses are `hN+$x` as in [07-executable-map.md](07-executable-map.md), whose hunk bases hold for this
boot too.

The kept evidence is `engine/crates/fd-conform/tests/play_measured.rs` (labelled `full`, about 32 s) and
the script `engine/conform/scripts/boot-to-level1.txt`. The other measurements were scratch probes on
the oracle (`step`, `peek`, `poke`, `cpu_writes`, `irqs`, `blit_events`); the probes were deleted and
their results are below. "Frame `n`" means after `n` steps, as in 07.

## Reaching the level: the sync point

Measured. `boot-to-level1` is `boot-to-title` (title idle from frame 4922), then:

| Step | Frame | What happens |
|---|---|---|
| Fire on START | held 5000-5010 | `title_screen` returns in that frame; `game_running` is 1 from frame 5001; world 1 loads from disk 2 through the OS |
| SCENA 1 | from frame 6206 | `scena_screen` (`h0+$7c4c`) draws the world name and spins on CIA-A fire (`h0+$7c7e`) |
| Fire on SCENA 1 | held from step `N`, `N` ≥ 6206 | the copper list of play is set and the first level-3 interrupt runs `game_frame` in the same frame: frame `N+1` is play's first, `irq_count` 1 |

The script presses at 6300, so **frame 6301 is the first frame of play**. Two boots give identical chip
and slow RAM there (test). How long the SCENA screen waits does not matter: with the press at 6300 and
at 6500, every byte of all five hunks was equal 1 000 frames into identical play (scratch probe), so a
sync point anywhere on the SCENA wait is equivalent. Fire held earlier than 6206 was not tried. World 1
starts with `char_x`, `char_y` = `$70`, `$70`, set by `load_list_world_1`; the character falls to `y` `$8c` by
frame ~6420.

## Passes and frames

Measured over 3 000 frames of idle play and 3 000 of moving and jumping (test, and probes), and
**re-measured with T31's code-ran counter** at `level3_handler` (`h0+$1eda`), `game_frame` (`h0+$1fec`)
and the pass end (`h0+$1f02`) by T06, 2026-11-05 (`play_measured.rs`,
`the_code_ran_counter_sees_one_handler_one_game_frame_and_one_pass_end_a_step`):

- One level-3 interrupt a frame, always: the copper raises it (`INTREQ=$8010` in the copper list at line
  `$e9` = 233). The CPU took it at line 233 when idle and up to 239 when moving (the blitter was holding
  the bus; inferred). Counter: `level3_handler` started once in each of 6 000 steps.
- `irq_count` +1 every frame. `game_frame` runs once per interrupt, never skipped and never twice.
  Counter: `game_frame` started once in each of 6 000 steps.
- **Where a pass ends** (counter). The handler writes `INTREQ=$4010` at `h0+$1f02` just after
  `game_frame` returns; `Amiga::watch_write(Some((0x9c, 0x4010)))` catches it, and the RAM it copies
  holds `irq_count`, which names the pass. Idle, `game_frame` returned before the step's end in 2 968
  of 3 000 frames. Moving and jumping, it returned after the step's end in 2 977 of 3 000: the state at
  a step's end is mid-pass on almost every moving frame (`char_update`, `char_object_cells` and
  `play_keys` come after `countdown_step` in `game_frame`). Once in a while a step sees no pass end
  and the next sees two (around `main_loop`'s 32-interrupt check); the RAM copy then holds only the
  first. T00 saw the CPU in the handler at every step's end; when idle that is the 7-line wait after
  `h0+$1f02` (read, not counted).
- T00 wrote (2026-11-05, corrected by T06 the same day): "`game_frame` and the handler's 7-line wait end
  after the frame's end: every frame", and "`game_frame`'s own tail (`countdown_step`, near its end) fell
  after the step's end in 35 of 3 000 moving frames, and none of 3 000 idle frames". Both were read from
  the CPU being in supervisor mode and from `countdown_tick`, not from the routine's return; the counter
  shows the return before the step's end when idle, and after it on nearly every moving frame.
- So a case compares at the end of each pass, not at the end of each frame. T06's runner does
  (`engine/conform/README.md`, "How a level case compares").
- Paused (`paused` set) the interrupt still counts and `game_frame` runs only `play_keys`.

## Input

Measured: right held from step `N` changes `char_x` in the state after step `N` (frame `N+1`), by one
pixel, and nothing before it (test, `N` = 6400). `read_joystick` reads `JOY1DAT` once a pass, inside
`game_frame`, so the joystick is sampled at the interrupt (line 233-239) of the frame it is held in.

Keys (measured): `level2_handler` puts the raw CIA byte in `last_key_byte`, and `play_keys` reads it
each pass.

| Key | Effect |
|---|---|
| P, press then release | toggles `paused`: P down sets `pause_key_down`, the release flips `paused`; frame 6506 after a press at 6500-6504 |
| Esc, press then release | sets `give_up`; `main_loop` goes to game over at its next check (within 32 interrupts) |
| HELP | with `cheat_flag` set ends the level (`level_exit_touched`, `items_left` 0); `cheat_flag` is set only by a title code (`title_code_world`), so it is out of this plan's reach (read) |

The owner decided on 2026-11-05 that the app keeps DESIGN §2.7: Esc quits, and no key pauses. The
engine still models P and Esc so a case that feeds them matches.

## Display

Measured from the copper list of play (`h1+$d4`, read back at frames 6500 and 7000) and from `cpu_writes`:

- DIW is set by the **copper**; the CPU writes no DIW, DDF or sprite pointer in play.
- Play area: `DIWSTRT=$2f91`, `DIWSTOP=$e2a1`: lores 145 to 417 (272 wide), lines 47 to 226. `DDFSTRT=$30`
  fetches one word early for `BPLCON1` scrolling. `BPLCON0=$5000`: 5 bitplanes, 32 colours, interleaved
  (`BPL1MOD=$bc`, rows `$e6` apart).
- A copper `WAIT` partway down (line `$e8` at frame 6500, `$7e` at 7000) reloads the bitplane pointers:
  the play screen is a circular buffer in hunk 3 from `h3+$398`, scrolled by moving the wrap line.
- Status bar from line 241: `DIWSTRT=$f181`, `DIWSTOP=$29c1`, 320 x 56, `BPLCON0=$5200`, its own buffer
  at `$53a68`.
- `Rgb12Frame` crops to the frame-end DIW, the status bar's, so `picture()` shows only the 320 x 56
  bar (measured: every play frame). T04 adds `picture_in`.

### The status bar's redraw and the next pass (T71, measured 2026-11-06)

`main_loop` calls `h0_6a96` while `irq_count & 31` is 0, not once: after pass `32k` it redraws what
changed, runs the edge redraws, then spins through `main_loop` and `h0_6a96`'s four compares until the
level-3 interrupt at line 233 starts pass `32k + 1`. `h0_6a96` reads `countdown` (`h0+$6a96`), `lives`
(`$6aaa`), `score` (`$6abe`) and `items_left` (`$6ad2`); a number whose read comes after the
interrupt is drawn as pass `32k + 1` left it, and stays so until the next redraw.

Where the interrupt lands, read off line samples (the exception frame's PC under `level3_handler`'s
`movem`; `fd_conform::interrupted_pc`), varies from redraw to redraw: about 20 different PCs over 220
redraws, idle, `death-spikes`, `collect-return`, `contact-route`, `stretch-scroll`. Idle, mostly between
the score's and the items' reads or after all four; on the spikes, 38 of 50 in `main_loop` before any
read (pass 1441, the life lost at frame 7742, among them); moving, about half before any read and half
after all. It depends on the cycles every pass took, so the engine cannot know it: it redraws before
pass `32k + 1` and offers the other versions (`Game::cut_redraw`), and a case accepts the version the
cut it measured gives, on the bar only (owner, 2026-11-06; `engine/conform/README.md`). Never seen: a
cut inside a draw routine, where `h0_6c80` would draw lives' digits and icons from two reads and
`h0_6bc0` would store the new value as drawn after drawing the old. `tests/status_race.rs` holds it.

## Sprites, bobs, playfields

Measured:

- **No dual playfield** (`BPLCON0` bit 10 clear in both windows).
- **Hardware sprites: the character.** Sprites 0-3 are two attached pairs side by side, 32 pixels wide,
  40 lines, 15 colours. `char_sprite_copy` (`h0+$9b06`) copies the frame at `char_anim_ptr` into their
  data every pass; `char_sprite_position` (`h0+$5462`) writes their control words from `char_screen_x`
  (the sprite moved with it in the probe). Sprites 4-7 point at one empty sprite. `BPLCON2=$1f`: sprites
  in front of the playfield. So the plan's gate holds: T10 takes Superfrog's sprite composition into the
  toolkit.
- **Bobs: the enemies and the items.** One moving frame (7050) made 45 blits: tile copies into the play
  screen (`BLTCON0=$09f0`, A to D, from `_GenN-1`'s tiles), items cut over a tile (`$0fca`, A and B from
  `_ELEMENTY`, C a tile, D the screen) and two creatures cut onto the screen (`$9fca`, `$5fca`, A and B
  from `StworkiNew`, **C and D the screen**).

## Are the display buffers game state?

Measured with two experiments, overwriting memory at frame 7000 and comparing with an untouched run:

- **The play screen is not game state.** With `h3+$398..$c3c8` (`$12fd0..$1f000`) overwritten, every word
  of hunks 0, 1, 2 and 4 was equal 400 frames later, idle or moving. Nothing in play logic reads the
  screen.
- **It is picture state.** Idle, all 24 600 overwritten words were still there 400 frames later: nothing
  redraws a still screen. Moving, all but 63 (an unused margin at `h3+$398..$414`) were redrawn from the
  map as the camera passed. Creatures are cut onto whatever the screen holds; whether their old place is
  restored by a tile copy (one frame showed a tile copied to a creature's destination) was not proven.
  T10 decides with a picture case.
- **The collision map is game state.** `collision_map` (`h3+$fa50`, 1 bit a pixel, 256 rows of `$2e` bytes,
  row `y & $ff`, offsets in `collision_rows`) overwritten at frame 7000 changed 110 words of hunk 0 within
  100 frames. It is written by the foreground's `redraw_edge_*` (read: its references sit in them) and by
  `game_frame` via `h0+$2fea` (read), and read by `char_probe`. So it is built incrementally as the view
  scrolls, like the screen, and the engine must keep it the same way (T08, T09).

## Random numbers

None found. Measured: play from a SCENA press 200 frames later was equal in all five hunks after 1 000
frames of the same input, so nothing seeds from time. Read: the only beam read in play outside raster
waits is `game_frame`'s `VHPOSR` into `beam_sample_unread`, which nothing reads; no CIA timer or TOD is
read; no routine has a seed's shape. So there is no start seed to carry. A source reached only by an
event not in the probe (an enemy type not met) is not ruled out; T21 meets every object.

## Variables

| Name | Where | Size | What | How known |
|---|---|---|---|---|
| `char_x`, `char_y` | `h0+$1e5a`, `$1e5e` | long | level pixels; start `$70`, `$70` | measured |
| `camera_x`, `camera_y` | `h0+$1e62`, `$1e64` | word | view's top-left; clamped to `$17cf`, `$53f` (`$52f` corrected by T34, T36) | measured follows, clamps read |
| `char_screen_x` | `h0+$4d78` | word | `char_x - camera_x` | measured |
| `lives` | `h0+$6a84` | BCD word | 32 at level 1 (`lives_reset`); +1 to 99, -1 to 0 | measured start, rest read |
| `hits_taken` | `h0+$3b26` | word | contact passes in a row; the third costs a life (`char_hit_check`, [10](10-object-system.md)) | measured (T59) |
| `countdown` | `h0+$6a80` | BCD, 3 digits | 999 at level start, -1 every 63 passes; +100 capped at 999 | measured |
| `items_left` | `h0+$6a88` | BCD word | 282 in world 1, -1 a pickup; the exit counts only at 0 | measured |
| `score` | `h0+$2a66` | BCD long | `score_add` added by `score_add_routine` | measured |
| `world_index` | `h0+$6dfe` | long | 0 for world 1 | measured |

Whether the player sees `countdown` as time or as energy is not known; `Game::energy` reads it (T23).

## Level end and game over

`main_loop` checks only when `irq_count & 31` is 0, so every one of these acts up to 31 frames late.

- **Level end** (measured by putting `$ff` in the object map under the character and `items_left` to 0
  at frame 6500): `char_object_cells` clears `level_exit_touched` each pass and sets it on an `$ff` cell of
  the object map (`h0+$cb38`, `_MapaBobowN`, 96 cells a column). With it set and `items_left` 0,
  `main_loop` calls `next_world`: `world_index` was 1 by frame 6560, world 2 loaded from disk and its SCENA
  screen waited from frame ~7531. No other end-of-level sequence ran in between. With items left the exit
  does nothing (read). So the comparison's end is `world_index` becoming 1.
- **Level end, measured (T26, `fd-conform/tests/level_end.rs`).** `cheat_flag` set at the sync point,
  HELP pressed in step 6523 is read by pass 224's `help_key`; `main_loop`'s check after it runs
  `next_world` in step 6525, before `h0_6a96`, so that pass's frame keeps the old status bar. `h0+$500`
  then blacks the play area's 32 colours. A HELP read by a pass the check does not follow is lost on
  the original (`char_object_cells` clears `level_exit_touched` next pass, T22), not on the engine yet.
  The level-end milestone for T27: case goal `level_end = true`; on the engine alone `Game::finished()`.
- **Game over** goes to `h0+$1aa`: `game_over_screen` (waits for fire), then the high scores (`h0+$89d0`),
  then `session_loop` and the title. Measured: Esc (`give_up`) and `countdown` reaching 0 (poked to 1 at
  frame 6500: 0 at 6526, game over screen by 6560). Read: `lives` 0 does the same.
- **The game over screen, measured (T23, `fd-conform/tests/death.rs`).** Esc held in the step a pass
  starts sets `esc_down` in that pass, its release `give_up`. `main_loop`'s check after pass `32k`
  enters `game_over_screen` in the next step, and no pass runs again (`irq_count` stays; its copper
  list `h1+$810` raises no interrupt). Fire held in the 21st step after that one or later counts as
  pressed (`h0+$8a78`'s 21 frames); the step it is no longer held in enters the high scores. The
  countdown run out from frame 6400 shows the screen at frame 69245. The engine skips the high scores
  and restarts the level (DESIGN §2.4).

## The character's update

`game_frame` calls these for the character each pass, in this order: `read_joystick`, then
`char_update`, which runs four sub-steps (`h0+$3e0c`, a `dbra` from 3; counter latched at `h0+$3e2c`), each
`char_probe`, `char_probe_flags`, `char_step`; and around it `camera_update`, `char_sprite_position`,
`char_sprite_copy`, `char_hit_check`, `char_object_cells`. Sizes count each routine's own instructions,
following branches from its entry; callees are listed apart. Sizing them into porting tasks is T34's.

| Routine | Address | Instructions | Interruptible points |
|---|---|---|---|
| `char_update` | `h0+$3e0c` | 8 | loop of four, counter in memory at `h0+$3e2c` |
| `char_probe` | `h0+$4f84` (`bra` to `h0+$5500`) | 199 | reads `collision_map`, which the foreground writes |
| `char_probe_flags` | `h0+$4f90` | 60 | |
| `char_step` | `h0+$3e30` | 107 | branches on state flags `h0+$4b62`, `$4ecf`, `$4ed0` |
| | `h0+$3ea8` | 93 | 4-line raster wait at `h0+$3f36` and `h0+$400e` (a sound effect's start) |
| | `h0+$4274` | 25 | |
| | `h0+$4696` | 21 | |
| | `h0+$4718` | 57 | |
| | `h0+$47e2` | 15 | |
| | `h0+$48be` | 57 | |
| | `h0+$497c` | 15 | |
| | `h0+$4b0e` | 17 | |
| `char_step_4be6` | `h0+$4be6` | 13 | |
| | `h0+$4c28` | 13 | |
| | `h0+$4c6a` | 34 | |
| | `h0+$410c` | 18 | |
| | `h0+$416c` | 13 | |
| | `h0+$4d32`, `h0+$4d54` | 6, 6 | |
| `read_joystick` | `h0+$8dfc` | 40 | reads `JOY1DAT` once; also appends to `input_log` |
| `camera_update` | `h0+$4d7e` | 48, plus four of ~32 | sets the edge flags `main_loop` redraws from |
| `char_sprite_position` | `h0+$5462` | 34 | |
| `char_sprite_copy` | `h0+$9b06` | 73 | |
| `char_hit_check` | `h0+$3b2a` | 131, plus 52 | raster waits of 3-4 lines at `h0+$3ae6`, `h0+$3d98` (sound) |
| `char_object_cells` | `h0+$2698` | 14, then `h0+$26ec` 153, plus 88 | blitter wait in `h0+$29ca` (read: redraws a taken item's cell) |

About 780 instructions for the movement itself and 760 more around it. What can stop it partway
(measured unless marked):

- Nothing interrupts `game_frame`: it runs at interrupt level 3 and only levels 2 and 3 are enabled
  (`INTENA $c018`, read; one level-3 interrupt a frame, measured).
- The raster waits above only take time; they change no state.
- `game_frame` does interrupt the foreground. `main_loop` runs `redraw_edge_*` between interrupts, and they
  write the screen and `collision_map`; a redraw longer than the rest of the frame is cut by the next
  pass, whose `char_probe` then reads a half-drawn edge (inferred, not seen).
- `main_loop`'s 32-interrupt check is a latch for level end and game over (above).
- `game_frame`'s tail crosses the frame's end under load (above).

### The animation pickers and the joystick read (T51)

Read off the listing before porting, 2026-11-05; ported to `fd-core/src/character/{anim,input}.rs`
and proven on the rig (`fd-conform/tests/character_anim_routines.rs`, 10 000 states each).

| Routine | Reads | Writes | Interruptible points |
|---|---|---|---|
| `char_step_4be6`, `h0_4c28` | `char_walk_t`, the walk table (`h0+$4f30` left, `$4edc` right) | `char_facing` (`$ff`, 1), `char_walk_t`, `char_anim_ptr`; d0, a0 | none |
| `h0_4c6a` | `char_probe_1`, bits 0-1 of `char_probe_2`, `char_facing`'s sign, one of `h0+$4f04`..`$4f60` | `char_anim_ptr`, or nothing | none |
| `h0_4d32`, `h0_4d54` | `char_facing`'s sign, `h0+$4ef0`/`$4f44`, `$4ef4`/`$4f48` | `char_anim_ptr` | none |
| `read_joystick` | JOY1DAT once, `input_log_index` | `joy_left`..`joy_down`, `input_log`, `input_log_index`; d0, d1, a0 | none: one hardware read, no wait |

None waits on the beam or the blitter, touches INTENA or loops across passes; each runs whole inside
`game_frame`. Readings kept by the port:

- `cmpi.b #$28; bmi` keeps a walk counter of `$a8`..`$ff` (the difference is negative), so the walk
  tables are read up to 31 entries in, past `h0+$4f84` into code. The game counts up from 0 and wraps
  at `$28`, so play never gets there; the port reads hunk 0 as loaded (`AnimTables`) and matches anyway.
- JOY1DAT encodes direction as the hardware does: left bit 9, right bit 1, up bit 9 xor 8, down bit 1
  xor 0. So `$0002` is right and down. `fd_core::character::joy1dat` makes the word from a `FrameInput`.
- `input_log_index` past `$2ee0` would write past `input_log`; the game stops at `$2ee0`. The engine
  keeps no byte there: an exclusion no generator state reaches.
- `char_anim_ptr` and the tables hold relocated pointers. At the sync point it is `$c491a0`, the
  program's own `h0+$419c8` at the oracle's hunk 0 base. Whoever loads the tables for play (T08)
  loads hunk 0 at that base for `char_anim_ptr` to compare.

### The character's movement (T08)

Read off the listing before porting, 2026-11-05; ported to `fd-core/src/character/physics.rs`
(`Character`, state in `State.motion`) and `level/collision.rs` (`CollisionPlane`), and proven on the
rig (`fd-conform/tests/character_routines.rs`, 10 000 states each). `game_frame` itself is T63's.

| Routine | Reads | Writes | Interruptible points |
|---|---|---|---|
| `char_update` | all below | `char_update_t` (`h0+$3e2c`, 3 to 0); d7 `$ffff` | the loop count lives in memory, but nothing interrupts `game_frame` (above) |
| `char_probe` | `char_x`, `char_y`, `collision_rows`, `collision_map` and past it | `char_samples` (`h0+$54f0`..`$54ff`); d0-d4, a0, a5 | reads `collision_map`, which the foreground redraws write between passes |
| `char_probe_flags` | the samples, `h0+$4f88` | `char_probe_1`..`_6`; d0, a0 | none |
| `char_step` | flags, joystick, `char_dying`, `char_rising`, `char_falling` | as its callees; the dying fall `char_dying_t`, `char_unhurt_t` | none |
| `h0_3ea8` | run counters, `h0+$40ac`/`$40cb` | slide, `char_x`, `char_anim_ptr`; AUD2 and DMACON on the slide's step 3 | a 4-line raster wait before the sound: time only |
| `h0_410c`, `h0_416c`, `h0_4274` | `char_jump_t`, `h0+$44d4`, `$431e`, `$42ec`, `$43ba`, up | `char_y`, jump state; d0, a0 | none |
| `h0_4696`, `h0_4718`, `h0_48be`, `h0_47e2`, `h0_497c`, `h0_4b0e` | walls, slope bits, run counters, `h0+$49d0` | `char_x`, `char_y`, run and slide state; d0, a0 | none |

Readings the port keeps:

- The collision map is indexed `rows[y & $ff] + (x >> 4 << 1)` with `x` a level coordinate and no
  wrap at a row's `$2e` bytes, so past 368 pixels a probe reads later rows, and near row 255 past
  the map's `$2e00` bytes. `CollisionPlane` holds `$400` bytes after the map; a case transplants them.
- `h0_47e2` and `h0_497c` return their step in d0 and the callers move `char_y` by it.
- A table indexed past its end reads what follows: `h0+$40cb + $3d` is `char_slide_t` itself. The
  engine reads a modelled variable there from the state (found on the rig).
- `char_update_t` is 3 at the sync point: frame 6301 ends inside pass 1 at `char_update`'s start, none of its sub-steps run (T69, docs/12).

## The camera (T09)

Ported in `fd-core/src/level/camera.rs`, each proven on the routine rig (`fd-conform/tests/camera_routines.rs`).
Read from the listing 2026-11-05; the behaviour is what T36's witnesses showed equal to the original.

| Routine | Address | Instructions | Interruptible points |
|---|---|---|---|
| `camera_update` | `h0+$4d7e` | 48 | none inside; writes the latches and positions `main_loop` reads |
| `h0_4e9a`, `h0_4ea6`, `h0_4eb4`, `h0_4ec2` | flag steps (right, left, up, down) | 9-10, loop at `h0+$51a0`.. | none: a `dbra` loop on `d7`, run whole |
| `h0_51fc`, `h0_526a`, `h0_52d8`, `h0_5346` | one-pixel scrolls | 23 | none |
| `h0_513c` | clamp | 13 | none |

- All run inside `game_frame` at level 3, which nothing interrupts (above). No raster wait, no hardware.
- `camera_update` clears the crossed flags `h0+$51f8..$51fb`, then for right, left, down, up in that order
  reloads `char_x - camera_x`, `char_y - camera_y` (low words) and runs the flag step past the dead zone
  (`$94`, `$78`, `$5a`, `$46`). A flag step scrolls one pixel at a time until a scroll refuses.
- A scroll that crosses a 16-pixel boundary sets its crossed flag, its latch (`h0+$5196`, `$5198`, `$519a`,
  `$519c`: right, left, up, down) and `frame_flag`, unless the latch is already set, when it does not move.
  So the camera crosses at most one boundary per direction until `main_loop` clears the latch.
- The latches are the hand-off to the foreground: `main_loop` reads them with the positions `camera_update`
  stored (`h0+$5186`.. one x, y word pair a direction) and redraws that edge (T55, below).
- The clamp is `$17cf`, `$53f`: `cmpi.w` overflows for `camera_y` in `$8000..$853e`, giving `$53f`, not 0
  (T34, T36). T00's `$52f` is the clamp of an unlabelled joystick pan at `h0+$50a2`, which calls the flag
  steps' loops with `d7 = 4`; nothing in `game_frame`'s tree reaches it.
- `level3_handler` saves and restores every register around `game_frame` and sets `frame_flag` after it, so
  no register the camera leaves is read, and a scroll's `frame_flag` write is overwritten the same pass.

## The edge redraws (T55)

Ported in `fd-core/src/level/redraw.rs`, each proven on the routine rig over 10 000 states
(`fd-conform/tests/edge_redraw_routines.rs`). Interruptible points listed from the listing 2026-11-05,
before porting; the cut was then measured.

| Routine | Address | Instructions (own) | Interruptible points |
|---|---|---|---|
| `redraw_edge_a`, `_b` (down, up) | `h0+$844`, `$b20` | 140, 141 | anywhere: run by `main_loop` in the foreground, which every pass interrupts; a raster wait in `h0_3ac2` before each of 23 tile blits and each bob |
| `redraw_edge_c`, `_d` (left, right) | `h0+$df6`, `$114a` | 181, 177 | the same, 16 tile blits |
| `h0_774`, `h0_a06`, `h0_ce4`, `h0_104c`, `h0_1390` | blits | 15-38 | the raster and blitter wait before the register writes |
| `h0_3ac2` | wait | 5 | loops while VHPOSR's byte is `$e7` to `$66` (signed) or the blitter is busy |
| `h0_7cc`, `h0_681c`, `h0_6852`, `h0_6882` | | 13-78 | none inside; interrupted between, like any foreground code |

- `main_loop` (`h0+$2a0`): when `frame_flag` is set, for down, up, left, right in that order, it copies a
  latched direction's `edge_at` to `h0+$1e66`/`$1e68`, clears the latch and calls its redraw, then clears
  `frame_flag`. While a latch is set the camera refuses to cross that way again, so the interrupt cannot set
  a latch `main_loop` then clears unredrawn.
- A pass's state is the original's before the foreground has redrawn: `h0+$1e66` lags the latches by one
  pass, and `frame_flag` reads `$ffff` at every pass's end, the sync point included (measured 2026-11-05).
  So `Game::step` runs the foreground's redraws for the last pass's latches first, then the pass.
- At x 0 (y 0) the left (up) scroll crosses every pass and latches; the redraw's `x >= 1` (`y >= 1`) guard
  then draws nothing (measured: the left latch reads `$ffff` at every pass from the sync point).
- **Cut short, measured.** Two edges in one frame (a row and a column) do not finish before the next pass:
  8 of 520 frames of a scroll each way, the original's `collision_map` after the next pass lacked part of
  the second redraw and equalled the engine's one pass later. The engine runs the foreground whole, so a
  pass in such a frame sees a whole edge where the original's `char_probe` could see half of one. Not
  modelled: it needs the foreground's timing against the raster (T41 to T43 show whether a case meets it).
  Where it changes a latch, it is modelled since T73, and a case hands the engine the measured cut
  since T74 (next section).

### The edge redraws and the next pass (T73)

Measured 2026-11-09 on T24's planner route (`fd-plan level W1-1`, seed 1, 100 rounds, at `0c06449a`),
frozen as `fd_plan::confirm` plays it; its first 1 300 steps are `cases/edge-latch-route.toml`, held by
`fd-conform/tests/edge_latch.rs`.

- **The window.** `h0_3ac2` starts no blit outside lines 103 to 230, and the next pass comes at line
  233. A frozen pass ends near line 311 or within the next frame's first lines, so the foreground's
  blits always start at line 103: a row's 23 tiles (`BLTSIZE $1401`, A to D) end near line 161, its
  shapes and scan follow, and its first bob (A to D through a mask) starts near line 197, a bob every
  5 lines. A column's 16 tiles take about 3 lines each.
- **When the latch after a row is cleared.** `main_loop` clears a latch just before its redraw, so the
  pass after a two-edge pass (always a row and then a column) still holds the column's latch only when
  the row's redraw has not returned by line 233. Rows of 9 bobs or more never return in time (all 17 of
  the 1 300 steps; pass 1211's up row had 9, so its right latch stayed `$ffff` through pass 1212); rows
  of 7 or fewer always do (the 7th bob at line 227); of two rows of 8, pass 893's was cut (still in its
  scan at `h0+$ca6`) and pass 899's was not. A row finished, the column is cut instead, its latch already
  clear: 103 frames of 36 tiles, no latch differing.
- **What follows a cut row.** After the next pass `main_loop` finishes the row, tests the latches after
  it as they then stand, and clears `frame_flag`; a latch that pass set earlier in the order (a row
  again) waits for the pass after, when the level-3 handler has set `frame_flag` again.
- **The engine alone** (`level::main_loop_redraws`, `Cut::Rows`, `ROW_BOBS_CUT` = 8) stops its redraw
  half after a row of 8 bobs or more and resumes it from `State::edges_resume` before the following
  pass. It draws the cut row whole, so the row's last bobs land in the edge row outside the view one
  pass early. Kept for the app and the planner's search (T74, below): it is right for every row of 8 or
  more bobs and every row of 7 or fewer but one, and the cuts it misses depend on cycles.
- **Unfrozen.** With creatures drawn and `h0_6882` running, a pass can end as late as line 132 of the
  next frame, which shrinks the window, and the scan's animated cells cost time: on the same route
  unfrozen, the pass after pass 343 found a row with no bobs still running (in `h0_6882`), and 6 cuts
  in all left a column latch set, some after a status bar redraw.
- Each redraw draws a row of 23 tiles or a column of 16 into its 16-line band of the play screen (the program's
  pointer tables at `h0+$1496` and `h0+$18d6` are 16 rotations of the bands; tile row `r` lands in band
  `r & 15`), the same cells' shapes into `collision_map`, then scans 20 (rows) or 13 (columns) object-map
  cells just outside: 1 to 6 and `$87` to `$fe` start a creature (`h0_681c`) and clear the cell, `$20` to
  `$3f` add an animated cell (`h0_6882`), `$40` to `$86` cut a bob over the screen, `$ff` ends the scan.
- `collision_map` is not wrapped across: a cell's shape goes `col * 2` bytes into its band, so moving right
  the writes run past the map's `$2e00` bytes into the slack before the tiles (`h3+$13540`).
- The rows' scans load only `d0`'s low word before a bob, so after a creature that found no slot (`d0 = -1`)
  the next bob's column is `$1fffxxxx` and its blit, masked to chip RAM, lands far off the screen. The
  engine lists it as the original computes it; T10 decides what the picture does with it.

#### Every cut, measured, and `frame_flag` swallowed (T74, 2026-11-09)

Measured since T47 runs the springs and the animated cells in every case (creatures frozen), with the
line samples of each step: where the interrupt took the foreground, and, inside a redraw, `main_loop`'s
return address (`h0+$2da`, `$302`, `$32a`, `$352`) on the user stack of the last sample taken in user
mode (the exception frame is on the supervisor stack). `fd_conform::edges_cut` reads it.

- **When a pass swallows `frame_flag`.** `main_loop` clears `frame_flag` (`clr.w` at `h0+$352`) after
  its last redraw, whatever has set it since its test at `$2a0`. A pass that comes before that `clr.w`
  (inside any redraw, or between the latches' tests) sets its latches and `frame_flag`, and when it ends
  `main_loop` finishes its redraw, tests the latches after it in the order and clears `frame_flag`: a
  latch the pass set earlier in the order waits a pass. Cut inside `redraw_edge_d` (`$352` on the stack)
  every latch the pass sets waits. `mechanic-vanish` pass 293 (frame 6592, in `h0_6882` from the right
  column's scan) set `edge_latch_down`, which stays `$ffff` through pass 294; `mechanic-spring` and
  `camera-bottom-right` pass 495 (frame 6794) the same. Passes cut so: 20 of `mechanic-vanish`'s 1 608,
  39 of `mechanic-spring`'s 2 598, 50 of `edge-latch-route`'s 1 208 (25 in or just after the right
  column, 6 in the left, 19 in the up row).
- **Rows, re-measured.** On `edge-latch-route` with the animated cells running: all 17 rows of 9 bobs
  cut, 1 of the 2 rows of 8 (pass 893), and 1 of the 142 rows of 7 or fewer, one of no bobs (pass 328,
  in `h0_6882`). `edge_latch.rs` holds it.
- **The scan cut.** The list `h0_6882` gives a new animated cell is the one with the fewest cells, and
  the pass's `h0_62b6` changes the counts, so a cell the scan reaches after the pass can join another
  list: pass 495 of `mechanic-spring` came as `h0_6882` returned (`$69a4`) in the right column, whose
  later coins joined other lists than with the scan whole, and from frame 6845 spun out of step. The
  scan's cell is `a2` (`h0+$cb38` plus `d5` and `d6`), which no routine the scan calls writes; it is
  read from `level3_handler`'s `movem.l` of the foreground's registers.
- **What the runner and the engine do.** The runner samples a step's lines only where its pass may find
  the foreground in the redraws (`frame_flag` or a latch set at the step's start, or the last pass still
  running and short of `camera_update`), hands the cut to the engine before it runs that pass
  (`Engine::foreground_cut`, `level::Cut::At`, `level::EdgesCut`), and the engine stops its redraws at
  the latch not yet tested, holds the interrupted scan's later animated cells (`Edges::deferred`) and
  adds them before resuming after the pass. Its own rule is then off. Sub-frame timing stays out of the
  engine (DESIGN §2.1): which pass is cut is the original's cycles, handed over as T67 hands the input
  each pass read. Not handed: a creature the cut scan starts after the pass (T75), and a pass that
  ends within the few dozen cycles before line 233 that four clear latches' tests take.
- **Only where creature starts are frozen** (T74 review, 2026-11-09, owner). Sampling a step's lines
  (`Amiga::set_profile`) changed the original with creatures live: `enemy-route` and `enemy-contact`
  ended a pass a step later from step 336 and 377. A measurement that changes the original is not
  one, so the runner samples for the cut, and hands it, only in cases freezing `h0_681c`
  (`fd_conform::records_edge_cuts`, `Sampler`); elsewhere the engine keeps `ROW_BOBS_CUT`. In those
  ten cases the original sampled as the runner samples it equals the original alone every frame (chip
  and slow RAM, picture, registers; `edge_latch.rs`). T78 makes sampling observe only.

## What this changes for later tasks

- T03: the sync point is frame 6301 of `boot-to-level1`, or any frame on the SCENA wait.
- T04: the window to capture is the play area above plus the status bar.
- T06: compare at the end of each pass (`INTREQ=$4010` at `h0+$1f02`), because the frame's end falls inside
  `game_frame` on almost every moving frame. One pass a frame holds (measured with the counter, T06).
- T08, T09: the character runs four sub-steps a pass; `collision_map` is state the edge redraws build.
- T10: the character is a hardware sprite; the screen is picture state, not logic state.
- T23, T26: game over and level end act at `main_loop`'s 32-interrupt check.
- No start seed.
