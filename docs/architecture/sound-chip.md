+++
id = "sound-chip"
title = "The sound chip"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 87, game = 0, data = 0, shell = 13 }
reuse = "as-is"
owns = [
  "engine/crates/sf-paula/",
]
+++

# The sound chip

## 1. What it is

`sf-paula` is a model of Paula's four audio channels and the part of Agnus that feeds them DMA,
run one colour clock at a time, plus a resampler from Paula's clock to the host's sample rate. It
takes the Paula register writes of one PAL frame and the chip memory the channels' DMA reads, and
returns host-rate stereo. Nothing in it is specific to Superfrog. Its pieces:

- **`PaulaWrite`** (`engine/crates/sf-paula/src/write.rs`): one write to a Paula-related custom
  register (`reg` as an offset from `$DFF000`, `value`) and the beam position it was made at
  (`vpos`, `hpos`). It is the interface between the game's sound driver, the mixer and the oracle
  (§4).
- **The channel model** (`engine/crates/sf-paula/src/channel.rs`, `Chip`): the audio state
  machine of the Hardware Reference Manual (states 000, 001, 101, 010, 011) as vAmiga implements
  it, quirks included. Per channel: the length, period and volume latches, the data word, the DMA
  request that Agnus takes at the end of the line and serves in the channel's slot (hpos `$0D`,
  `$0F`, `$11`, `$13`), the period counter (0 means 65 536), and the DAC value, sample byte times
  latched volume. DMACON lands 2 clocks after its write and INTREQ 1 clock after; the rest at once.
  The order of events within one clock is fixed (module header of `channel.rs`). ADKCON's attach
  modes (a channel's data setting the next one's volume or period) are modelled.
- **`Paula`** (`engine/crates/sf-paula/src/lib.rs`): the channels, the frame length in lines
  (`Paula::set_frame_lines`, 313 long or 312 short), and the mix: channels 0 and 3 left, 1 and 2
  right (`Paula::frame`). `Paula::channel_taps` gives each channel's DAC value at every clock of
  the last frame, for the per-channel check. `Paula::booted` starts with DMA's master switch on,
  as the game's boot leaves it.
- **`AudioDma`** (`lib.rs`): the same channel state machine on zeroed chip memory, run only for
  which DMA slots the channels take in each frame. The front end and the story intro use it to
  time the CPU's and the blitter's bus cycles around audio DMA (rust-engine T96).
- **The resampler** (`engine/crates/sf-paula/src/resample.rs`, `Resampler`): each change of a
  side's level becomes a band-limited step (a Kaiser-windowed sinc from a 128-phase table) on the
  host timeline. The output runs `RESAMPLER_DELAY` (24) host samples late.
- **`ChipMemory`**: the trait through which the DMA reads sample words. `sf-core` implements it on
  `Game` (`Game::chip_word`), so the mixer owns no game data.

Its unit tests are `engine/crates/sf-paula/tests/unit.rs`. The comparison with the reference
emulator lives in `engine/crates/sf-conform/tests/paula_vs_oracle.rs`, outside the crate, because
the purity test scans the pure crates' tests too. The RE note on how the game drives Paula is
[docs/11](../11-sound.md); the crate's place in the engine is
[rust-engine DESIGN §3.1, §3.2](../../plans/rust-engine/DESIGN.md).

## 2. Kind mix

`amiga` 87, `game` 0, `data` 0, `shell` 13.

- **`amiga`**: `channel.rs`, `write.rs`, `lib.rs` (`Paula`, `AudioDma`, `ChipMemory`, the PAL
  constants) and `tests/unit.rs`. All of it imitates the chip.
- **`shell`**: `resample.rs`, which turns Paula's output into host audio and imitates nothing on
  the Amiga.

Judged by attributing whole files by line count, tests with the code they test. The left/right
pairing in `Paula::frame` is a few lines and counts as `amiga`.

### What is exact and what is approximate

- **Exact**: each channel's DAC value at every Paula clock, given the original's writes at the
  original's beam positions and the same chip memory. `paula_vs_oracle.rs` feeds the oracle's
  writes and chip RAM through the title music and finds the taps equal to vAmiga's at every clock.
- **Approximate, by design**: the resampler. It uses floating point, the one exception to the pure
  crates' integer rule (rust-engine DESIGN §3.1, owner 2026-10-10), because its output feeds no
  game state. Its mix is held to vAmiga's own 48 kHz mix within a relative RMS tolerance, not
  sample for sample (`the_mix_is_within_tolerance_of_the_references_own`).
- **Approximate, from outside the crate**: in the engine the beam positions come from the
  engine's cost model, not the original (§4), so the engine's channel output is close to the
  original's, not equal. `sf-conform channels` judges how close.
- **Not modelled**: the A500's analog low-pass filter (about 4.4 kHz) and the LED filter
  (rust-engine FINDINGS 2026-10-11). The audio interrupts are raised inside `Chip` (into its
  INTREQ copy, which the no-DMA mode reads) but never reach a CPU: there is no output for them,
  and INTENA is ignored. Superfrog's driver runs from the vertical blank and does not need them.

