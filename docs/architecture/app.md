+++
id = "app"
title = "The app"
tier = "component"
depth = "light"
kind = "shell"
mix = { amiga = 10, game = 0, data = 0, shell = 90 }
reuse = "small-changes"
owns = [
  "engine/crates/sf-app/",
]
+++

# The app

## What it is

`sf-app` is the `superfrog` binary, the macOS app around the pure game
([rust-engine DESIGN §3.1](../../plans/rust-engine/DESIGN.md), §3.2). It owns the window
(winit), the GPU picture and its filters (wgpu), the audio device (cpal), gamepads (gilrs), the
native menu bar (muda) and panels (rfd, AppKit), and every file the game keeps:

- **Run loop**: `engine/crates/sf-app/src/main.rs` (`App::start`, `App::step`,
  `ApplicationHandler::about_to_wait`), `engine/crates/sf-app/src/pacing.rs` (`Pacer`,
  `Presenter`), `engine/crates/sf-app/src/audio.rs` (`Audio`, `Clock`).
- **Picture**: `engine/crates/sf-app/src/window.rs` (scale, place, widescreen size),
  `engine/crates/sf-app/src/video.rs` (`Video`, `picture_rgba`), the filter shaders
  `engine/crates/sf-app/src/crt.wgsl`, `engine/crates/sf-app/src/port.wgsl`,
  `engine/crates/sf-app/src/crt_lottes.wgsl`, `engine/crates/sf-app/src/crt_geom.wgsl`,
  `engine/crates/sf-app/src/crt_easymode.wgsl`.
- **Input**: `engine/crates/sf-app/src/input.rs` (`OriginalInput`, `KeyMap`, `map_input`,
  `Input`), `engine/crates/sf-app/src/pad.rs` (`PadMap`, `Gilrs`), the Controls window
  `engine/crates/sf-app/src/controls_window.rs`.
- **Menus and dialogs**: `engine/crates/sf-app/src/menu.rs` (`Command`, `App::on_command`),
  `engine/crates/sf-app/src/import.rs` (first-run disk import, `Ui`, `NativeUi`, `ScriptedUi`),
  `engine/crates/sf-app/src/start_level_window.rs`.
- **Files**: `engine/crates/sf-app/src/settings.rs`, `engine/crates/sf-app/src/progress.rs`,
  `engine/crates/sf-app/src/states.rs` (save states), `engine/crates/sf-app/src/levelfile.rs`
  (`.sflevel` export and play, [docs/14](../14-sflevel-format.md)),
  `engine/crates/sf-app/src/home.rs`, `engine/crates/sf-app/src/lock.rs`,
  `engine/crates/sf-app/src/atomic_write.rs`, and `engine/crates/sf-app/build.rs`, which hashes
  the game crates' sources into the build identity save states are tagged with.
- **Bug recorder UI** ([plans/bug-recorder](../../plans/bug-recorder/DESIGN.md)):
  `engine/crates/sf-app/src/recorder.rs` (`Recorder`, `Replay`, the zip),
  `engine/crates/sf-app/src/recording_bar.rs`, `engine/crates/sf-app/src/save_window.rs`.
- **Scripted runs**, the test layer `sf-e2e` drives: `engine/crates/sf-app/src/args.rs` (the
  seatbelts `--timeout`, `--max-frames`), `engine/crates/sf-app/src/script.rs` (`--script`),
  `engine/crates/sf-app/src/capture.rs` (`--capture`), and `--pacing`/`--replay` in `main.rs`.

## Kind mix

`amiga` 10, `game` 0, `data` 0, `shell` 90.

- **`amiga`** (about 1,050 of 10,399 lines): the four monitor filters and their RetroArch shim
  (the five `.wgsl` files, about 650 lines); `pacing.rs`, stepping on a PAL frame of Paula clocks
  (135); in `input.rs` the original's inputs, `OriginalInput`, `AMIGA_KEYS` (the Amiga raw key
  codes the game reads) and the mouse as port 1's button (about 150); the Paula hand-off in
  `App::step` (about 40); `window::GAME`, the original's largest display window, and
  `picture_rgba`'s 12-bit colour (about 30); and the two places the app reads or writes the
  game's memory by original address (below).
- **`shell`**: everything else. Window, GPU, device, menus, panels, settings, progress, save
  state files, level files, the recorder's UI and zip, scripts, captures, the seatbelts and their
  tests. The widescreen setting is counted here, as [display-hardware](display-hardware.md)
  counts the renderer's half of it.
- **`game`** and **`data`**: none. The game's rules and the original's tables are all behind
  `sf_core::Game`; the app only chooses which `Start` to make and which input to feed.

