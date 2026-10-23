+++
id = "verification"
title = "Verification"
tier = "component"
depth = "light"
kind = "shell"
mix = { amiga = 4, game = 0, data = 0, shell = 96 }
reuse = "big-changes"
owns = [
  "engine/crates/sf-oracle/",
  "engine/crates/sf-conform/",
  "engine/crates/sf-e2e/",
  "engine/conform/",
]
+++

# Verification

## What it is

The machinery that judges the engine against the original, by machine
(rust-engine [DESIGN §2.1, §4](../../plans/rust-engine/DESIGN.md)). It is the largest chapter by
lines (47 779) and most of the code that carries to another Amiga game. It is also what a native
rewrite must keep passing, so this chapter is the rewrite's acceptance test, seen from the inside.
Three crates and one folder of inputs:

- **`sf-oracle`** (`engine/crates/sf-oracle/`): the original, run on vAmiga (MPL-2.0) at a pinned
  commit (`VAMIGA_COMMIT`) with five core patches (`patches/`), built outside the repo by
  `tools/setup/oracle.sh` and linked through a C shim (`shim/shim.cpp`, `shim/shim.h`, `src/ffi.rs`).
  `Amiga` steps one PAL frame per `Amiga::step` with the engine's own `FrameInput` and returns a
  `FrameCapture` (the display window's picture as 12-bit colour, the Paula writes with their beam
  positions, the DAC taps). It snapshots and restores the whole machine, reads and pokes RAM, and
  reports the CPU's registers, interrupts taken, blitter events and every CPU custom-register write.
  `cached_trace` memoises a scripted run in `~/.cache/superfrog-re/traces/`. `Cpu68k`
  (`src/routine.rs`, `shim/moira_shim.cpp`) is the routine runner: Moira, vAmiga's 68000 core,
  compiled alone over a flat 16 MiB memory with no chipset, which calls one routine of the original
  and returns its registers and every byte it changed ([docs/RE-WORKBENCH](../RE-WORKBENCH.md)
  "Differential-testing a routine").
- **`sf-conform`** (`engine/crates/sf-conform/`): the conformance runner and its CLI
  (`src/main.rs`: `run`, `all`, `scan`, `record`, `feed`, `channels`, `bug replay|compare|case`).
  `src/case.rs` reads cases, `src/script.rs` input scripts, `src/sync.rs` brings both sides to a
  sync point and reads the original's pacing, `src/lib.rs` (`run`) steps both and judges them,
  `src/compare.rs`, `src/sound_events.rs` and `src/milestones.rs` are checks 1–4, `src/channels.rs`
  is the per-channel output check, `src/report.rs` the divergence and its diff PNG, `src/feed.rs` the
  pacing feed, `src/bug.rs` the bug recorder's replay and comparison ([docs/BUG-FIX](../BUG-FIX.md)),
  `src/routine.rs` routine conformance (`Rig`), `src/freeze.rs` what a case leaves out. Its tests
  (50 files and a shared module) are the engine's conformance suite, and its examples (68 files)
  the planners that write input scripts and the probes past tasks used to find causes.
- **`sf-e2e`** (`engine/crates/sf-e2e/`): the real `superfrog` binary driven end to end, through its
  script layer (`App::run_script`: `--script`, `--capture`, `--pacing`, `--capture-hashes`) and an
  OS layer of real key events, menu clicks and window captures (`src/os.rs`, only under
  `SF_E2E_OS=1`); `engine/crates/sf-e2e/README.md` has the detail.
- **`engine/conform/`**: 212 case files and 178 input scripts, inputs only, never disk data
  (`engine/conform/README.md` documents both). None of it is counted as code (DESIGN §2.2).

## Kind mix

`mix = { amiga = 4, game = 0, data = 0, shell = 96 }`, judged by sorting files (and the parts of
`sf-oracle/src/routine.rs` and `sf-conform/src/sync.rs`) into one kind and weighing by lines. All of
it is test tooling, which DESIGN §2.1 calls `shell`; the Superfrog-specific parts (sync points,
milestones, route planners, case tests) stay `shell` here because none of it would exist in a
reimplementation of the game, and their specificity shows in the reuse rating instead. What counts
as `amiga` is the little that imitates the hardware rather than driving the emulator that does:

