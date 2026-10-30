+++
id = "sound-chip"
title = "The sound chip"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 87, game = 0, data = 0, shell = 13 }
reuse = "as-is"
owns = [
  "toolkit/crates/amiga-paula/",
]
+++

# The sound chip

## 1. What it is

`amiga-paula` is a model of Paula's four audio channels and the part of Agnus that feeds them DMA, run
one colour clock at a time, plus a resampler from Paula's clock to the host's sample rate. It takes the
Paula register writes of one PAL frame and the chip memory the channels' DMA reads, and returns
host-rate stereo.

- **`PaulaWrite`** (`toolkit/crates/amiga-paula/src/write.rs`): one write to a Paula-related custom
  register (`reg` as an offset from `$DFF000`, `value`) and the beam position it was made at (`vpos`,
  `hpos`). It is the interface between a game's sound driver, the mixer and the oracle.
- **The channel model** (`toolkit/crates/amiga-paula/src/channel.rs`, `Chip`): the audio state machine
  of the Hardware Reference Manual (states 000, 001, 101, 010, 011) as vAmiga implements it, quirks
  included. Per channel: the length, period and volume latches, the data word, the DMA request that
  Agnus takes at the end of the line and serves in the channel's slot (hpos `$0D`, `$0F`, `$11`, `$13`),
  the period counter (0 means 65 536), and the DAC value, sample byte times latched volume. DMACON
  lands 2 clocks after its write and INTREQ 1 clock after; the rest at once. The order of events within
  one clock is fixed (module header of `channel.rs`). ADKCON's attach modes (a channel's data setting
  the next one's volume or period) are modelled.
- **`Paula`** (`toolkit/crates/amiga-paula/src/lib.rs`): the channels, the frame length in lines
  (`Paula::set_frame_lines`, 313 long or 312 short), and the mix: channels 0 and 3 left, 1 and 2 right
  (`Paula::frame`). `Paula::channel_taps` gives each channel's DAC value at every clock of the last
  frame, for a per-channel check. `Paula::booted` starts with DMA's master switch on.
- **`AudioDma`** (`lib.rs`): the same state machine on zeroed chip memory, run only for which DMA slots
  the channels take in each frame, for timing the CPU's and the blitter's bus cycles around audio DMA.
- **The resampler** (`toolkit/crates/amiga-paula/src/resample.rs`, `Resampler`): each change of a side's
  level becomes a band-limited step (a Kaiser-windowed sinc from a 128-phase table) on the host
  timeline. The output runs `RESAMPLER_DELAY` (24) host samples late.
- **`ChipMemory`**: the trait through which the DMA reads sample words, so the mixer owns no game data.

Its unit tests are `toolkit/crates/amiga-paula/tests/unit.rs`. The comparison with the reference
emulator needs a game's music, so it lives with the game, and the purity test scans the pure crates'
tests too.

## 2. Kind mix

`amiga` 87, `shell` 13, by whole files and line count, tests with the code they test.

- **`amiga`**: `channel.rs`, `write.rs`, `lib.rs` and `tests/unit.rs`. The left/right pairing is a few
  lines and counts as `amiga`.
- **`shell`**: `resample.rs`, which turns Paula's output into host audio and imitates nothing.

### What is exact and what is approximate

- **Exact**: each channel's DAC value at every Paula clock, given the original's writes at the
  original's beam positions and the same chip memory.
- **Approximate, by design**: the resampler. It uses floating point, the one exception to the pure
  crates' integer rule, because its output feeds no game state. Its mix is held to vAmiga's own 48 kHz
  mix within a relative RMS tolerance, not sample for sample.
- **Approximate, from outside the crate**: an engine's beam positions come from its cost model, so its
  channel output is close to the original's, not equal.
- **Not modelled**: the A500's analog low-pass filter (about 4.4 kHz) and the LED filter. The audio
  interrupts are raised inside `Chip` (into its INTREQ copy, which the no-DMA mode reads) but never
  reach a CPU, and INTENA is ignored.

## 3. Why a write needs a position

