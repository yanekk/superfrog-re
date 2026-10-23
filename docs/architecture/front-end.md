+++
id = "front-end"
title = "The front end"
tier = "subsystem"
depth = "deep"
kind = "amiga"
mix = { amiga = 60, game = 25, data = 6, shell = 9 }
reuse = "superfrog-only"
owns = [
  "engine/crates/sf-core/src/front/",
  "engine/crates/sf-core/tests/front.rs",
]
+++

# The front end

## 1. What it is

The front end covers every screen around the levels except the story intro and the end
sequence: boot, the title and its intro (the eyes in the dark), the menu and the options
screen, level codes, the award pages, the hall of talent and the credits, the high-score
name entry, game over, and the set-up of the attract demo. It also includes the shared machinery
these screens run on: the fades, the IFF ANIM player the end sequence calls, the frog
graphics loader, and the disk data the front end reads. The original's side is in
[docs/13](../13-front-end.md); this chapter describes the engine's port.

The port runs the original's routines over the original's memory. Each routine is a resumable
state machine over the 68000 code (`Routine` in `engine/crates/sf-core/src/front/mod.rs`, one
variant per routine, a `pc` field per state). It runs until the original would wait and then
returns a `Yield`: a raster line (`Yield::Raster` is `wait_frame_raster`, line `$12b`), a fade
record's done flag (`Yield::Fade`), a busy CPU (`Yield::Busy`), interrupts off past the frame's
end (`Yield::Masked`), a call or a jump. `Front::step` runs one frame. It runs the vertical-blank
handler first (`vbl::run`, owned by [frame-interrupts-input](frame-interrupts-input.md)), then
the main program's call stack (`run_main_split`) in the order of the lines each part wakes on.
It then draws the picture from the copper list as each part left it on the beam (the `history`
of `Snapshot`s in `Front::step`). What the player experiences, such as the eyes bouncing, the
menu choice, a code typed or a name entered, is the original's logic. How it reaches the screen
is copper-list rewrites, sprite records, blits and colour fades, timed to the line.

The game's flow between levels (`chain.rs`), the story intro (`story/`), the end sequence
(`ending.rs`), the front end's vertical blank (`vbl.rs`), the blitter (`blit.rs`) and hardware
sprites (`sprite.rs`) live under `front/` but belong to other chapters: [game-flow](game-flow.md),
[story-and-ending](story-and-ending.md), [frame-interrupts-input](frame-interrupts-input.md) and
[display-hardware](display-hardware.md).

Files owned:

- `engine/crates/sf-core/src/front/mod.rs` holds the framework. It has `Routine` and `Yield`,
  the call stack (`Front`, its private `Frame`) and `Cx`, the context a routine runs in: memory,
  input, the beam line, COP1LC, Paula writes, the jobs and loads reported for conformance.
  `Cx::busy`, `Cx::blit_wait`, `Cx::masked_job` and `Cx::unpack` move the beam for CPU work.
  `Cx::read_entry` and `table_entries` raise disk prompts from `file_table`. `Keyboard` is the
  front end's copy of CIA-A's serial register. `Front::power_on`, `Front::resume`,
  `Front::after_level`, `Front::level_over` and `Front::enter_slot` are the ways into the front
  end. `main_chain` reads the original's call stack off its memory. `RUNS_DURING_LOAD` and
  `LOAD_LEAVES` are what a load hands over from the original.
- `engine/crates/sf-core/src/front/title.rs` holds boot (`boot`, `level_tables`), the
  Accessing-disk overlay (`overlay_show`, `OverlayShow`, `OverlayEnd`), `Title` (`title_screen`,
  h0+$109e), `IntroAnim` (`title_intro_anim`, h0+$3902), `TitleFadeIn`, `TitleExit` and the
  eyes' blink (`eyes_init`, `blink`). It also holds the title's blits with priority
  (`blit_planes`, `stalled_past`, `blit_clear`, `blit_restore`, `blit_eyes`) and the title's
  resume points (`resume`, `cop1lc_of`).
