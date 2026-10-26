+++
id = "sound-driver"
title = "The sound driver"
tier = "subsystem"
depth = "deep"
kind = "game"
mix = { amiga = 30, game = 65, data = 5, shell = 0 }
reuse = "big-changes"
owns = [
  "engine/crates/sf-core/src/audio/",
  "engine/crates/sf-core/tests/audio_driver.rs",
]
+++

# The sound driver

## 1. What it is

The port of the original's sound driver from `d1_c03`, routine for routine: three players sharing
Paula's four channels ([docs/11](../11-sound.md) describes the original).

- **Music** (`engine/crates/sf-core/src/audio/music.rs`): The Player 4.1A replayer (Jarno
  Paananen) as Superfrog modified it. `music_start` (`h0+$13126`), `music_stop` (`$132a0`),
  `music_tick` (`$13424`: `p41_play`, then the fade and the shadow copy `$145b6`) and the DMA
  stage `music_dma_tick` (`$13cc4`). Superfrog's change: the replayer writes each channel's
  pointer, length, period and volume to a shadow block, and the shadow copy sends them to Paula
  only for the channels no effect holds, after the fade caps the volume.
- **Effects** (`engine/crates/sf-core/src/audio/sfx.rs`): Team 17's envelope player over an
  `LnFX` bank, `sfx_bank_init` (`$12a26`), `sfx_play` (`$12a72`), `sfx_tick` (`$12c60`). An
  effect is a priority word and 18-byte steps of sample, duration, and period and volume ramps.
  An effect takes its channel from the music through the replayer's four "free" bytes
  (`$145b2`).
- **The streamer** (`sfx.rs`): `sample_play` (`$148b8`) and `sample_stream` (`$147e4`), eight
  long samples in hunk 0 copied 160 bytes a frame into a chip buffer and played once on channel 3.
- **`AudioDriver`** (`engine/crates/sf-core/src/audio/mod.rs`): the driver's state and its entry
  points (`play_music`, `stop_music`, `play_sfx` and its split form, `play_sample`, `tick`, and the
  four per-frame stages one by one). `Mem` is chip RAM as the driver sees it: the files the game
  loaded there (`File::Tune`, `File::Bank`, `files_for`), the bytes the driver wrote (the module
  header it relocates), and the first 12 bytes of chip RAM a zero instrument pointer reads
  (`LOW_CHIP`).
- **The story intro's copy** (`AudioDriver::intro_from`, `intro_vars`, `music_start_intro`,
  `IntroVars`): the intro (`d3_c04`) carries the same driver at other addresses; the engine runs
  this port on the intro's variables, taken from its memory and written back
  (`engine/crates/sf-core/src/front/story/sound.rs`, chapter [story-and-ending](story-and-ending.md)).
- **Tests**: `engine/crates/sf-core/tests/audio_driver.rs`, the driver on hand-built banks and
  modules for paths the disks' own data may never reach. The comparison with the original is
  `engine/crates/sf-conform/tests/audio_driver.rs` (owned by [verification](verification.md)).

The tunes, banks and streamed samples come from the disks through `sf_assets::sound::SoundData`
(`engine/crates/sf-assets/src/sound.rs`, chapter [disks-and-assets](disks-and-assets.md)).

## 2. Kind mix

`amiga` 30, `game` 65, `data` 5, `shell` 0.

- **`game`**: the replayer's logic (rows, effects, positions, the period and vibrato tables'
  use), the effect envelope player (priority, steps, ramps), the streamer, and the entry points.
  This is what the game sounds like, and any reimplementation needs it. The hand-built tests count
  here.