Judged by reading each file and attributing whole items: the shaders, `pacing.rs` and the named
items of `input.rs`, `window.rs`, `video.rs` and `main.rs` are counted `amiga`, the rest `shell`.
The unit tests in each file follow the code they test.

## How the app drives `Game`

What a native core must still offer the app is exactly this interface; everything below it is the
other chapters' business.

1. **Start.** `Game::with_display(display, Start::PowerOn { seed })` with `DisplayData::new` over
   the cached `GameData` (`App::start`, `App::power_on` in `menu.rs`), or `Game::start_level`
   from Game ▸ Start Level… and `--replay`, or `Game::load_with` from a save state
   (`states::load`). The seed is `Rng::seeded` over the host clock (`host_seed`), standing where
   the original's `rng_seed_from_cia` reads CIA-B's time-of-day counter
   (`engine/crates/sf-core/src/rng.rs`). `Game::set_view` gives the widescreen setting.
2. **Pace.** `about_to_wait` asks `Pacer::due` how many whole PAL frames the clock has passed
   (a frame is `LINES_PER_FRAME` × `CLOCKS_PER_LINE` = 313 × 227 Paula clocks at
   `PAULA_CLOCK_PAL`, 49.92 Hz) and calls `App::step` once for each. The clock is the audio
   device's samples played (`Clock::Device`), with a 40 ms lead so the queue never runs dry, or
   the wall clock when there is no device or it fails (`Clock::Wall`). A modal panel is followed
   by `Pacer::skip_to`, so the game resumes rather than catching up.
3. **Step.** `App::step` turns the frame's host events into one `FrameInput` (`Input::frame`, the
   pure `map_input`) and calls `Game::step(input) -> FrameOut`. Under `--pacing` it plays a
   `sf_core::feed::FeedFrame` instead, one or more game steps as the original's lag frames ran
   them ([cpu-time-and-beam](cpu-time-and-beam.md)).
4. **Sound.** One `sf_paula::Paula::booted(rate)` per game. Each step it first mixes
   `Game::take_load_paula` (the vertical blanks of a load the engine did at once, played unheard),
   then each `FrameOut::mixed()` (`unheard`, then `paula`) through `Paula::frame`, which reads the
   samples out of `Game` as `ChipMemory` (`Game::chip_word`): the game's chip RAM at original
   addresses. The samples go to `Audio::push`. See [sound-chip](sound-chip.md).
5. **Picture.** The last `FrameOut::picture` becomes RGBA through `to_rgb12` (`picture_rgba`),
   is handed to `Presenter::completed`, and is uploaded and drawn at the next display refresh;
   `Game::render(view)` gives frame 0 and the picture after a start. See
   [display-hardware](display-hardware.md).
6. **Around the step.** `Game::progress` after each step (Continue from World N,
   `progress.rs`); `Game::save` for save states and `Game::unported` for the capture;
   `FrameOut::disk_prompt` is not read, since the cache holds every disk
   ([game-flow](game-flow.md)).

So a native core must still give the app: a start from power-on with a seed, from a level with
chosen values and from saved bytes; one step per PAL frame on a `FrameInput` of joystick bits and
Amiga raw key codes; each frame's Paula writes, and the chip memory they read samples from, or a
replacement sound hand-off (a rewrite of [sound-chip](sound-chip.md)); a 12-bit picture for a
view; progress; and the two address hooks below, or replacements for them.

## Where it touches the Amiga-shaped parts

- **PAL pacing** ([glossary](glossary.md)). The app's own imitation: the Amiga's PAL frame rate
  kept on any display, music and picture paced by one clock, never an interpolated frame
  (rust-engine DESIGN §2.5).
- **Paula writes and chip memory.** The app owns the mixer instance and feeds it the engine's
  writes over the engine's chip RAM, so the [sound-chip](sound-chip.md) model and the
  [memory-model](memory-model.md)'s chip RAM are both in its path. `Paula::set_frame_lines` is
  never called: every frame is mixed as 313 lines though the game keeps stretches of 312-line
  frames (`Pacer::set_frame_lines` is likewise test-only).
- **The joystick and keyboard.** `FrameInput` is the original's inputs: the port-2 joystick's
  directions and fire, the left mouse button as port 1's button (`Joy::PORT1_BUTTON`, which the
  crack intro waits for), and the Amiga keys by raw code. How the game reads them is
  [frame-interrupts-input](frame-interrupts-input.md).
- **The display window and colour.** `window::GAME` is 334 × 261, the largest display window the
  game opens (DIWSTRT `$2779`); every other screen is centred in it. Colours are OCS 12-bit,
  widened by 17. The picture is drawn with square pixels at a whole scale; the PAL monitor's
  pixel shape is not modelled.