- `engine/crates/sf-core/src/front/menu.rs` holds `Menu` (START GAME / OPTIONS, h0+$1406) and
  `Options` (h0+$16be: lives, game mode, saving the high scores, the level code). `draw` paints
  each option's choices as copper colour MOVEs, and `game_mode_tuning` sets the frog's tuning for
  each game mode.
- `engine/crates/sf-core/src/front/password.rs` holds level codes. `table`, `password_for` and
  `decode_password` are pure functions over the disk's table. `Entry` is `password_entry`
  (h0+$22ba), `check` is `password_check` (h0+$2432), and `draw_code` and `text` draw the
  digits into the options picture.
- `engine/crates/sf-core/src/front/hiscore.rs` holds `NameEntry` (`hiscore_name_entry`,
  h0+$2bba), with `rank`, `char_of` and `enter` (the score into `hiscore_table`, h0+$2f90).
- `engine/crates/sf-core/src/front/gameover.rs` holds `GameOverSeq` (h0+$74d0),
  `GameOverCheck` (h0+$749c), the GAME OVER object (`GAME_OVER`) and the level's handler run
  from the front end (`level_vbl`, `in_level_from`).
- `engine/crates/sf-core/src/front/attract.rs` holds `TopLoop` (`top_loop` from the title on),
  `AwardPages` (h0+$1e98), `HiscoreCredits` (h0+$20e0) and `HallOfTalent` (h0+$2e56), plus the
  credits' scroll blit (`scroll`, `scroll_races`, `text_line`). `Demo` is the attract demo's
  load and set-up.
- `engine/crates/sf-core/src/front/fade.rs` holds the palette fades. There are four records at
  h0+$157d8 (`RECORDS`, `Rec`). Their set-ups (`set_and_step`, `fade_in`, `fade_out`) and their
  vertical-blank steps (`step_set`, `step_in`, `step_out`) follow the original's code. Their
  68000 timing is `divu_cycles`, `fade_in_cycles` and `level_step_times`.
- `engine/crates/sf-core/src/front/anim.rs` holds the game's IFF ANIM player over the original's
  memory: `open` (h0+$ab02), `next` (h0+$ab82, DLTA operation 5) and `show_pointers`. Each call
  returns its 68000 cycles. [story-and-ending](story-and-ending.md)'s `ending.rs` is its only
  caller.
- `engine/crates/sf-core/src/front/common.rs` holds small shared routines. `FadeSetup` is a fade
  set-up between INTENA off and on, which can run past the frame's end. The others are
  `PaletteWait` (h0+$3ca2), `FireUp`, `Frames`, `MusicFadeWait` (h0+$1327a) and `MusicStart`
  (h0+$13126, `p41_init` taking 20 lines).
- `engine/crates/sf-core/src/front/data.rs` holds `FrontData`: hunk 0 relocated again, the
  title's files, the packed pictures and the overlay program (`OVERLAY_AT`), and `relocate`.
- `engine/crates/sf-core/src/front/hero.rs` holds `load_hero` (h0+$9172): `HERO` and its
  mirror image cut into attached hardware-sprite frames at h3+$4c0b0, with their table at
  h0+$19ddc.
- `engine/crates/sf-core/tests/front.rs` holds the level-code table's crate tests.

### Per screen

The mix column is the rough share of that screen's code by kind, judged as in §2.

