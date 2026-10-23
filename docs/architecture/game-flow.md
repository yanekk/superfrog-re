+++
id = "game-flow"
title = "Game flow"
tier = "subsystem"
depth = "deep"
kind = "shell"
mix = { amiga = 30, game = 20, data = 5, shell = 45 }
reuse = "superfrog-only"
owns = [
  "engine/crates/sf-core/src/lib.rs",
  "engine/crates/sf-core/src/game.rs",
  "engine/crates/sf-core/src/scene.rs",
  "engine/crates/sf-core/src/rng.rs",
  "engine/crates/sf-core/src/recording.rs",
  "engine/crates/sf-core/src/savestate.rs",
  "engine/crates/sf-core/src/front/chain.rs",
  "engine/crates/sf-core/Cargo.toml",
  "engine/crates/sf-core/tests/savestate.rs",
  "engine/crates/sf-core/tests/start_level.rs",
]
+++

# Game flow

## 1. What it is

The top of the game: the one object every caller holds, which part of the game is running, the
fixed order the original goes through worlds and levels, its random numbers, and the two ways a
game is written to a file (save states and bug recordings).

- **`Game`, the decision function** (`engine/crates/sf-core/src/game.rs`; rust-engine
  [DESIGN §3.3](../../plans/rust-engine/DESIGN.md)). A function of the `GameData`, its `Start` and
  the inputs, and nothing else. `Game::new` (or `Game::with_display`, sharing one `DisplayData`)
  takes a `Start`: `PowerOn { seed }`, `Disk3 { seed }` (the story intro first), `Level(LevelId)`,
  and for conformance `Transplant(AmigaMemory)` and `Resume { memory, cpu }`. `Game::step` is one
  PAL frame of normal play; `Game::step_paced` is the same frame on the original's schedule
  (`Pacing`); `Game::step_unseen` skips drawing a level frame, for the route planners. Each returns
  a `FrameOut`: the `Picture`, the frame's Paula writes, and what conformance needs about work the
  engine did at once (`load`, `resync`, `jobs`, `beam_races`, `unheard`, `disk_prompt`).
  `Game::render` draws the current state for a `View`; `Game::probe` reads every modelled variable
  at its original address (from the story's own layout while the intro runs); `Game::save` and
  `Game::load` are in `engine/crates/sf-core/src/savestate.rs`. `step_body` hands the frame to the
  story intro's program (`front::Story`), the front end's (`front::Front`, which also runs the slot
  machine and everything between levels) or the level's main program (`Scene::step`), and moves
  between them as the original does: the intro hands over to a disk-1 power-on, the front end hands
  a level over once `level_frame_loop` is entered, and the level hands back to the front end when
  the loop returns (`Front::level_over`).
- **Starting a game the player's way.** `Game::start_at` (the app's "Continue from World N",
  rust-engine DESIGN §2.9) and `Game::start_level` (the bug recorder's Start Level dialog,
  [bug-recorder DESIGN §2.1](../../plans/bug-recorder/DESIGN.md)) switch on and type the level code
  on the options screen as a player would (`Game::type_code_with`, timed by `CODE_TIMES`); no
  shortcut into the level exists. `start_level` then runs unseen to the level's first frame and
  writes the dialog's score, hearts, spud and wings. `Game::location` names where play is: the
  frog's cell in a level, or a screen's name (`recording::SCREENS`).
- **Scenes** (`engine/crates/sf-core/src/scene.rs`). `Scene` is which program runs the frame:
  `Placeholder`, `Level`, `Bonus(LevelId)`, `Front`, `Slot { coins_in }`, `Story`. `SceneKind` is
  a different thing: the screen a frame shows, told from the copper list it starts with
  (`SceneKind::classify`), described in §3.