- `sf-conform/src/sound_events.rs` (430): Paula's latching, reproduced to reduce writes to events (a
  sample's address and length as the DMA latches them at the channel's start, period and volume as
  the frame leaves them).
- `Cpu68k`'s hardware mode (`set_hardware`, `set_hardware_read`): the routine runner records custom
  writes, runs area-mode blits, reproduces a CPU read of a write-only register as vAmiga does, and
  steps `VHPOSR` so a routine's raster waits end, standing in for the chipset.
- The bus and beam studies in `sf-conform/examples/`: `slot_lamps_sim.rs` and `fade_bus.rs` run
  routines on the story's bus model, `slot_audio.rs` runs `AudioDma`, and `fit.rs` fits the level's
  beam costs (`sf_core::level::beam::Cost::cycles`) to the original's positions.

Copied-in data is close to none: `script.rs`'s `C<entry>.<k>` token reads a level code's digits from
`password_table` on the player's disks when a script is parsed, so no code is committed. The
original's addresses in `sync.rs`, `freeze.rs` and `milestones.rs` are RE knowledge about the
original, not data read from it.

By part (lines are code lines, tests included):

| Part | Lines | What it is |
|---|---:|---|
| `sf-oracle/src` | 2 432 | `Amiga`, `Cpu68k`, traces, config, FFI |
| `sf-oracle/shim`, `build.rs` | 1 411 | the C++ shim over vAmiga and over Moira alone |
| `sf-oracle/tests` | 1 404 | determinism, the routine runner, drives, blits, `exe_map_claims.rs` |
| `sf-conform/src` | 9 693 | the runner, checks, sync points, feed, bug replay, routine rig |
| `sf-conform/tests` | 17 099 | the conformance suite |
| `sf-conform/examples` | 9 016 | route planners and probes |
| `sf-e2e/src` | 1 333 | launching the app, the script and OS layers |
| `sf-e2e/tests` | 5 391 | the app's features end to end |

## Where it touches the Amiga-shaped parts

The checks reach into the engine through a small number of doors, and most of the doors are
Amiga-shaped: they speak original addresses, Paula writes, copper list addresses and the original's
beam. That is the coupling a native rewrite meets first ([memory-model §8](memory-model.md)).

### What each check compares, and what it needs from the engine

| Check | Compares | Needs from the engine |
|---|---|---|
| 1. State, pass by pass (`compare::gameplay_vars`, `compare::PassMatch`) | every non-`presentation` `LAYOUT` variable at the end of each main-loop pass, the original's `n`th pass with the engine's `n`th | `Game::layout` and `Game::probe` (values by original address); `Start::Transplant` and `Start::Resume` (begin, and re-sync after lag or a load, from the original's memory and CPU); `Game::step_paced` with the `Pacing` read off the original; `Game::passes_ended`; `FrameOut::jobs` (memory the original's CPU may still be writing); `FrameOut::load` and `FrameOut::resync` (where a load ends, what it wrote that is not modelled) |
| 2. Picture within tolerance (`compare::PictureJudge`, `Tolerance::OWNER`) | the display window as 12-bit OCS colour per pixel, each frame | `FrameOut::picture` (`Picture`, indexed pixels plus a per-line palette, `to_rgb12`) in the original's display-window coordinates; `FrameOut::beam_races` to excuse rows a write raced the beam on (`compare::picture_except`) |
| 3. Sound as events (`sound_events`) | per channel: sample started (address, length, period, volume), period or volume change, stopped, within ±2 frames | `FrameOut::paula`, the frame's Paula register writes; `FrameOut::unheard` for writes inside a load |
| 4. Plays alone (`milestones`, `Milestones`) | scenes, coins, items, kills, lives, exits, tallies and the end, in order, within ±10 frames of play | a second `Game` started from the same `Start` and fed only the pacing feed (`sf_core::feed`: input, `Pacing`, `Game::load_blanks`, `Game::set_load_line`, `Game::set_screen_frames`, the RNG seeds poked into `rng_state`); `FrameState::read` over the engine's state by original address (`State::read`, 13 `LAYOUT` variables including `obj_slots` as 20 slots of `$7c` bytes and `level_map`); `Game::cop1lc`, from which `SceneKind::classify` (`engine/crates/sf-core/src/scene.rs`) names the screen by copper list address; `Game::mid_pass` |
| Each channel's output (`channels`) | each channel's level frame by frame of play over whole cases | the feed replayed (`FeedFrame::play`), `FrameOut::mixed`, `Game::take_load_paula`, and `Game::chip_word` so `sf_paula::Paula` can fetch the samples from chip RAM at their original addresses |
| `render` cases (`compare::picture`) | the renderer alone on the original's memory | `Start::Transplant`, then `Game::render` |
| Routine conformance (`routine::Rig::check`) | one ported routine against the original's on Moira, many generated states: bytes written, sound driver state, Paula writes | `State::transplant`, `AudioDriver::transplant`, and the port callable at the original routine's grain, as `FnOnce(&mut LevelCtx)`; every byte the original writes must be a `LAYOUT` or sound-driver variable |
| Bug replay and compare (`bug`) | a bug zip replayed, then the original fed the same inputs in legs of 8 passes | `Game::start_level`, `Game::step`, `Game::location`; put-back writes the engine's `LAYOUT` variables into the original's memory (the transplant's inverse) |
| `sf-e2e` | the app's frames, audio and state against the engine run headless | the app's script layer and `--pacing`; `feed::frame_hashes`, whose state hash is the engine's own save encoding (`Game::encoded`), so engine is compared with engine; `persist.rs` and `view.rs` also read `Game::probe` and `Game::state` |

Conformance also feeds the engine the original's timing, which no check compares but the cases need
to line up: the beam anchors (`Game::anchor_writes` with where each Paula write was made,
`Game::anchor_blits` with `Amiga::blit_starts`), the interrupt handler's entry and end
(`Game::set_handler_entry`, `Game::set_handler_end`, `Game::set_blank_merged`, from `Amiga::irqs`
and `sync::handler_end`), and the memory a load leaves or hands over (`Game::set_load_state`,
`Game::set_handover_state`, `Game::load_leaves`). `Pacing::Late` places the original's frame end by
its Paula write count, beam, pass phase, `sync::pass_step` and `sync::pass_point`, all read from the
original's CPU and memory at fixed addresses in `sync.rs` (the level loop's wait at h0+$aae, the
pass's calls, `bob_draw`'s seam). This is the [cpu-time-and-beam](cpu-time-and-beam.md) mechanism
seen from the checks; normal play gets none of it.