| Screen | Code | Mix (amiga / game / data) | Mechanisms | Must preserve (§5) |
|---|---|---|---|---|
| Boot | `title::boot`, `level_tables`, `hero::load_hero` | 40 / 10 / 50 | Loaded hunk image, boot-built tables, hardware sprites, instant loads | Memory at the end of boot; the frog's sprite frames |
| Accessing-disk overlay | `title::overlay_show`, `OverlayShow`, `OverlayEnd` | 80 / 10 / 10 | A relocated overlay program with its own fade code, copper list, cycle-timed unpack | Its fade in and out on the beam; where the load ends |
| Title and intro | `title::Title`, `IntroAnim`, `TitleFadeIn`, `TitleExit`, `blink` | 55 / 40 / 5 | Copper list writes, blits with priority (stalls), sprite records, fades, RNG | Bounce, blink timers from the RNG, fire skipping, the 400-frame idle, fade timing |
| Menu and options | `menu::Menu`, `Options`, `draw`, `game_mode_tuning` | 45 / 50 / 5 | Copper colour MOVEs, sprite cursor, CPU-timed unpacks | The choices, the flashing, the options' effects, the frog's tuning per mode |
| Level codes | `password::Entry`, `check`, `table`, `draw_code`, `text` | 30 / 45 / 25 | Code read as data, CIA keyboard, plane text, copper colours | The 28 entries, first match wins, the entry's keys |
| Award pages, hall of talent, credits | `attract::AwardPages`, `HiscoreCredits`, `HallOfTalent`, `scroll` | 65 / 30 / 5 | Copper lists, a 49-frame unpack, the credits' scroll blit racing the beam | Page order and frame counts, abort keys, the scroll as the beam sees it |
| High-score entry | `hiscore::NameEntry`, `rank`, `enter` | 40 / 55 / 5 | Plane text, sprite cursor, CIA keyboard | The rank, the keys typed, the table's insertion |
| Game over | `gameover::GameOverSeq`, `GameOverCheck`, `level_vbl` | 55 / 45 / 0 | The level's handler and pass run from the front end, jobs across frames | The sign, two passes, the 350-frame wait, the way back to the title |
| Attract demo (set-up) | `attract::Demo`, `TopLoop` | 50 / 45 / 5 | Instant loads, level set-up on the beam | Which level and recording, the cycle back to the title |

### The attract demo, and what keeps it in step

The demo is not a recording of the picture. It is W1-1 played by the game with the joystick
replaced by a recording, so it stays in step only while everything the recording drives
behaves as on the original.

- **Its set-up is the front end's.** `attract::Demo` runs `top_loop`'s attract path (h0+$a8):
  the overlay, the demo's world and level loads (`crate::level::load::demo_load`), `overlays_end`,
  `demo_init` on the beam (`crate::level::exit::init_on_beam`, always W1-1) and `level_start`.
  The level's own loop then runs, and its exit fade goes back to the title
  (`crate::level::exit::LevelOver`).
- **Its play is the frog's.** `demo_play` in `engine/crates/sf-core/src/frog/physics.rs`, owned by
  [frog-and-items](frog-and-items.md), runs in the 50 Hz handler. It counts `demo_left` down a
  frame at a time and takes the next step of the recording at h0+$3ad8c when the count runs out.
  Fire, or the recording's `$ffff`, ends it. The recording is copied-in data
  ([copied-in-data](copied-in-data.md)).
- **What it depends on:**
  - **Frames.** The recording counts vertical blanks, not passes, so every frame boundary must
    fall where the original's does, from power-on through the title, the award pages, the hall
    of talent and the credits, whose lengths are all in frames.
  - **The 50/25 Hz split.** The frog moves in the handler and the objects in the pass
    ([frame-interrupts-input](frame-interrupts-input.md)). The original's pass ran late on 32 of
    721 demo frames (rust-engine FINDINGS 2026-10-11). In conformance that lag is an input
    (`Pacing::Late`), including in the run-alone check. In normal play the engine never lags
    (rust-engine DESIGN §2.1).
  - **The RNG.** It never moves in W1-1 (rust-engine FINDINGS 2026-10-13, T19), so the demo does
    not depend on the RNG state the title's eye blinks leave. The title does: `blink` draws from
    the RNG (`rng.range(0x4b)`), so the RNG state decides the title's timing up to the demo.
    The power-on seed comes from CIA-B's clock, and conformance feeds it as an input.
  - **Unknown: whether the demo, played with no lag as in normal play, still ends where the
    original's does.** No check runs it unpaced. To find out, run `front-attract` with the
    engine unpaced (`Pacing::Unpaced`) and compare its milestones with the original's.