## 3. Where else the mechanism is used

- **The app** (`engine/crates/sf-app/src/main.rs`): one `Paula::booted` per run, fed each frame's
  `FrameOut::paula` writes (`mixed()`) and the load's blanks (`Game::take_load_paula`), with the
  `Game` as `ChipMemory`. The samples go to the audio device (`engine/crates/sf-app/src/audio.rs`,
  chapter [app](app.md)).
- **The front end and the story intro** run `AudioDma` on their frames' writes to time bus
  cycles (`engine/crates/sf-core/src/front/mod.rs`, `engine/crates/sf-core/src/front/story/sim.rs`,
  chapters [front-end](front-end.md), [story-and-ending](story-and-ending.md)).
- **The game's sound driver** makes the writes (`engine/crates/sf-core/src/audio/`, chapter
  [sound-driver](sound-driver.md)); the level, front end and story place them on the beam (§4).
- **Verification**: `engine/crates/sf-conform/src/channels.rs` replays a pacing feed headless
  through the same mixer and compares `Paula::channel_taps` with vAmiga's taps
  (`sf_oracle::FrameCapture::taps`, from the core patch
  `engine/crates/sf-oracle/patches/0004-audio-sample-hook.patch`).
  `engine/crates/sf-conform/src/sound_events.rs` reduces `PaulaWrite`s to sound events for
  check 3. Chapter [verification](verification.md).

## 4. The interface: `PaulaWrite` and the beam position (T213)

The game's driver emits its writes in the original's order with `vpos` and `hpos` left 0
(`wr` in `engine/crates/sf-core/src/audio/mod.rs`). The caller then stamps each with where the
original's handler would have made it:

- **In a level**, the beam clock of [cpu-time-and-beam](cpu-time-and-beam.md):
  `LevelCtx::audio_spent` and `LevelCtx::stamp` in `engine/crates/sf-core/src/level/mod.rs`, each
  write costing `Cost::AudioWrite` or `Cost::AudioStop`.
- **Outside a level**, `front::vbl::place` (`engine/crates/sf-core/src/front/vbl.rs`): a cost model
  of the handler, `music_tick`'s writes spread over it, the DMA stage's eight lines later after
  the wait at `h0+$12a14`.
- **In the story intro**, at its wait's end (`engine/crates/sf-core/src/front/story/intro.rs`).
- **In conformance** the original's own positions can replace them (`Game::anchor_writes`).

Why it matters: Paula restarts a note only if the channel had time to go idle between a DMACON
off and the DMACON on that follows it. With every write at one instant, a channel retriggered
partway through a sample never noticed DMA went off and played its old sample on: the new note
was lost (rust-engine T213, found by T30 at frame 108762 of the whole game). T213 gave the
engine's writes positions and added the per-channel output check; it is ✅ in
`plans/rust-engine/PROGRESS.md`. Its follow-ups T215 (judging against neighbouring frames) and
T216 (the bonus level's channel 3) are ✅ too. What is left open is recorded in
`plans/rust-engine/FINDINGS.md` (2026-10-19 rows): the story's `sfx_play` makes all its writes at
one instant, copper writes to Paula in a front frame reach the mixer as the next frame's, and
writes past a frame's end land together at its last clock; none was measured audible.

What a native rewrite of either side must keep:

1. **The order of a frame's writes**, as the original makes them: check 3 compares the events
   they reduce to, and the mixer applies them in list order.
2. **A position for every write**, in beam order within the frame, with a DMACON off and its
   following on far enough apart for the channel to go idle (the period of the sample playing).
   A list whose positions go back is read as the rest belonging to the next frame
   (`Chip::run_frame`), so a driver must not emit positions out of order by accident.
3. **Writes the mixer can play unheard**: a load's blanks (`Game::take_load_paula`) go through the
   mixer so its channels leave the load as the original's did.
4. **The chip memory the DMA reads** (`ChipMemory`), with the bytes the driver changed there.

## 5. What depends on it

- The app's sound, every frame.
- `sf-conform channels` and its tests, `paula_vs_oracle.rs`, `sf-e2e`'s `whole_game.rs` (the
  app's loudness per frame against the original's `.audio`).
- The front end's and the story's bus timing through `AudioDma`, and so their state checks:
  a change to the slot timing moves where their blits end (`story_draws.rs`).
- Save states: `AudioDma` is serialized inside `Game` (`serde` derives on `Chip`). The app's
  `Paula` is not, and a loaded state starts a fresh mixer (rust-engine FINDINGS 2026-10-18).