Paula restarts a note only if the channel had time to go idle between a DMACON off and the DMACON on
that follows it. With every write at one instant, a channel retriggered partway through a sample never
notices DMA went off and plays its old sample on: the new note is lost. A driver feeding this mixer must
provide:

1. **The order of a frame's writes**: the mixer applies them in list order.
2. **A position for every write**, in beam order within the frame, with a DMACON off and its following
   on far enough apart for the channel to go idle (the period of the sample playing). A list whose
   positions go back is read as the rest belonging to the next frame (`Chip::run_frame`), so a driver
   must not emit positions out of order by accident.
3. **Writes the mixer can play unheard**, such as those made during a load, so its channels leave the
   load as the original's did.
4. **The chip memory the DMA reads** (`ChipMemory`), with the bytes the driver changed there.

## 4. What depends on it

A desktop app's sound through `amiga-app`'s `audio.rs`; `amiga-conform`'s `channels.rs` and
`sound_events.rs` ([verification](verification.md)); bus timing through `AudioDma`, so a change to the
slot timing moves where a game's blits end. `AudioDma` is serializable (`serde` derives on `Chip`), so
it can sit in a save state; `Paula` is not.

## 5. What must be preserved

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| Channel state machine and DAC | Right frequency, reloads, volume | `a_channel_at_period_428_loops_its_waveform_at_the_right_frequency`, `the_pointer_and_length_reload_at_the_end_of_each_block`, `volume_64_is_full_and_0_is_silent` (`tests/unit.rs`) |
| Register delays | DMACON lands 2 clocks late, INTREQ 1 clock; without DMA the CPU plays a word per interrupt | `dmacon_lands_two_clocks_after_its_write`, `with_dma_off_the_cpu_plays_a_word_per_interrupt_and_intreq_lands_a_clock_late` |
| Small periods | A period below the DMA minimum starves the channel, it does not play faster | `a_period_below_the_dma_minimum_starves_rather_than_speeds_up` |
| Frame boundary | Writes listed after the positions go back land at the next frame's start; a frame is 313 or 312 lines | `writes_listed_after_the_frame_wrapped_land_at_the_next_frames_start`, `a_short_frame_is_one_line_shorter`, `a_frame_is_long_or_short_and_nothing_else` |
| Stereo pairing | Channels 0 and 3 left, 1 and 2 right | `channels_0_and_3_are_left_1_and_2_right` |
| Attach modes | ADKCON attached volume and period | No check covers them. A game that uses them needs a test against vAmiga |
| Purity | Same writes and memory give the same output | `the_output_is_a_pure_function_of_the_writes_and_memory`; `toolkit/crates/toolkit-checks/tests/purity.rs` (only `resample.rs` exempt) |
| Resampler (shell) | Host-rate share of samples per frame with a fixed delay | `a_frame_makes_the_host_rate_share_of_samples_with_a_fixed_delay` |

## 6. Native sketch (non-binding)

This is already the native shape of what it imitates: a clock-stepped model behind a small interface
(`PaulaWrite` in, PCM out). A driver rewrite can keep emitting register writes. One that stops (a native
mixer playing notes directly) loses per-channel equality with the original and needs a new proof of the
retrigger behaviour. Running per clock is the cost: 71 051 clocks a frame; an event-driven version
stepping between state changes would give the same taps.

## 7. Reuse

**`as-is`**. Paula is the same chip in every OCS and ECS Amiga. It carries to any Amiga game whose port
emits register writes with beam positions. A second game might need, none of it a change to what is
there:

- NTSC: the constants are PAL (`PAULA_CLOCK_PAL`, `LINES_PER_FRAME`); an NTSC game needs NTSC values
  beside them.
- A replayer run from the audio interrupts needs `Chip`'s INTREQ exposed to its CPU model.
- Attach modes are modelled but unchecked.
- `booted`'s DMACON `$8200` is one boot's state; another game may boot with other bits.

## 8. Sources

`toolkit/crates/amiga-paula/`, the Hardware Reference Manual.