### Level codes: game logic, encoded as the original encoded it

There is no algorithm. `password_table` (h0+$24dc) holds 28 entries of six ASCII digits, each
followed by a pointer to a routine of the form
`move.l #world,start_world; move.l #level,start_level; rts`. `password::table` reads the digits
and decodes each routine's two immediates. It checks the opcode words (`$23fc`) and drops an
entry whose routine has any other shape. The table is the player's disk's, so the codes are never
in the repo (rust-engine DESIGN §2.3). What must stay exact:

- the 28 entries and their order. `check` takes the **first** entry whose digits match, and a
  code that matches none leaves `start_world` and `start_level` as `Entry` cleared them (0, 0).
  The game then starts W1-1.
- the entry's keys. Raw key codes map through h0+$2e30 (ended by `$ff`) to characters at
  h0+$2e0a. Backspace (`$41`) starts again. Escape (`$45`), Return (`$44`), fire or the sixth
  digit end the entry. A key is consumed by clearing `key_down`.
- `password_for`, the first entry that starts a given world and level, which `Game::start_at`
  types to start a game where the app's "Continue" asks. The slot machine's jackpot reads the
  table itself (`password_for_level` in `engine/crates/sf-core/src/slot/rounds.rs`,
  [slot-and-bonus](slot-and-bonus.md)).

## 2. Kind mix

`amiga` 60, `game` 25, `data` 6, `shell` 9. Judged by reading each file and splitting its lines
by the parts below, weighted by each file's size (inline tests counted with their file). It is
an estimate, not a count.

- **`amiga`.** Most of `mod.rs`: the `Yield` waits on raster lines and fade flags, the beam line
  moved by `Cx::busy`, `Cx::blit_wait` (1.23 cycles per fetch slot under the display) and
  `Cx::masked_job`, the picture drawn from copper-list snapshots by line, the fades' colour
  writes timed channel by channel, the stall of a blit with priority, the carry of Paula writes
  past the frame's end, `Keyboard`. Also the copper-list and sprite-record writes in every
  screen, the title's and the credits' blits, the measured cycle constants
  (`OVERLAY_UNPACK_CYCLES`, `OPTIONS_UNPACK_CYCLES`, `HALL_UNPACK_CYCLES`, `P41_INIT_LINES`), the
  fade records' layout and their `divu.w` timing, and the ANIM player's instruction-by-instruction
  cycle counts.
- **`game`.** The order and content of the screens. This covers what the title, menu, options,
  code entry, name entry, game over and award pages do with input and how long each waits in
  frames, the high-score ranks and insertion, the game-mode choice, and the fade's visible
  stepping of each colour towards its target.