### What a native rewrite must still provide

The options for the address coupling are [memory-model §8](memory-model.md)'s: keep a mapping layer,
write a probe adapter (native state projected to original addresses, and the inverse for transplant),
or change the checks. From the checks' side, each door needs:

| Door | Native rewrite without an adapter | With an adapter |
|---|---|---|
| `Game::probe`, `Game::layout` (check 1) | breaks: the check compares by original address | the adapter's projection; the check is unchanged |
| `Start::Transplant`, `Start::Resume`, re-sync (checks 1–3, `render` cases, every mid-game sync point) | breaks: every case but the power-on ones starts from the original's memory | the adapter's inverse, original bytes to native state, pointers mapped back to native objects; `Resume` also needs a native place to resume the front end's routine from the original's PC |
| `FrameState::read` (check 4) | breaks: the milestones read 13 variables by address, and `obj_slots` by the original's slot layout | a small adapter, the 13 values plus each slot's object identity and hit points; or a native `FrameState` built from native fields, which keeps both sides on one derivation only if the original's side is still read by address |
| `Game::cop1lc` and `SceneKind::classify` (check 4) | breaks: the scene is named by copper list address | a native scene id mapped to `SceneKind` |
| `FrameOut::picture` (check 2, `render`, `sf-e2e`) | carries, if the picture keeps the display window's coordinates and the OCS colours | none |
| `FrameOut::beam_races` (check 2) | a native renderer that draws a whole frame at once reports no races, so frames the original raced may fail check 2's 2-frame and 1% budget | report the rows a draw would race, or match the original's raced frames |
| `FrameOut::paula`, `FrameOut::unheard` (check 3) | carries only if native sound still emits Paula register writes; `SoundEvent::Start` carries the sample's chip RAM address | an event adapter, native sound to `SoundEvent`, with sample addresses mapped to the original's |
| `Game::chip_word` (channels) | breaks if samples leave chip RAM at their original addresses | the adapter maps the original's sample addresses to native buffers |
| `Pacing`, anchors, handler and load timing (checks 1–3) | inputs a native engine may ignore; then checks 1–3 see more lag re-syncs and timing differences, which their tolerances may or may not absorb (not measured) | none; or keep a beam model only for conformance |
| `routine::Rig` (routine conformance) | lost for every routine restructured: the rig needs the port at the original routine's grain over byte state | per routine, an adapter at its inputs and outputs; or retire the routine test in favour of cases, losing its exhaustive inputs |
| The pacing feed (check 4, `sf-e2e` `whole_game.rs`) | carries: input, pacing, load timing and seeds are not addresses, but `feed::poke_seed` writes `rng_state` by address | a native seed setter |
| Bug compare's put-back | breaks: it writes the engine's `LAYOUT` into the original | the adapter's projection, written into the original |