- **The 4:3 picture and widescreen.** By default the app shows the original's 4:3 picture.
  Widescreen (`window::view`, `window::extra_columns`) asks the renderer for 65 more columns each
  side; it is the app's extension, not the Amiga's, and the game steps the same state in both.
- **CRT filters.** View ▸ Filter imitates a monitor after integer scaling: Basic (`crt.wgsl`, our
  own scanlines, aperture grille and vignette) and WGSL ports of libretro's `crt-lottes`,
  `crt-geom` and `crt-easymode`, run through `port.wgsl`'s copy of what RetroArch hands a
  single-pass shader. The one departure from the ports is `port.wgsl`'s quarter-pixel nudge at
  2×, where the shipped shaders draw no scanlines.
- **Original addresses in the app.** Two hooks reach past `Game`'s interface into the
  [memory-model](memory-model.md): a script's test-only `N: exit` pokes `level_state`
  (`sf_core::layout::h0(0xa06)`) to 1 as reaching a level's exit does (`App::step`), and Start
  Level logs `frog_lives`, `score`, `frog_hearts`, `opt_game_mode`, `frog_spud_on` and
  `frog_flight` read through `LAYOUT` and `Game::peek`, so an `sf-e2e` test can see the values
  arrived (`App::start_level` in `menu.rs`).
- **Save states** are `Game::save`'s bytes, the byte-exact state included, tagged with
  `home::build` (`build.rs`) and `sf_assets::cache_hash`; a state from another build is refused
  ([game-flow](game-flow.md)).

### What a native rewrite must preserve

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| PAL pacing | One step per whole elapsed PAL frame (313 × 227 Paula clocks), 499–500 steps in 10 s at any host rate; the wall clock keeps the same pace with no audio device | `ten_seconds_of_audio_clock_is_499_or_500_pal_frames`, `steps_come_whole_and_only_once_their_frame_has_elapsed` (`engine/crates/sf-app/src/pacing.rs`); `max_frames_ends_the_run_by_itself_with_status_0_at_the_pal_rate` (`engine/crates/sf-e2e/tests/smoke.rs`); `with_no_audio_output_the_game_runs_silently_at_the_same_pace` (`engine/crates/sf-e2e/tests/persist.rs`); `the_app_soaks_50_000_frames_without_a_fault_or_drift` (`engine/crates/sf-e2e/tests/whole_game.rs`, ignored: 17 min) |
| No interpolated frame | Each refresh shows the newest whole stepped frame | `the_presenter_shows_the_newest_whole_frame_at_every_refresh` (`pacing.rs`); `crt_drops_no_frame_over_3000_frames` (`engine/crates/sf-e2e/tests/view.rs`) |
| Short frames | The game's 312-line frames are 50.08 Hz, and Paula's frame is that long | No check: the pacer and the mixer run every frame as 313 lines, and only `short_frames_run_a_little_faster` (`pacing.rs`) tests the pacer's setter. Whether it is audible or visible is unknown; see [sound-chip](sound-chip.md) |
| Paula hand-off | The app's sound is the mixer's output for the engine's writes over its chip RAM, load blanks first; heard where the original's is | `the_captured_audio_is_the_mixer_s_output_for_the_same_writes` (`smoke.rs`); `the_app_paced_from_power_on_through_the_boot_loads_is_the_run_alone` (`whole_game.rs`, which compares the app's loudness frame by frame with the original's through `heard_where_the_original_is`) |
| The original's inputs | Keys and pads reach the game only as joystick bits and Amiga raw codes, whichever device fed them; a tap inside one frame is seen once; the mouse is port 1's button | `arrows_and_space_are_the_joystick_and_letters_are_amiga_keys`, `a_tap_inside_one_frame_is_seen_once`, `the_mouse_is_port_1_s_button_and_unmapped_keys_do_nothing`, `map_input_key_and_pad_held_together` (`engine/crates/sf-app/src/input.rs`); `arrow_keys_and_fire_move_the_frog_in_w1_1_as_the_same_input_does_headless_and_progress_is_saved` (`persist.rs`) |
| Display window and colour | Every screen at the window's scale, centred in 334 × 261; the frames shown are the engine's | `every_screen_shows_at_the_scale_its_window_opened_at` (`engine/crates/sf-app/src/window.rs`); `the_title_and_a_level_both_show_at_the_scale_chosen`, `at_1x_2x_and_3x_the_window_is_that_multiple_and_the_picture_fills_it_unsmeared` (`engine/crates/sf-e2e/tests/window.rs`); `the_first_frame_is_power_on_and_frame_400_is_the_engine_s` (`smoke.rs`) |
| 4:3 and widescreen | The 4:3 picture is the original's; widescreen keeps its centre and the game's state | `widescreen_mid_level_keeps_the_centre_and_the_game_of_the_4_3_run` (`view.rs`) and the rows in [display-hardware](display-hardware.md) |
| CRT filters | A filter changes only the scaled picture, never the game's frame; Basic stays legible at 1× | `every_filter_changes_only_the_scaled_picture_and_basic_stays_legible_at_1x` (`view.rs`). No check compares the three ports with RetroArch's own output |
| The original's lag frames (`--pacing`) | Fed the original's pacing, the app's frames are `sf-conform`'s engine run alone, picture and state hash for hash | `the_app_paced_from_power_on_through_the_boot_loads_is_the_run_alone`, `the_app_paced_through_the_whole_game_is_the_run_alone` (ignored: 75 min) (`whole_game.rs`) |
| Power-on seed | Reset is the same power-on as a fresh launch with the same seed | `reset_from_a_script_is_power_on_and_the_frames_and_sound_then_equal_a_fresh_launch` (`engine/crates/sf-e2e/tests/menus.rs`). Nothing checks the seed's source; the host clock stands in for CIA-B's counter |
| Address hooks | `exit` ends a level as reaching it does; Start Level's values are in the game at its first frame | `engine/crates/sf-e2e/tests/recorder_app.rs` (`a_recording_goes_on_through_the_level_s_end_the_tally_and_into_the_slot_machine`) and `engine/crates/sf-e2e/tests/recorder_start_level.rs` use them. A native core without original addresses needs a named setter for the level's exit and a named reader for those six values |
| Save states | A state loaded and run on gives the frames of never having saved; another build's is refused | `a_state_saved_mid_level_and_loaded_after_playing_on_shows_the_saved_frame_and_goes_on_from_it`, `a_state_from_another_build_is_refused_with_both_builds_named_and_the_game_carries_on` (`engine/crates/sf-e2e/tests/states.rs`) |