- **`amiga`**: the byte-exact state and what serves it in `mod.rs` (`Block`, `Mem`, `vars`,
  `transplant`, `vars()`, `var_block`/`set_var_block`, the intro's variable plumbing); the 68000
  bus quirks (`clr`, a `clr.w` to a custom register writing the prefetched word first, then 0;
  `ramp_value` reproducing `mulu`/`divu` word results and the overflow case); the routines split
  at their raster waits so the frame can take a vertical blank between their halves
  (`sfx_play_begin`, `sfx_play_wait_over`, `sfx_play_record`, `sfx_play_start`;
  `music_start_body`, `p41_init_head`, `p41_init_tail`); and `TickWork`, `fetches_rows`,
  `quiet_tick`, which say how long a tick's handler runs so the caller can place its writes.
- **`data`**: the tables read out of hunk 0 by address (`PERIODS` `$135a2`, `ARP_STEPS`,
  `FINETUNES`, `VIBRATO_TABLE`, the stream table) and the power-on values `AudioDriver::new`
  copies from the executable.

Judged by reading each file and attributing its lines: about 370 of `mod.rs`'s 523 lines are
state plumbing, about 15% of `music.rs` and 20% of `sfx.rs` are the split entry points, timing
queries and 68000 arithmetic. The replayer's logic is counted `game` although it reads and writes
its state at the original's offsets (`self.p41.w(ch(n) + PERIOD)`): the offsets are the
[memory-model](memory-model.md) mechanism, the rules are the game's.

## 3. Mechanisms it uses

- **Byte-exact state at original addresses**, its own instance. The driver's variables are eight
  runs of hunk 0 (`audio::vars::ALL`: `SFX` `$12f62`, `MUSIC_ON` `$13420`, `P41` `$144dc`,
  `SHADOW` `$146e0`, `STREAM` `$148b0`, `SFX_TABLES` `$1499e`, `MODULE_ADDR` `$9ec`,
  `HEADER_BACKUP` `$4814c`), each kept as a `Block` of the original's bytes inside `State::audio`,
  not in `State`'s own buffers. They are still `LAYOUT` entries (`AudioSfx` … `AudioHeaderBackup`
  in `engine/crates/sf-core/src/layout.rs`), and `State::byte`/`State::put` route an address in
  them to the driver. Why byte-exact: the 68000 code overlays words and bytes (a row's last byte is
  also its empty-row counter; a volume slide works on a word's low byte), and its pointers are
  absolute original addresses into files at their original places in chip RAM (the
  `audio/mod.rs` header). See [memory-model](memory-model.md) §5.
- **The loaded hunk image**: the period, arpeggio, finetune and vibrato tables and the streamed
  samples are read from hunk 0 through `SoundData::h0_word` and `h0_bytes`
  ([copied-in-data](copied-in-data.md)).
- **Paula writes**: every write is appended to the caller's list in the original's order with no
  position (`wr`); the caller places it on the beam ([sound-chip](sound-chip.md) §4).
- **Cycle charging and beam position**: in a level the frame calls the four per-frame stages one
  by one around the game's own code (`engine/crates/sf-core/src/level/frame.rs`) and charges each
  write (`LevelCtx::audio_spent`, [cpu-time-and-beam](cpu-time-and-beam.md)); outside a level
  `front::vbl::place` turns `TickWork` into positions (`engine/crates/sf-core/src/front/vbl.rs`).
- **The vertical-blank interrupt**: the driver ticks once per frame in the handler, in the order
  `stream_tick`, `music_tick`, `music_dma_tick`, `sfx_tick` (`AudioDriver::tick`;
  [frame-interrupts-input](frame-interrupts-input.md)). The split `sfx_play` exists because its
  raster wait holds a vertical blank off and the blank's `sfx_tick` runs in the middle of it
  (rust-engine T194).

## 4. What depends on it

- Every caller of a sound: the frog, items and enemies through the level's sound calls in
  `engine/crates/sf-core/src/level/mod.rs`, the slot machine (`engine/crates/sf-core/src/slot/rounds.rs`)
  and the story (`engine/crates/sf-core/src/front/story/ops.rs`) through `play_sfx` and
  `play_sample`; the level's load and exit, the front end and the game's flow through
  `sfx_bank_init`, `play_music`, `stop_music`, `fade_out` (`engine/crates/sf-core/src/level/load.rs`,
  `engine/crates/sf-core/src/front/`, `engine/crates/sf-core/src/slot/`, `engine/crates/sf-core/src/game.rs`).
- The state check of every `sf-conform` case: the driver's blocks are `LAYOUT` entries, compared
  pass by pass, except `music_fade` (the fade and the shadow registers), which is tagged
  presentation and left to the sound checks (`engine/conform/README.md`).
- Check 3, sound events, on every case; `sf-conform channels`; the app's sound; `whole_game.rs`.
- The mixer's view of chip RAM: `Game::chip_word` reads `AudioDriver`'s `Mem` outside the story,
  and `stream_source` names the 160 bytes the frame copies to `STREAM_BUFFER`.
- The vertical blank's timing outside a level: `TickWork` decides how long the handler runs, and
  so where later work in the frame lands.
- Save states: `AudioDriver` is serialized inside `State`.

## 5. What a native rewrite must preserve

The interface with the chip is [sound-chip](sound-chip.md) §4: the order of writes, a position
for each, a DMACON off and on far enough apart, and the chip memory the DMA reads. Beyond that:

| Amiga-shaped part | Behaviour to keep | Check that proves it |
|---|---|---|
| Driver variables at original addresses | After every command and every tick, every driver variable equals the original's, the relocated module header included (`AudioDriver::vars`) | `engine/crates/sf-conform/tests/audio_driver.rs`: `every_tune_matches_the_original_replayer`, `every_effect_alone_matches_the_original` (routine conformance on Moira); every case's state check on the `Audio*` `LAYOUT` entries |
| Write order and values | The same Paula writes, in order, after every command and tick | The same tests, and `title_music_matches_the_full_system_oracle` (2 000 frames on vAmiga); check 3 on every case |
| `clr.w` double write | A `clr.w` to a custom register writes the prefetched word, then 0 | Routine conformance compares the writes one by one (`audio_driver.rs`'s `rv` note); no test names it alone |
| Channel sharing | An effect or stream takes a channel through the free bytes; the priority word decides a busy channel; music drops its notes there | `overlapping_effects_and_streams_match_the_original`, `effects_during_music_match_the_original` |
| Fade and stray shadow copy | The fade caps volumes; at its end the shadow copy lands in RAM from `sfx_steal`, not on Paula | `fades_match_the_original` |
| Streamer | 160 bytes a frame from the original's sample, once on channel 3, then the channel back to music | `streamed_samples_play_the_originals_bytes`; `engine/crates/sf-core/tests/audio_driver.rs`'s `a_streamed_sample_plays_160_bytes_a_frame_then_stops` |
| Ramp arithmetic | `mulu`/`divu` word results, the dividend left on overflow | `every_effect_alone_matches_the_original` over every effect in every bank |
| `sfx_play`'s raster wait | A vertical blank held off by the wait runs one instruction after it, stepping the old effect from index 0 | `a_blank_held_by_sfx_plays_wait_steps_the_new_effects_period_on` (sf-core tests); `engine/conform/cases/enemies-w6-2-complete.toml` (frames 6658–6661) |
| Handler length per tick | `TickWork` gives the handler's length, so writes and later work land where the original's do | `engine/crates/sf-conform/tests/channels.rs` (`the_titles_retriggered_notes_restart_as_on_the_original`, `the_slot_machines_effects_restart_as_on_the_original`); the front cases' state checks (`engine/conform/cases/front-attract.toml`) |
| Files at original places | Modules and banks sit where the game loaded them; the attract demo keeps the title's module | `files_for_keeps_the_titles_module_in_the_attract_demo`; `bank_init_points_at_samples_and_effects_and_puts_the_module_after_them` |
| Zero instrument pointer | Reads its loop from chip RAM `$6..$b`, as transplanted | No check names it; routine conformance runs from the title's memory, where it may or may not arise. Find out by asserting on `LOW_CHIP` reads in `audio_driver.rs` |
| The intro's copy | Its `music_stop` turns music and audio DMA off and nothing more; its init keeps no header backup | `engine/crates/sf-conform/tests/story.rs` (the story cases' Paula writes); `channels.rs`'s `the_storys_music_plays_on_through_its_loads_as_the_original_does` (ignored, slow) |
| Not a module | A file that is not a module does not start | `a_file_that_is_not_a_module_does_not_start` |

## 6. Native sketch (non-binding)

Non-binding: designing the rewrite is a later plan's job. The driver is already behind a narrow
interface (commands in, `PaulaWrite`s out, ticked once per frame), so it can be rewritten on its
own. One shape: native structs per player (`Replayer` with four `Channel`s, `EffectPlayer`,
`Streamer`) and `Mem` replaced by the loaded files and a small relocated-header overlay, with a
projection to the eight hunk-0 blocks for probe and transplant (the adapter route in
[memory-model](memory-model.md) §8). The overlaid fields (the row word whose last byte is the
empty-row counter, the volume slide on a low byte) would become explicit fields whose projection
reproduces the overlay. The split entry points and `TickWork` stay as long as the caller places
writes by a cost model: they are what puts a note's DMACON off and on far enough apart.

## 7. Reuse

**`big-changes`**, 2174 lines (`python tools/architecture.py totals`, 2026-10-23).

For another Amiga game by any developer, the approach carries: a ported driver with its state at
the original's addresses, writes emitted in order and placed by the caller, compared routine by
routine on Moira. The code mostly does not. The replayer core (about 700 of `music.rs`'s lines)
would carry to another game that uses The Player 4.1A, after removing Superfrog's shadow block,
fade and free-byte sharing and changing every address. The effect player and the streamer are
Team 17's and Superfrog's. The `vars` offsets, `files_for` and `IntroVars` are Superfrog's alone.

**Other Team 17 games (unchecked).** The Player 4.1A was a common replayer, and Team 17's other
titles of the period may share both it and the `LnFX` effect format and envelope player, perhaps
with the same shadow-register change. If so, `music.rs` and `sfx.rs` would carry with new
addresses: `small-changes` for those games. Not checked: it needs their disks and its own RE work
(this plan's DESIGN §8).

## 8. Sources

- `engine/crates/sf-core/src/audio/mod.rs`, `music.rs`, `sfx.rs`;
  `engine/crates/sf-core/tests/audio_driver.rs`.
- `engine/crates/sf-conform/tests/audio_driver.rs`, `engine/crates/sf-conform/tests/channels.rs`,
  `engine/conform/README.md`.
- `engine/crates/sf-core/src/level/frame.rs`, `engine/crates/sf-core/src/front/vbl.rs`,
  `engine/crates/sf-core/src/front/story/sound.rs`, `engine/crates/sf-core/src/layout.rs`.
- [docs/11](../11-sound.md); [rust-engine DESIGN](../../plans/rust-engine/DESIGN.md) §2.1;
  `plans/rust-engine/tasks/T194-w6-2-sfx-double-blank.md`,
  `plans/rust-engine/tasks/T213-paula-write-timing.md`.