- **`data`.** `FrontData` and `relocate` (hunk 0 again, the title's files, the overlay), the
  level-code table read out of the executable, `KICKSTART_LOW_CHIP`, the bounce table and font
  read from hunk 0, the `game_mode_tuning` immediates, and `load_hero`'s conversion of `HERO`.
- **`shell`.** Conformance plumbing in `mod.rs`: `Front::resume` and `main_chain`, which rebuild
  the call stack from the original's memory, the `load_line`, `schedule`, `load_runs` and
  `resync` inputs, `anchored`, `RUNS_DURING_LOAD`, `LOAD_LEAVES`, and `Job` reporting. Also each
  screen's `resume` points and `engine/crates/sf-core/tests/front.rs`.

## 3. Mechanisms it uses

Terms are the [glossary](glossary.md)'s.

- **Byte-exact state at original addresses** ([memory-model](memory-model.md)). Every variable a
  screen touches is read and written by its h0 address (`START_WORLD`, `ATTRACT_MODE`,
  `OPT_LIVES`, the fade records, `hiscore_table`). `Cx::b` falls back on hunk 0 and hunk 2 as
  the disks put them there, so routines read the original's tables in place.
- **The loaded hunk image** ([copied-in-data](copied-in-data.md)). `FrontData::h0` is a second
  relocation of hunk 0. `password::table`, `table_entries` (which scans the code for
  `lea file_table+n,a2`) and the bounce table read it as data.
- **Boot-built tables** ([copied-in-data](copied-in-data.md)). `title::level_tables` and
  `hero::load_hero` build what `rng_seed_from_cia` and `load_hero` build.
- **Beam position and cycle charging** ([cpu-time-and-beam](cpu-time-and-beam.md)). The front
  end has its own beam model, separate from the level's `Clock`: `Cx::line` and `Cx::into`,
  moved by whole lines or cycles. Blits, unpacks and fade set-ups charge measured cycles, and a
  job that runs past the frame's end delays the next interrupt (`Yield::Masked`, `Cx::stall`).
- **The vertical-blank interrupt** ([frame-interrupts-input](frame-interrupts-input.md)).
  `Front::step` runs `vbl::run` before the main program and `vbl::end` after it. During game
  over it runs the level's handler instead (`gameover::level_vbl`). `Front::load_blanks` runs the
  handler on the frames a load takes.
- **The CIA keyboard** ([frame-interrupts-input](frame-interrupts-input.md)). `Keyboard::feed`
  sends one byte per key change. `Front::after_level` hands the register over from the level so
  ESC is not read twice. Code entry and name entry read `key_down` as the poll leaves it.
- **The copper run and bitplane fetch** ([display-hardware](display-hardware.md)). Each screen
  shows itself by writing a copper list (title `h2+$7158`, awards `h2+$6110`, hall `h2+$6ff0`,
  blank `h2+$6fd8`) and setting `cop1lc`. The picture is rendered from the snapshots of that list
  by line.
- **The blitter and hardware sprites** ([display-hardware](display-hardware.md)). The title's
  restore and eyes, and the credits' scroll, are `blit::Blit`s. The eyes, the options cursor and
  the name cursor are sprite records updated through `sprite::update`.
- **Paula writes** ([sound-chip](sound-chip.md), [sound-driver](sound-driver.md)). The
  `INTENA`/`DMACON` writes of fade set-ups and blits go out in beam order with the sound's
  (`in_beam_order`, `anchored`). `MusicStart` and `MusicFadeWait` drive the sound driver.
- **Disk prompts and swaps** ([game-flow](game-flow.md), [disks-and-assets](disks-and-assets.md)).
  `Cx::read_entry` raises `DiskPrompt::Insert` when a file's disk is not the one last asked
  for. The engine's loads are instant, and `Cx::loaded` reports where the original's would end
  (`LoadEnd`).

## 4. What depends on it

- **[game-flow](game-flow.md).** `chain.rs`'s `Chain` and the screens between levels are
  `Routine` variants run by this chapter's executor, and `Game` switches between `Front` and the
  level scene through `Front::after_level`, `Front::level_over` and `Front::enter_slot`.
- **[story-and-ending](story-and-ending.md).** `ending.rs` runs on `Front` and calls
  `anim::open`, `anim::next`, `anim::show_pointers`, the fades, `MusicFadeWait` and
  `hero::load_hero`. The story hands over to `Front::power_on`.
- **[slot-and-bonus](slot-and-bonus.md).** The slot machine's routines are `Routine` variants
  too (`Routine::Slot*`), and they use `Cx`, the fades and the overlay. The jackpot's code comes
  from the same `password_table`.
- **[level](level.md).** Game over, the level's exit, restart and tally (`crate::level::exit`,
  `crate::level::tally`) are run on the front end's stack, and the level scene is entered from
  `Routine::LevelLoop`.