## Reuse

**`small-changes`**, 10,399 lines (`python tools/architecture.py totals`).

Most of the app carries to another Amiga game unchanged: the pacer and audio clock (any PAL
game; an NTSC one changes the frame length), the window, scaling and filters, the input mapping
machinery and pads, settings, the lock, atomic writes, save state files, the recording bar,
`--script`, `--capture` and the seatbelts. What changes, each local:

- **Names and the title**: "Superfrog" in the window, the menus and the app-data folder
  (`home.rs`), the build identity string (`build.rs`).
- **The disk import**: the number of disks and the messages naming them (`import.rs`); the
  decision itself is `sf_assets::decide` ([disks-and-assets](disks-and-assets.md)).
- **Menus with game content**: Continue from World N and `progress.toml`'s world and level
  (`progress.rs`, `menu::continue_world`), Export Level's level list (`menu::export_levels`), the
  Start Level window's lives, points, energy, difficulty, Spud and Wings
  (`start_level_window.rs`), and the matching `start-level` and `form` script lines.
- **The level-file format**: `.sflevel` holds Superfrog's per-level files (`levelfile.rs`,
  [docs/14](../14-sflevel-format.md)); another game needs its own sections.
- **The inputs and screen size**: `AMIGA_KEYS` lists the keys this game reads, `window::GAME` is
  this game's largest display window, and widescreen needs the other game's renderer to widen.
- **The address hooks**: the `exit` poke and Start Level's logged variables name Superfrog's
  addresses.

Judged by these files' sizes, about 1,700 lines (16%) are Superfrog's: chiefly
`start_level_window.rs`, `levelfile.rs`, `progress.rs`, and parts of `menu.rs`, `script.rs`,
`recorder.rs` and `import.rs`. That is larger than the "sf-app shell ~1.7k" of the 2026-10-15
estimate in DESIGN §1 because the app has since grown the bug recorder and its windows, which
are mostly generic.

## Sources

- [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.5 (timing), §2.6 (input), §2.7
  (display options), §2.8 (save states), §2.9 (what persists), §2.10 (level override), §2.11
  (unhappy paths), §3.1–§3.5.
- [bug-recorder DESIGN](../../plans/bug-recorder/DESIGN.md) for the recorder, the recording bar
  and the save window.
- [docs/12](../12-display.md) for the display window, [docs/11](../11-sound.md) for the sound,
  [docs/14](../14-sflevel-format.md) for `.sflevel`.
- The code under `engine/crates/sf-app/` and the tests under `engine/crates/sf-e2e/tests/`.