Mixed routes are possible per subsystem, as memory-model says. The checks' unit of judgement fixes a
floor: check 2 (picture) and the `sf-e2e` hashes need nothing Amiga-shaped; check 3 needs Paula
writes or an adapter; check 4 needs a dozen values and a scene; check 1 and routine conformance need
the whole projection. Whether checks 2–4 alone would catch what check 1 catches is unmeasured; a
way to find out is to plant divergences (as `runner.rs`'s
`a_planted_one_variable_divergence_is_named_with_its_frame` does) and see which checks fail.

### The vAmiga patches

Five patches to the core at `VAMIGA_COMMIT`, applied by `tools/setup/oracle.sh`; their SHA-1 is part
of the build path and of `sf_oracle::BUILD`, so every trace and sync-point cache key changes with
them. Each hook is a null function pointer in the core that the shim installs.

| Patch | Hook or access | What it exposes | Why |
|---|---|---|---|
| `0001-custom-write-hook.patch` | `vamiga_custom_write_hook` in `Memory::pokeCustom16` | every custom-register write, CPU or copper, with the beam position, before it takes effect | check 3's Paula writes (`FrameCapture::paula`), the beam anchors (`Amiga::blit_starts`, each Paula write's line and slot), `Amiga::cpu_writes` (COP1LC for check 4's scene), `Amiga::watch_write` |
| `0002-shim-access.patch` | `SfShimAccess` a friend of `Emulator` | the emulator's private power-on and per-frame stepping | step one frame synchronously on the caller's thread, with no emulator thread or wall clock, so a run is deterministic and repeatable |
| `0003-keyboard-access.patch` | `SfShimAccess` a friend of `Keyboard` | the key-down matrix | vAmiga snapshots do not carry the ports' and keyboard's held state; `seatInput` re-seats them after `Amiga::restore`, so a cached sync point continues as the run that made it |
| `0004-audio-sample-hook.patch` | `vamiga_audio_sample_hook` in Paula's state machine (`penhi`, `penlo`) | each new DAC value per channel, with volume and beam position | `FrameCapture::taps`, the original's channel output for `channels` and `paula_vs_oracle.rs` |
| `0005-irq-entry-hook.patch` | `vamiga_irq_hook` in `Moira::willInterrupt`, `vamiga_blit_hook` in the blitter's begin, busy-clear and end | interrupts taken and blitter events with their beam position | `Amiga::irqs` for the handler's entry fed to the engine (owner, T46), `Amiga::blit_events` for the story's draw timing |

The shim adds what needs no patch: snapshots, RAM reads and pokes, the CPU's registers at the frame
boundary (`Amiga::cpu`), per-line CPU samples for profiling (`Amiga::line_samples`), and RAM copied
at the first CPU write of a value to a register (`Amiga::watch_write`). `Cpu68k` compiles a private
copy of Moira from the same source tree in its own namespace, with `shim/moira-stub/` standing in
for the two vAmiga headers Moira includes, because vAmiga's copy has its memory interface bound to
the emulated Amiga.

The oracle also patches the original game, in emulated RAM, not the core: `freeze` writes `rts` over
spawners the engine does not model (a case's `[[exclude]]`), and `sync::step_inserting` rewrites
h0+$c540 so the story intro reads the disk-change line again after a swap (vAmiga returns it stale;
rust-engine FINDINGS 2026-10-15, T47).

### What a native rewrite must preserve

The Amiga-shaped parts of this chapter, and the check that proves each.

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| The oracle's determinism (patch 0002) | The same inputs from the same start give the same frames | `power_on_to_the_title_twice_is_identical_on_every_frame`, `input_reaches_the_game_on_the_title` (`engine/crates/sf-oracle/tests/determinism.rs`) |
| Snapshot restore with held input (patch 0003) | A restored snapshot continues as the run that made it, keys and fire held across it | `a_restored_snapshot_continues_identically` (`determinism.rs`: fire, left and P held across frame 1 000) |
| Custom-write capture (patch 0001) | Every Paula write with its beam position, in order | every case's check 3; `a_paula_divergence_shows_the_first_differing_write_with_its_index` (`engine/crates/sf-conform/tests/runner.rs`) |
| DAC taps (patch 0004) | Each channel's DAC value at every change | `channel_taps_are_the_references_at_every_clock_through_the_title_music` (`engine/crates/sf-conform/tests/paula_vs_oracle.rs`) |
| Interrupt and blitter events (patch 0005) | Each blit's begin, free and end with its beam, flagged when past the frame's end; the interrupts taken | `a_frame_of_the_intro_has_its_bltsize_writes_and_each_blits_begin_free_end`, `a_blit_run_past_the_frames_end_is_flagged_next` (`engine/crates/sf-oracle/tests/blits.rs`); `draws_write_their_blits_on_the_originals_cycles` (`engine/crates/sf-conform/tests/story_draws.rs`, which reads `Amiga::irqs`) |
| Paula latching in `sound_events` | Writes reduce to the same events whatever their order in a frame; a sample's address and length latch at its start | `sound_events_from_writes`, `sound_events_reordered_in_a_frame_or_shifted_2_frames_match`, `sound_events_3_frames_apart_or_a_different_sample_fail`, `a_change_just_before_its_channel_stops_need_not_be_met` (`engine/crates/sf-conform/tests/faithful.rs`) |
| The original's pacing read (`sync::pacing`, `mid_pass`) | A late pass is not a divergence; frames ending mid-pass are left out, a lag re-synced or failed, and a case fails past its lag limit | `a_resync_after_lag_fails_a_case_that_forbids_it`, `a_case_fails_once_the_original_stays_mid_pass_past_its_limit` (`runner.rs`); `a_faithful_case_fails_once_the_original_stays_mid_pass_past_its_limit` (`faithful.rs`); `a_late_cut_the_engine_cannot_reach_ends_the_frame_as_normal_play_does` (`engine/crates/sf-conform/tests/alone.rs`) |
| Beam-race excusal (`RACE_LINES`, `picture_except`) | A frame differing only on rows a draw raced the beam on is excused and counted, never silently | `the_slot_machine_attract_loop_draws_as_the_original_does` (`engine/crates/sf-conform/tests/front.rs`: races seen, at most 45); no test plants a non-raced difference on a raced frame |
| Pass matching (`PassMatch`) | State compared by pass count, a pass a frame late on one side not a difference | `state_is_matched_by_pass_count` (`faithful.rs`) |
| Transplant and probe | The engine started from the original's memory probes back the original's values | `transplant_then_probe_returns_the_originals_values` (`runner.rs`) |
| `Cpu68k` hardware mode | A routine touching the chipset is refused outside hardware mode; in it, raster waits end, blits run and custom writes are recorded | `custom_chip_and_cia_accesses_are_errors_with_the_pc` (`engine/crates/sf-oracle/tests/routine.rs`); the routine suites that use `Rig` (`items_routines.rs`, `enemies_w1_routines.rs` and the rest) |
| Beam cost fitting and bus studies (`fit.rs`, `slot_lamps_sim.rs`, `fade_bus.rs`) | Tools, not behaviour; what they fitted is proven in the engine | no check runs them; `the_cost_model_alone_puts_paula_writes_on_the_originals_lines` (`engine/crates/sf-conform/tests/beam.rs`) holds the fitted costs |

`exe_map_claims.rs` (`engine/crates/sf-oracle/tests/exe_map_claims.rs`) is a different kind of
check: it holds RE claims about the original against the oracle (the game owns the vertical blank,
the title never lags, the RNG seed comes from the clock), so a native rewrite does not touch it.

## Reuse

Judged against another Amiga game by any developer (DESIGN §2.2). The machinery carries; the content
does not.

**Generic, `as-is` to `small-changes`: 9 521 lines.**

- `sf-oracle`'s harness, 3 843 lines: `amiga.rs`, `ffi.rs`, `routine.rs`, `trace.rs`, `lib.rs`,
  the shim, `build.rs`, the five core patches (any OCS game; nothing in them names Superfrog).
  `config.rs` needs the disk count and names and the `SUPERFROG_*` variables changed; the A500
  configuration (OCS, 512K chip plus 512K slow, Kickstart 1.3) suits most OCS games and is a change
  of a line for others.
- `sf-conform`'s checks and plumbing, 3 339 lines: `compare.rs` (bar `PANEL_ROWS` and the panel
  exclusion), `report.rs`, `sound_events.rs`, `channels.rs`, `script.rs` (bar the `C` token),
  `feed.rs`, `main.rs`, `routine.rs` (whose `Rig` takes a port as `LevelCtx`, the engine's type).
- `sf-e2e/src`, 1 333 lines: any macOS app with the same script layer; the binary's name and flags
  change.
- `faithful.rs` and `purity.rs`, 1 006 lines of tests: synthetic tests of the checks and the purity
  rule, which carry with what they test.

**Approach carries, `big-changes`: 4 556 lines.** `sf-conform`'s `lib.rs` (the run loop: its
shape carries, but loads, jobs, handler timing, the story's memory and the frog-only start are
Superfrog's), `case.rs` (the format carries, the sync point forms and exclusions do not), `bug.rs`
(level start by the recorder's dialog), `disks.rs`; and `sf-oracle`'s determinism, routine, drive
and blit tests, which use Superfrog's inputs and executables.

**`superfrog-only`: 33 702 lines.** `sync.rs` (about fifty original addresses: the handler, the level loop's
wait, pass steps, routes to every level by code), `milestones.rs` (what a coin, an item, a kill is,
read from Superfrog's variables), `freeze.rs`, `reference.rs`, `exe_map_claims.rs`; the 48 other
test files of `sf-conform`; all 68 examples, of which about 4 100 lines are route planners
(`plan_exit.rs`, `plan_exit_w6.rs`, `explore.rs`, `plan.rs`, `route.rs`, `route_oracle.rs`,
`complete.rs`, `plan_bonus.rs`, `slot_plan.rs`, `map_route.rs`) and the rest probes; and
`sf-e2e`'s tests, which follow the app (the [app](app.md) chapter). The method carries: the kinds of
sync point, the milestone list, the planners' search. `engine/conform/`'s cases and scripts are
per game too, and uncounted.

The 2026-10-15 estimate (DESIGN §1) counted about 5.5k of `sf-conform/src` as generic; this count
puts 3.3k there as `small-changes` and most of the rest of `lib.rs`, `case.rs` and `bug.rs` in
`big-changes`, because their Superfrog content is threaded through rather than set aside. For another
Team 17 game nothing here is likely to carry beyond what carries to any game, since the coupling is
to Superfrog's addresses, not its formats (unchecked).

The front matter carries the combined rating, `big-changes`, over 47 779 lines: the approach and a
fifth of the code carry, and 70% of the lines are per game. The totals in the overview count all of
it as `big-changes`; the split above is the better figure for a reuse estimate.

## Sources

- Design: rust-engine [DESIGN](../../plans/rust-engine/DESIGN.md) §2.1 (what faithful means), §3.1
  (the boundary and the purity test), §3.3 (the decision function), §4 (testing); this plan's
  [DESIGN](../../plans/engine-architecture/DESIGN.md) §2.5.
- Docs: `engine/conform/README.md` (checks, sync points, cases, scripts, pacing, beam races, bug
  zips), `engine/crates/sf-e2e/README.md`, [docs/RE-WORKBENCH](../RE-WORKBENCH.md),
  [docs/BUG-FIX](../BUG-FIX.md), rust-engine [FINDINGS](../../plans/rust-engine/FINDINGS.md)
  2026-10-11 (stepping vAmiga without its thread, `Amiga::cpu`), 2026-10-13 (patch 0005).
- Code: `engine/crates/sf-oracle/` (`src/amiga.rs`, `src/routine.rs`, `src/trace.rs`, `shim/shim.h`,
  `shim/shim.cpp`, `shim/moira_shim.h`, `patches/`), `engine/crates/sf-conform/src/`,
  `engine/crates/sf-e2e/src/`, and the engine's side of the doors: `engine/crates/sf-core/src/game.rs`
  (`Start`, `FrameOut`, `Game::probe`, `Game::step_paced`), `engine/crates/sf-core/src/scene.rs`
  (`SceneKind`), `engine/crates/sf-core/src/feed.rs`.