## 6. What a native rewrite must preserve

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| Channel state machine and DAC | Each channel's DAC value equals vAmiga's at every Paula clock, given the same writes and chip memory | `paula_vs_oracle.rs`'s `channel_taps_are_the_references_at_every_clock_through_the_title_music`; `unit.rs`'s `a_channel_at_period_428_loops_its_waveform_at_the_right_frequency`, `the_pointer_and_length_reload_at_the_end_of_each_block`, `volume_64_is_full_and_0_is_silent` |
| Register delays | DMACON lands 2 clocks late, INTREQ 1 clock; without DMA the CPU plays a word per interrupt | `unit.rs`'s `dmacon_lands_two_clocks_after_its_write`, `with_dma_off_the_cpu_plays_a_word_per_interrupt_and_intreq_lands_a_clock_late` |
| Small periods | A period below the DMA minimum starves the channel, it does not play faster | `unit.rs`'s `a_period_below_the_dma_minimum_starves_rather_than_speeds_up` |
| Retrigger restarts only after idle | A channel turned off and on restarts its note only if it went idle between | `engine/crates/sf-conform/tests/channels.rs`: `the_titles_retriggered_notes_restart_as_on_the_original`, `the_slot_machines_effects_restart_as_on_the_original`; `the_whole_games_channels_sound_as_the_original_does` (ignored, over `engine/conform/cases/game-whole.toml`) |
| Frame boundary | Writes listed after the positions go back land at the next frame's start; a frame is 313 or 312 lines | `unit.rs`'s `writes_listed_after_the_frame_wrapped_land_at_the_next_frames_start`, `a_short_frame_is_one_line_shorter`, `a_frame_is_long_or_short_and_nothing_else` |
| Short frames in the app | The game keeps whole stretches of short frames (LACE cleared with LOF short) | No check. `paula_vs_oracle.rs` sets each frame's length from the oracle, but the app and `channels::engine_levels` never call `Paula::set_frame_lines`, so they mix every frame as long. Whether that is audible is unknown: run `sf-conform channels` with the frame lengths fed |
| Stereo pairing | Channels 0 and 3 left, 1 and 2 right | `unit.rs`'s `channels_0_and_3_are_left_1_and_2_right` |
| DMA slots for bus timing | `AudioDma` takes the slots Agnus would | `engine/crates/sf-conform/tests/story_draws.rs` (the intro's draws against the original's register writes, BLTSIZE among them, with `AudioDma` run on the original's Paula writes holding the blits up) |
| Attach modes | ADKCON attached volume and period | No check covers them. Superfrog's driver writes no ADKCON that this chapter found; a game that uses them needs a test against vAmiga |
| Purity | Same writes and memory give the same output | `unit.rs`'s `the_output_is_a_pure_function_of_the_writes_and_memory`; `engine/crates/sf-conform/tests/purity.rs` (only `resample.rs` exempt) |
| Resampler (shell) | Host-rate share of samples per frame with a fixed delay; mix within tolerance of vAmiga's | `unit.rs`'s `a_frame_makes_the_host_rate_share_of_samples_with_a_fixed_delay`; `paula_vs_oracle.rs`'s `the_mix_is_within_tolerance_of_the_references_own` |

## 7. Native sketch (non-binding)

Non-binding: designing the rewrite is a later plan's job. `sf-paula` is already the native shape
of what it imitates: a clock-stepped model behind a small interface (`PaulaWrite` in, PCM out).
A native rewrite of the game's sound driver could keep it unchanged and keep emitting
register writes. A rewrite that stops emitting writes (a native mixer playing notes directly)
would lose the per-channel equality with the original that this crate gives, and would need a
new way to prove the retrigger behaviour above. One open improvement either way: pass each
frame's length to the mixer (§6, short frames). Running per clock is the cost: 71 051 clocks a
frame; an event-driven version stepping from one state change to the next would give the same
taps and is the obvious optimisation if it is ever needed.

## 8. Reuse

**`as-is`**, 1142 lines (`python tools/architecture.py totals`, 2026-10-23).

Paula is the same chip in every OCS and ECS Amiga, and nothing in the crate names Superfrog: it
carries to any Amiga game whose port emits register writes with beam positions. The things a
second game might need, none of them a change to what is there:

- NTSC: the constants are PAL (`PAULA_CLOCK_PAL`, `LINES_PER_FRAME`); an NTSC game needs NTSC
  values beside them.
- A game whose replayer runs from the audio interrupts needs `Chip`'s INTREQ exposed to its CPU
  model; today nothing reads it from outside.
- Attach modes are modelled but unchecked (§6).
- `booted`'s DMACON `$8200` describes Superfrog's boot; another game may boot with other bits.

## 9. Sources

- `engine/crates/sf-paula/src/channel.rs`, `lib.rs`, `write.rs`, `resample.rs` and
  `engine/crates/sf-paula/tests/unit.rs`.
- `engine/crates/sf-conform/tests/paula_vs_oracle.rs`, `engine/crates/sf-conform/tests/channels.rs`,
  `engine/crates/sf-conform/src/channels.rs`, `engine/crates/sf-conform/src/sound_events.rs`,
  `engine/conform/README.md` ("Each channel's output (T213)").
- `engine/crates/sf-core/src/audio/mod.rs`, `engine/crates/sf-core/src/level/mod.rs`,
  `engine/crates/sf-core/src/front/vbl.rs`, `engine/crates/sf-core/src/game.rs`,
  `engine/crates/sf-app/src/main.rs`.
- [docs/11](../11-sound.md); [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.1, §3.1,
  §3.2; `plans/rust-engine/tasks/T213-paula-write-timing.md`; `plans/rust-engine/FINDINGS.md`.