- **The fixed sequence** (`engine/crates/sf-core/src/front/chain.rs`;
  [docs/08 "Scene flow"](../08-executable-map.md#scene-flow)). `Chain` runs the original's
  `top_loop` game path (from h0+$e6) by reading its instructions out of hunk 0 and calling the
  engine's routine for each `jsr` target (`target`, `Chain::dispatch`): `access_overlay_show`,
  `world_load_wN`, `level_resume_jump`, `level_load_wN_lM`, `overlays_end`, `level_init_wN`,
  `level_start`, `level_frame_loop`, `level_exit_fade`, `game_over_check`, the `bmi` back after a
  death, `level_tally`, and after a world's fourth level `end_world_pic_wN` and `music_fade_wait`;
  the bonus and final levels' own loads and inits; the end sequence. It follows `jsr`, `tst.l`,
  `tst.w`, `lea`, `move.w` and `bra`/`bne`/`beq`/`bpl`/`bmi`, and records anything else as
  unported. `game_start_at` checks the level code and jumps to the start world's entry, keeping
  the level's call in `resume_target` for `level_resume_jump`. The screens between levels are here
  too: `WorldLoad`, `WorldIntro` (the loading picture with the frog's walk, `walk_setup`,
  `exit_walk_vbl`, `walk_step`, `clouds`), `LevelLoad`, `MapShow` and `progress_map_vbl` (the
  progress map), `EndPic` (the end-of-world picture), and their take-downs `MapEnd`, `IntroEnd`,
  `EndPicEnd`.
- **The RNG** (`engine/crates/sf-core/src/rng.rs`; [docs/08 "RNG"](../08-executable-map.md#rng-oracle)).
  `Rng` is `rng_state`'s two longs; `Rng::seeded` is `rng_seed`, `Rng::next` is `rng_next`,
  `Rng::range` is `rng_range`, including its `n = 0xFFFF` case that returns 0 without advancing.
- **Save states** (`engine/crates/sf-core/src/savestate.rs`; rust-engine DESIGN §2.8). A
  bincode `StateHeader` (magic `SFST`, engine build, asset cache hash, frame), a BLAKE3 of the
  body, then the whole `Game` encoded field by field. `Game::load` refuses another build
  (`StateError::OtherBuild`), other game data (`OtherCache`), a damaged file (`Corrupt`) or a
  foreign one (`NotAState`). `DisplayData` is not saved; it is rebuilt from the `GameData` and the
  state's own `.sflevel` maps are put back (`Game::restore_display`).
- **Bug recordings** (`engine/crates/sf-core/src/recording.rs`;
  [bug-recorder DESIGN §2.6](../../plans/bug-recorder/DESIGN.md)). `Recording` is a text format:
  a fixed header (`superfrog recording 1`, build, the `LevelSetup`, the seed, frames, `stopped-at`
  as a `Location`, the note) and the inputs in the conformance script's tokens. One parser
  (`Recording::parse`, `Recording::to_text`) serves the app (`engine/crates/sf-app/src/recorder.rs`)
  and `sf-conform` (`engine/crates/sf-conform/src/bug.rs`).
- `engine/crates/sf-core/src/lib.rs` declares the modules and re-exports the crate's public
  types; `engine/crates/sf-core/Cargo.toml` names its dependencies (`sf-assets`, `sf-disk` for the
  intro's `ATN!` unpacking, `sf-paula`, `serde` and `bincode` for save states).
- Tests owned here: `engine/crates/sf-core/tests/savestate.rs` and
  `engine/crates/sf-core/tests/start_level.rs`, plus the unit tests in each file.

**What belongs to [cpu-time-and-beam](cpu-time-and-beam.md).** `game.rs` also holds that
mechanism's interface: `Pacing`, `PassClock`, `LoadEnd`, `Job`, `HandlerEnd`, `Cpu`, `RACE_LINES`
and `beam_race`, and the conformance setters `Game::anchor_writes`, `Game::anchor_blits`,
`Game::set_costs`, `Game::keep_trace`, `Game::load_blanks`, `Game::set_load_line`,
`Game::set_screen_frames`, `Game::set_handler_entry`, `Game::set_handler_end`,
`Game::set_blank_merged`, `Game::set_load_state` and `Game::set_handover_state`. That chapter
describes what they do. `CODE_TIMES` is not part of it: it is how many frames apart
`Game::type_code` presses keys on the options screen.

## 2. Kind mix

`amiga` 30, `game` 20, `data` 5, `shell` 45.

- `amiga`: in `game.rs`, the conformance interface above, `Start::Transplant` and
  `Start::Resume`, `probe`, `peek`, `poke` and `chip_word` by original address, and the keyboard's
  serial byte (`key_byte`, `sdr`, `level_io`); in `scene.rs`, `SceneKind::classify` (screens told
  by copper list addresses), `Scene::of` (a transplant's scene from `logic_running`) and
  `Scene::step`'s beam log and display-window cut; in `chain.rs`, reading and executing 68000
  opcodes out of hunk 0, the loads with their disk requests, and the screens' copper lists,
  plane pointers and hardware sprites.
- `game`: the scene hand-overs in `step_body` and `level_frame`, `Progress`, the RNG, and in
  `chain.rs` what the screens between levels do (the frog's walk across the loading picture, the
  clouds, the map, the end picture, which file each world loads).
- `data`: the hunk 0 addresses `chain.rs` dispatches on (`WORLD_LOADS`, `LEVEL_INITS`,
  `END_PICS`, the `target` table) and the `top_loop` code it reads, which is the original's own.
- `shell`: `recording.rs`, `savestate.rs` and both test files whole; in `game.rs`, `start_at`,
  `type_code`, `start_level`, `location`, the override and view plumbing and the unit tests;
  `lib.rs`.

Judged by reading each file and splitting its lines by the parts above; `chain.rs` was split
about half `amiga`, 40 % `game`, 10 % `data`, `game.rs` about 45 % `amiga`. `game`'s share is low
because most of Superfrog's rules live in the chapters the flow calls into, not here.

## 3. Mechanisms it uses

- **Byte-exact state at original addresses** ([memory-model](memory-model.md)). `Game` holds a
  `State`; the flow reads its variables by address (`world` h0+$40da, `level` h0+$40de,
  `attract_mode` h0+$18, `final_level_flag` h0+$1e, `vbl_count` h0+$9d8, `level_state` h0+$a06),
  and `start_level` writes the dialog's values the same way. `Game::probe` and the
  `Transplant`/`Resume` starts are the transplant and probe mechanism.
- **The loaded hunk image** ([copied-in-data](copied-in-data.md)). `Chain` runs `top_loop` from
  the hunk 0 bytes (`FrontData::h0_w`, `FrontData::h0_l`), and `level_disk` and `init_call` search
  them. `game_start_at` reads `start_resume_table` (h0+$bde) out of memory.
- **Screens told by copper list.** `SceneKind::classify` maps `COP1LC` to a screen: the level's
  list (`render::LEVEL_COPPER`, split by `attract_mode`, `frog_lives` and `world`), `TITLE_COPPER`,
  `AWARD_COPPER`, `HALL_COPPER`, `TALLY_COPPER`, `SLOT_COPPER`, `END_COPPER`, `INTRO_COPPER`,
  `MAP_COPPER`, `WORLD_END_COPPER`; `BLANK_COPPER` (the black list screens fade through) and no
  list give `None`, so the screen before stays the scene. It is an Amiga-shaped classifier: it
  works the same on the engine (`Game::cop1lc`) and on the original's memory in the oracle
  (`FrameState::scene` in `engine/crates/sf-conform/src/milestones.rs`), which is why one function
  serves both. Its two users are `sf-conform`'s `Milestone::Scene` (check 4) and the bug recorder's
  `Game::location`. The engine's `Scene` does not tell screens apart (`Scene::Front` covers the
  title, the tally and the map alike), so without the copper list there is nothing to classify.
- **Display hardware** ([display-hardware](display-hardware.md)). The screens between levels set
  copper moves (`set_planes`, the split at `INTRO_SPLIT`), fade palettes and turn DMA on, and put
  the frog and clouds on hardware sprites (`front::sprite::attach`, `front::sprite::update`).
- **Interrupts** ([frame-interrupts-input](frame-interrupts-input.md)). `exit_walk_vbl` and
  `progress_map_vbl` run in the front end's vertical blank. `Game` turns key changes into the
  CIA-A serial byte (`key_byte`) the keyboard poll reads, and carries it into a level
  (`kbd_flag`, `kbd_acked`).
- **CPU time and the beam** ([cpu-time-and-beam](cpu-time-and-beam.md)). Each level's
  `level_init_wN` runs on the beam with `BLANK_COSTS`, or `RESTART_COSTS` after the `bmi` back
  (`Chain::back`). `WorldLoad::clear_map_area` is a `Job` timed at 30 cycles a byte. Every load is
  done at once and reported as a `LoadEnd`.
- **The disk loader's behaviour.** `Cx::read_entry` (`engine/crates/sf-core/src/front/mod.rs`) is
  `track_read`'s disk check: a file on a disk other than the last one asked for sets
  `wanted_disk` (h0+$af54) and raises `DiskPrompt::Insert` in the frame's `FrameOut`. The
  `$adbe`/`$adca`/`$add6` calls in `top_loop`, the cracked disk requests, are a bare `rts`
  (`Target::Nothing`). See below.

### Disk prompts and swaps

The original asks for a disk whenever a file it needs is on another one
([docs/08 "Disk swaps"](../08-executable-map.md#disk-swaps)): disk 2 for W3–W5, the bonus level and
W6's sound and picture, disk 4 for W6's graphics and maps and the final level, disk 3 for the end
sequence. The engine has every disk, so it raises the prompt and goes straight on: nothing
answers it, and no frame waits. Today nothing reads `FrameOut::disk_prompt` either: `sf-app` does
not use it, and its only reader is the test below. What conformance compares is `wanted_disk`,
which the original's memory and the engine's both hold, as `Milestone::DiskAsked`.

Classified as `amiga`: it is the floppy drive's behaviour, not a rule of Superfrog's. It is kept
as game flow because `wanted_disk` is game state the original compares and sets the same way (the
disk a level code's way to a level leaves in the drive, `level_disk`, decides which map set
W6-2..4 load with an empty df1). A native rewrite with no disks still has to produce the same
`wanted_disk` changes for check 4, or change the check.

### The RNG

The RNG is game logic, the original's own routine and not hardware. What must be bit-exact is the
routine (the mask of bits 0, 4 and 5, the multiply by `2^17 + 3`, `rng_range`'s top-word
remainder and its `0xFFFF` case), the state's layout at `rng_state` (h0+$160d4, `lo` then `hi`),
and the order of calls, since about 40 object routines and the title draw from it and one call
moved shifts everything after it. The seed is part of the start (owner, 2026-10-11): normal play
seeds from the host clock (`host_seed` in `engine/crates/sf-app/src/main.rs`), a power-on
conformance run takes the oracle's, and a transplant copies `rng_state`. The original seeds a
second time, from CIA-B's time-of-day clock, after the end sequence
(`engine/crates/sf-core/src/front/ending.rs`); the engine keeps no clock, so normal play goes on
with the RNG as it is and conformance's run alone is fed the original's seeds as inputs (the
feed's `rng:` token, `AloneRun::seeds` in `engine/crates/sf-conform/src/lib.rs`).

## 4. What depends on it

- Every caller of the engine: `sf-app` steps a `Game`, saves and loads it
  (`engine/crates/sf-app/src/states.rs`) and writes recordings; `sf-conform` builds one per case
  from its sync point, steps it paced and compares `probe`, the picture and the Paula writes;
  `sf-e2e` drives the app; the route planners in `engine/crates/sf-conform/examples/` call
  `step_unseen`.
- [front-end](front-end.md), [level](level.md), [slot-and-bonus](slot-and-bonus.md) and
  [story-and-ending](story-and-ending.md) run inside the scene `Game` hands the frame to; the
  chain decides which of them runs next.
- Check 4's milestones (`engine/crates/sf-conform/src/milestones.rs`) depend on
  `SceneKind::classify` and on `wanted_disk`; the bug recorder's `stopped-at` and
  `sf-conform bug replay` depend on `Game::location` and `recording::SCREENS`.
- [verification](verification.md): check 1 matches states by `Game::passes` (`PassClock`), and the
  `chain-*`, `code-*` and `game-whole` cases run the whole sequence against the original.
- A save state's layout is the `Game` struct's field order: any change to `Game` or what it holds
  invalidates every saved state, which is why another build is refused.

## 5. What a native rewrite must preserve

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| `Chain` running `top_loop` from hunk 0 | The original's order of loads, inits, tally, end pictures and music fades per world and level, its branch back after a death, and the resume point a level code jumps to | `start_game_reaches_w1_1_as_the_original_does`, `a_level_code_starts_the_game_at_w3_1_as_the_original_does`, `w1_1_goes_on_to_w1_2`, `w1_4_goes_on_to_w2_1`, `w5_4_goes_on_to_the_bonus_level`, `the_bonus_level_goes_on_to_w6_1`, `game_over_then_w2_1_s_code_continues_in_w2_1` in `engine/crates/sf-conform/tests/chain.rs`; the `code_cases!` tests there (ignored, ~90 s each); `the_whole_game_plays_as_the_original_does` (ignored, ~25 min) |
| The screens between levels (loading picture and walk, progress map, end-of-world picture) | Shown on the original's frames with its picture and sound, the bonus level's picture held `BONUS_PICTURE_FRAMES` (451) | the `chain-*` cases above (checks 1–3 over the screens); `after_a_worlds_fourth_level_come_its_end_picture_and_the_next_worlds_intro` in `engine/crates/sf-core/tests/start_level.rs` names the order only |
| `SceneKind` told by copper list | The same screen names on the engine as on the original, the blank list naming none | `each_list_names_its_screen_and_the_blank_names_none` in `engine/crates/sf-core/src/scene.rs`; `Milestone::Scene` compared in every case's check 4 (`sf-conform all`); `the_location_is_the_frogs_cell_then_each_screen_as_the_chain_goes` in `engine/crates/sf-core/tests/start_level.rs` |
| Disk requests (`Cx::read_entry`, `wanted_disk`, `DiskPrompt`) | A prompt once per disk the game asks for on the way into a world (none for W1, disk 2 for W4, disks 2 then 4 for W6), never waited on; `wanted_disk` changed as the original changes it | `disk_prompts_are_raised_as_the_game_asks_and_need_no_answer` in `engine/crates/sf-conform/tests/flow.rs`; `Milestone::DiskAsked` in check 4 of the `code-*` and `chain-*` cases |
| Instant loads (`LoadEnd`, `Game::load_blanks`, `WorldLoad::clear_map_area`'s `Job`) | Described in [cpu-time-and-beam](cpu-time-and-beam.md) §5; the flow decides where each load's `LoadEnd` points | check 1 in every case crossing a world's load (`sf-conform all`); no test isolates it |
| RNG routine and state layout | `rng_next`, `rng_range` and `rng_seed` bit for bit, the state's eight bytes at `rng_state` | `rng_rs_agrees_with_the_originals_routines_for_100_000_calls`, `rng_seed_agrees_with_the_original` in `engine/crates/sf-conform/tests/runner.rs` (on the Moira CPU, against the routines themselves); the unit tests in `engine/crates/sf-core/src/rng.rs` |
| RNG call order and seed | The same calls in the same order, the start's seed put in place where the original seeds | check 1 in every stepping case, `rng_state` among the compared variables; `the_title_case_is_green` (`title-rng`) in `engine/crates/sf-conform/tests/runner.rs`; `power_on_boots_the_front_end_with_its_seed` in `engine/crates/sf-core/src/game.rs`; `the_same_setup_and_seed_play_the_same_and_the_seed_reaches_the_rng` in `engine/crates/sf-core/tests/start_level.rs` |
| The seed after the end sequence | Conformance's run alone takes the original's seed; normal play keeps its RNG | check 4 on `game-whole` through the end sequence (`AloneRun::seeds`); no test names it |
| A level code typed as the player types it (`type_code_with`, `CODE_TIMES`) | `start_at` and `start_level` reach the same state as the original started the same way | `start_at_equals_the_code_typed_on_the_original` in `engine/crates/sf-conform/tests/flow.rs`; `w1_1_normal_starts_as_the_original`, `levels_2_to_7_normal_start_as_the_original` and the rest in `engine/crates/sf-conform/tests/recorder_setup_start.rs` |
| `PassClock` | Passes counted by the same rule on both sides | `the_pass_clock_counts_passes_ended` in `engine/crates/sf-core/src/game.rs` ([cpu-time-and-beam](cpu-time-and-beam.md)) |
| `Start::Transplant`, `Start::Resume`, `Game::probe` | Described in [memory-model](memory-model.md) | `transplant_then_probe_returns_the_originals_values` in `engine/crates/sf-conform/tests/runner.rs` |
| The story's hand-over to a disk-1 power-on | The game booted as from disk 1, what ran during the load put in place | the story cases; [story-and-ending](story-and-ending.md) |

Not Amiga-shaped, and kept by their own tests: save states (`a_loaded_level_runs_on_as_the_saved_one`
and the other `a_loaded_…_runs_on_as_the_saved_one` tests, `a_state_from_another_build_or_cache_is_refused`,
`the_encoding_is_deterministic` in `engine/crates/sf-core/tests/savestate.rs`) and the recording
format (`a_full_recording_round_trips` and the rest in `engine/crates/sf-core/src/recording.rs`).

## 6. Native sketch (non-binding)

Designing the rewrite is a later plan's job; this is a shape it could start from, not a decision.

- **The chain as a plain sequence.** `top_loop`'s game path is a fixed list per world, so it
  could be a table in Rust (world → its loads, levels, end picture) and a small state machine for
  the branch back after a death and the resume point, instead of an interpreter of 68000 opcodes.
  The `chain-*` and `code-*` cases would keep it honest. What is lost is that today the order is
  the original's by construction; a hand-written table is the original's only by test.
- **Screens named by the game.** The native game would say which screen it is on (a `Screen`
  enum the front end sets as it enters each one), and the engine side of `SceneKind` would come
  from that. The original's side has no such thing, so `SceneKind::classify` by `COP1LC` stays in
  `sf-conform` for the oracle, with a test that each native screen maps to the kind the
  classifier gives the original's list on the same frame. The bug recorder's `Location` needs only
  the native enum.
- **Disks as an event, not a drive.** A native load could report "the game asked for disk N" as
  an event without a drive model. `wanted_disk` stays for check 4 unless that check moves to the
  event.
- **`Game` without the conformance hooks.** The twelve setters and the `FrameOut` fields that
  describe work done at once could move behind one conformance adapter
  ([cpu-time-and-beam](cpu-time-and-beam.md) §6 sketches the same split). Normal play would then
  see `new`, `step`, `render`, `save` and `load` alone.
- **Unchanged:** the RNG (already plain code, and must stay bit-exact), the save-state framing
  (though a native state layout would change what is in the body) and the recording format.

## 7. Reuse

**`superfrog-only`**, 4845 lines (`python tools/architecture.py totals`). The flow is Superfrog's:
its worlds, levels and screens, `top_loop`'s addresses, the screens between levels, the level
code's options-screen timing, the RNG's routine. Another game keeps only the method: run the
original's top-level loop from its own code, or write it down and test it against the oracle.

The generic parts, which a second game would take and change:

- **The `Game` shape**: a decision function of data, a start and inputs, with `step`, a paced
  `step` for conformance, `render`, `probe`, `save` and `load`. The shape carries as a pattern;
  the `Start` variants and the `FrameOut` fields beyond picture and sound are Superfrog's needs.
- **The save-state framing** (`savestate.rs`, 244 lines, and `engine/crates/sf-core/tests/savestate.rs`):
  `small-changes`. The header (magic, build, cache hash, frame), the body hash, the refusals and
  the `static_str` and `big_array` helpers carry; the magic and the type serialized change.
- **The recording format** (`recording.rs`, 885 lines): `big-changes`. The header-then-inputs
  layout, the input tokens, `MAX_FRAMES`, the parser's refusals by line and `Location`'s text form
  carry; `LevelSetup`, `Lives`, `Difficulty`, `SCREENS` and the level ids are Superfrog's.
- **`SceneKind::classify`**: the idea (name a screen by its copper list, on both sides) carries
  to any Amiga game that gives each screen its own list; the addresses do not.
- **`PassClock`**: carries to a game whose main loop runs every second frame and can overrun.

Team 17 note (unchecked): the RNG is a small standalone routine and could be shared with other
Team 17 games of the period; nobody has looked.

## 8. Sources

- [docs/08 "Scene flow", "RNG", "Disk swaps" and "Lag"](../08-executable-map.md#scene-flow)
- [plans/rust-engine/DESIGN.md](../../plans/rust-engine/DESIGN.md) §3.3 (the decision function),
  §2.8 (save states), §2.9 (what persists), §2.1 (the four checks)
- [plans/bug-recorder/DESIGN.md](../../plans/bug-recorder/DESIGN.md) §2.1 (the Start Level
  dialog), §2.6 (the recording format), §2.9 (engine steps against the original's frames)
- `plans/rust-engine/FINDINGS.md` (the RNG decision, 2026-10-11; T35's reseed after the end
  sequence)
- Code: `engine/crates/sf-core/src/game.rs`, `engine/crates/sf-core/src/scene.rs`,
  `engine/crates/sf-core/src/front/chain.rs`, `engine/crates/sf-core/src/rng.rs`,
  `engine/crates/sf-core/src/savestate.rs`, `engine/crates/sf-core/src/recording.rs`,
  `engine/crates/sf-core/src/front/mod.rs` (`Cx::read_entry`),
  `engine/crates/sf-conform/src/milestones.rs`