- **[app](app.md).** It reaches the title, the menu and the code entry through `Game`'s input.
  `password_for` backs its start-at-a-level feature (`Game::start_at` types the code).
- **[verification](verification.md).** Every power-on case starts in `Front::power_on`, and
  every case that syncs mid-front-end starts in `Front::resume` from the original's call chain
  (`main_chain`). The `jobs`, `load`, `resync` and `beam_races` reports in `FrontOut` are how
  conformance compares frames in which the engine did at once what the original spread across
  frames.

## 5. What a native rewrite must preserve

Faithful stays the bar. Each row is behaviour the original shows, and the check that holds the
engine to it today. Unless a path is given, the tests named are in
`engine/crates/sf-conform/tests/front.rs` and the cases are the files of that name under `engine/conform/cases/`.

| Behaviour to keep | Check that proves it |
|---|---|
| Boot to the title, frame for frame: the overlay fades out, the eyes bounce in, the title fades in and waits, with every gameplay variable, the picture and the sound equal at every frame. | `power_on_to_the_title_matches_the_original` (case `front-boot`). |
| The frog's sprite frames as `load_hero` leaves them at h3+$4c0b0 (`HERO` and its 32-pixel-strip mirror). | No state check: `hero_frames` is a `presentation` variable (`sf_core::layout::presentation`), left out of the faithful state check and compared only under `--strict`. The picture check of every level case judges the frog as drawn, e.g. `engine/conform/cases/code-w1-1.toml`. No check covers `load_hero`'s second call at the end sequence's end apart from the case `ending`. |
| The title intro: fire (not the mouse or a key) skips it, at the bounce, on the left, in the blink and in the wait, and the frame the fade-in starts on follows from the restore blit's length under six planes. | `fire_skipping_the_title_intro_matches_the_original` (`front-skip-blink`); `front_skip_bounce`, `front_skip_left`, `front_skip_wait`, `front_noskip` (`engine/crates/sf-conform/tests/cases.rs`). |
| A blit with priority that runs past the frame's end holds the CPU, so the next interrupt comes about line 30 of the next frame (`STALL_VBL_LINE`). | `front_options` (`cases.rs`, the options screen's exit, where it was measured). No unit test. |
| The eye blinks draw from the RNG (`rng_range($4b)+$4b`), and the title idles 400 frames before attract mode. | Case `title-rng` (the RNG compared at frame 4410, run by `engine/crates/sf-conform/tests/faithful.rs` and `runner.rs`); `the_attract_cycle_matches_the_original` (case `front-attract`). |
| The menu: up/down between START GAME and OPTIONS, the flash table; the options: lives, game mode (and its frog tuning), save high scores, each drawn as the original draws them. | `front_options`, `front_options_all` (`cases.rs`). The tuning written per mode is compared as state only; its effect on play is covered by the level cases run in the default mode. |
| Level codes: every one of the 1 000 000 six-digit codes starts where the original's `password_check` starts it. Each world and level has a code. The first match wins, and a routine of another shape is not a code. | `every_six_digit_code_starts_where_the_original_starts_it`, `each_world_and_level_has_a_code_that_starts_it`; `a_code_starts_where_its_routine_says_and_the_first_match_wins`, `an_entry_whose_routine_is_not_the_two_moves_is_left_out` (`engine/crates/sf-core/tests/front.rs`). |
| The code entry: keys through the original's key table, Backspace restarts, an unknown full code leaves the start at W1-1, and the digits are drawn into the picture late in the frame. | `the_level_code_entry_matches_the_original` (case `front-password`); `a_level_code_starts_the_game_at_w3_1_as_the_original_does` (`engine/crates/sf-conform/tests/chain.rs`); the `code-*` cases. |
| The attract cycle: title idle, award pages (300 frames each), hall of talent (20 lines in, 350 frames, 20 out), credits (a line every second frame), the demo and back to the title, in the original's frames. | `the_attract_cycle_matches_the_original` (case `front-attract`, 5 200 frames); `attract_demo_3` (three cycles, ignored as slow, `cases.rs`). |
| The credits' scroll shows each word new only if the blitter wrote it before the fetch read it. | `front-attract`'s picture check. The rows where blit and beam meet (about row 229) are counted as beam races (`scroll_races`, owner 2026-10-13), so those few pixels are tolerated rather than proven. |
| Fire or the mouse button cut an award page short; fire, the mouse button or Escape abort the credits; fire ends the demo and the next cycle moves `demo_index` on. | `fire_ends_the_attract_demo_as_the_original_does` (case `attract-abort`). No case aborts an award page or the credits. An abort in the credits sets `attract_abort`, and the title's path after it is not ported (`Cx::unported("title_screen after an aborted attract screen")`). |
| The demo replays the W1-1 recording against the game and ends where the original's ends, given the original's lag frames as input. | `the_attract_cycle_matches_the_original` and `attract-demo-3`, including check 4 (run alone, rust-engine DESIGN §2.1). Without the original's lag (normal play) it is unchecked (§1). |
| The high-score entry: rank text, eight places, letters, space, digits, Backspace, fire/Return/Escape to end; the score inserted at its place and the rest moved down; the score cleared. | `the_hiscore_name_entry_after_game_over_matches_the_original` (case `hiscore-entry`), `the_hiscore_name_entry_after_the_end_sequence_matches_the_original` (case `hiscore-ending`). |
| Game over: the sign at the camera, two passes on the level's handler with the first half past the frame's end, up to 350 frames for fire, then the exit fade and the title's reload. | `game_over_w1_1` (`engine/crates/sf-conform/tests/game_over.rs`); `game_over_esc_w1_1`, `game_over_fire_w1_1`, `game_over_timeup_w1_1` (`cases.rs`); `game_over_then_w2_1_s_code_continues_in_w2_1` (`chain.rs`). |
| Fades: four records, the colour stepped per channel by the original's arithmetic (`divu.w #$f`), a step every `speed` frames, a set-up with interrupts off whose `divu` time decides whether it runs into the next frame. | `divu_takes_the_68000s_time`, `the_bus_model_writes_each_changed_channel_then_acknowledges`, `steps_slow_down_inside_the_window` (`engine/crates/sf-core/src/front/fade.rs`); `a_fade_out_reaches_black_and_says_so` (`engine/crates/sf-core/src/front/mod.rs`); every case through a fade (`front-boot`, `frog-pit-w1-1`). |
| The ANIM player decodes `END1` and `END2` exactly, and its cycle counts place each decode's end in the original's frame. | `the_anim_player_draws_every_frame_as_the_decoder_does`; `the_end_sequence_matches_the_original` (case `ending`) for the timing. |
| Unpacks (options picture 12 frames, eyes 12, `HPIC` 49, overlay 3302 lines) end on the original's line, and the picture shows the data arriving from its end at an even pace. | `front_options`, `front-attract`, `game-whole` (`engine/conform/cases/game-whole.toml`, the overlay before the final level). The even-pace picture is an approximation the picture check tolerates, not a model of `atn_decompress`'s order. |
| The keyboard register: a byte per change, `$ff` once handled, the power-on `$7e` released, handed over from the level so ESC is read once. | `the_keyboard_sends_a_byte_per_change_and_reads_ff_once_handled` (`engine/crates/sf-core/src/front/mod.rs`); `frog_esc_w1_1` (`engine/crates/sf-conform/tests/frog_cases.rs`). |
| Paula writes in beam order, never going back, at the original's positions when conformance gives them. | `writes_in_beam_order_never_go_back`, `writes_take_the_originals_positions_when_given_for_all_else_their_lines` (`engine/crates/sf-core/src/front/mod.rs`); the sound check of every front-end case. |
| The disk asked for at each load, as the original asks. | `disk_prompts_are_raised_as_the_game_asks_and_need_no_answer` (`engine/crates/sf-conform/tests/flow.rs`). |

