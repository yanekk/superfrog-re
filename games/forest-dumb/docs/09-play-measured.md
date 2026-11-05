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
| `camera_x`, `camera_y` | `h0+$1e62`, `$1e64` | word | view's top-left; clamped to `$17cf`, `$52f` | measured follows, clamps read |
| `char_screen_x` | `h0+$4d78` | word | `char_x - camera_x` | measured |
| `lives` | `h0+$6a84` | BCD word | 32 at level 1 (`lives_reset`); +1 to 99, -1 to 0 | measured start, rest read |
| `hits_taken` | `h0+$3b26` | word | reading: the third hit costs a life (`char_hit_check`) | read |
| `countdown` | `h0+$6a80` | BCD, 3 digits | 999 at level start, -1 every 63 passes; +100 capped at 999 | measured |
| `items_left` | `h0+$6a88` | BCD word | 282 in world 1, -1 a pickup; the exit counts only at 0 | measured |
| `score` | `h0+$2a66` | BCD long | `score_add` added by `score_add_routine` | measured |
| `world_index` | `h0+$6dfe` | long | 0 for world 1 | measured |

Whether the player sees `countdown` as time or as energy is not known yet (T21, T23).

## Level end and game over

`main_loop` checks only when `irq_count & 31` is 0, so every one of these acts up to 31 frames late.

- **Level end** (measured by putting `$ff` in the object map under the character and `items_left` to 0
  at frame 6500): `char_object_cells` clears `level_exit_touched` each pass and sets it on an `$ff` cell of
  the object map (`h0+$cb38`, `_MapaBobowN`, 96 cells a column). With it set and `items_left` 0,
  `main_loop` calls `next_world`: `world_index` was 1 by frame 6560, world 2 loaded from disk and its SCENA
  screen waited from frame ~7531. No other end-of-level sequence ran in between. With items left the exit
  does nothing (read). So the comparison's end is `world_index` becoming 1.
- **Game over** goes to `h0+$1aa`: `game_over_screen` (waits for fire), then the high scores (`h0+$89d0`),
  then `session_loop` and the title. Measured: Esc (`give_up`) and `countdown` reaching 0 (poked to 1 at
  frame 6500: 0 at 6526, game over screen by 6560). Read: `lives` 0 does the same. Nothing was measured
  after fire on the game over screen.

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

## What this changes for later tasks

- T03: the sync point is frame 6301 of `boot-to-level1`, or any frame on the SCENA wait.
- T04: the window to capture is the play area above plus the status bar.
- T06: compare at the end of each pass (`INTREQ=$4010` at `h0+$1f02`), because the frame's end falls inside
  `game_frame` on almost every moving frame. One pass a frame holds (measured with the counter, T06).
- T08, T09: the character runs four sub-steps a pass; `collision_map` is state the edge redraws build.
- T10: the character is a hardware sprite; the screen is picture state, not logic state.
- T23, T26: game over and level end act at `main_loop`'s 32-interrupt check.
- No start seed.