## 6. Native sketch (non-binding)

This sketch is non-binding: designing the rewrite is the later plan's job.

Each screen could become a plain state machine over named fields (`TitleState { eyes_x,
bounce_at, blink_timers, idle_left }`) stepped once per frame, with the frame counts, input rules
and table contents kept exactly. The copper list and sprite-record writes would become a
per-frame screen description: which picture, which palette, where each sprite is. The renderer
would draw that directly. The parts where the original's timing shows in the picture would need
an explicit "line this takes effect on" value in that description: a fade's colours changing
mid-frame, a blit overtaken by the beam, the frame a fade-in starts after a skip, an unpack's
progress. These would be derived from measured lengths rather than from cycle-charged CPU work.
The fade engine is small and could stay as it is, with its records as a struct rather than
bytes at h0+$157d8. The level-code table could be decoded once into `CodeEntry`s, which
`password::table` already does, and the entry would keep its key table.

Two things the rewrite meets first. `Front::resume` rebuilds the call stack from the original's
return addresses, so conformance can start mid-screen. A native front end needs a mapping from
the original's call chain to its own states, or the checks need to start only at power-on and
at screen boundaries ([verification](verification.md)). And `Routine` is also the flow
executor for [game-flow](game-flow.md), [slot-and-bonus](slot-and-bonus.md) and the level's
exit, so splitting it has to be planned with those chapters.

## 7. Reuse

Rating: **`superfrog-only`**. 6 247 lines (`python tools/architecture.py totals`).

The screens, their order, the addresses, the measured timings and the tables are this game's.
For another Amiga game only the method carries: port the front end routine by routine as
resumable state machines over the original's memory, yield on the original's waits, and draw
from copper-list snapshots by line. Pieces that are generic or nearly so:

- **The routine executor** (`Routine`/`Yield`/`Frame` and `Front::step`'s main-program loop):
  the approach carries, but the variant list and `Cx`'s fields are Superfrog's (`big-changes`).
- **`Keyboard`**: CIA-A's serial register behaviour is the Amiga's (`as-is`), though another game
  may use the keyboard interrupt instead of polling.
- **`fade::divu_cycles`**: the 68000's `divu.w` timing (`as-is`). The fade records themselves are
  this game's code. Another Team 17 game may share the fade routines (unchecked).
- **`anim.rs`**: an IFF ANIM operation-5 player. The decode is standard, but the record address,
  buffers and cycle counts are this game's player (`small-changes`).
- **`main_chain`**: reading a call chain off a 68000 stack is generic, but its code range and
  vector are Superfrog's (`small-changes`).

Together these are a few hundred lines. They do not change the chapter's rating.

## 8. Sources

- [docs/13](../13-front-end.md) (scene flow, timing the engine keeps, the attract demo, game
  over, the keyboard, level codes); [docs/10 "ANIM"](../10-asset-formats.md) for the ANIM format.
- [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.1 (faithful, check 4, normal play
  never lags), §2.3 (the disks are the source).
- [rust-engine FINDINGS](../../plans/rust-engine/FINDINGS.md): demo lag (2026-10-11), the RNG in
  W1-1 (2026-10-13), the credits' beam race (2026-10-12, 2026-10-13).
- Code: the owned files above; `engine/crates/sf-core/src/frog/physics.rs` (`demo_play`),
  `engine/crates/sf-core/src/level/exit.rs`, `engine/crates/sf-core/src/level/load.rs`
  (`demo_load`), `engine/crates/sf-core/src/front/vbl.rs`, `engine/crates/sf-core/src/rng.rs`.
