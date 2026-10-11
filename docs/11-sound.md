# 11 — Sound

How `d1_c03` makes its music and sound effects, for the audio driver (`engine/crates/sf-core/src/audio/`,
T11) and the tasks that call it. Read [`08-executable-map.md`](08-executable-map.md) first for the
memory map and the interrupt. Addresses are `h0+$off` in `d1_c03`; names are in
[`re/symbols.toml`](../re/symbols.toml). Everything here is checked against the original by
`engine/crates/sf-conform/tests/audio_driver.rs` unless marked **(inferred)**.

There are three players, all on Paula's four channels:

- **Music**: a "P41A" module (The Player 4.1A by Jarno Paananen), replayed by a copy of its
  replayer that Superfrog modified to write a shadow register block instead of Paula.
- **Sound effects**: Team 17's own envelope player over a bank of samples and step lists (`LnFX`).
- **Streamed samples**: eight longer samples kept in hunk 0 (slow RAM, out of Paula's reach),
  copied 160 bytes a frame into a chip buffer and played on channel 3.

An effect or a stream takes its channel from the music: four "free" bytes (`$145b2`, channel 3's
first) hold `$ff` while music may use a channel, else the effect's number (or 1 for a stream).
Music keeps running on a taken channel but writes nothing to it and drops its notes there.

## Files

Read the way `load_file` reads them: `file_table` (`$cbfe`, 95 entries of start sector, sector
count, disk index 0..3 = disks 1..4), whole sectors, then `ATN!`. W1's music (`d1_c16`) only decodes
that way: its trailer is in the sector after the carver's last (docs/03).

| Tune (`TuneId`) | File | Entry | Heard with bank |
|---|---|---|---|
| `Title` | `MUSC` | 11 | none (loaded at the bank's place) |
| `World(1)`..`World(6)` | `L1MU`..`L6MU` | 22, 32, 53, 63, 73, 90 | W1–2 `L1FX` disk 1; W3–6 `L1FX` disk 2 |
| `Bonus` | `LBMU` | 84 | `LBFX` (83) |
| `Slot` | `LFMU` | 21 (copy on disk 2: 52) | `LFFX` (20; copy: 51) |

| Bank (`BankId`) | File | Entry |
|---|---|---|
| `Worlds12` | `L1FX` on disk 1 | 16 |
| `Worlds36` | `L1FX` on disk 2 (same name) | 47 |
| `Bonus` | `LBFX` | 83 |
| `Slot` | `LFFX` | 20 |

Where the final level gets its sound was not looked at **(inferred: W6's)**.

## Where things are in chip RAM

- A bank is loaded at `h3+$64c70` (`BANK_ADDR`, chip `$6c46c`). `sfx_bank_init` (`$12a26`) builds
  the sample and effect pointer tables and sets the module address (`$9ec`) to the end of the
  effects, so a world's module follows its bank.
- The title sets `$9ec` to `h3+$64c70` itself and plays `MUSC` from there, over the bank's place.
- The stream buffer is `h3+$64b70`, the 256 bytes before the bank.

Every pointer the driver writes to Paula is one of these absolute addresses, so the engine has to
place the files where the game does: `AudioDriver` keeps the files and their addresses, and the
module header the replayer relocates in place.

## The P41A module

Offsets from the module start + 4 (`h`, what the replayer keeps at `$145ae`):

| At | What |
|---|---|
| `-4` | `"P41A"` |
| `h+2` | byte: instrument count |
| `h+4`, `h+8`, `h+$c` | longs: offsets of the patterns, the position list, the sample data; init adds `h` |
| `h+$10 + 16(n-1)` | instrument `n` (1..31): sample offset (long, init adds the sample base), length (words), loop offset (long, relocated), loop length, volume, finetune (a byte offset into the period table, 74 per step) |

`music_start` copies the first 512 bytes of the module to `$4814c` before relocating; `music_stop`
copies them back to wherever `$9ec` points then, so a module can be started again.

**Positions**: four words each, the offset of each channel's pattern stream from the patterns base;
a negative first word restarts the list. A pattern is 64 rows (the row counter starts at `$3f`).

**Pattern streams** (per channel): a long per row, unless its first byte is negative, which makes it
a reference: a word whose low byte counts rows, then a signed offset (from the patterns base) of rows
to replay from there. A row long is: byte 0 bits 6..1 the note × 2 (0 none), word 0 bits 8..4 the
instrument × 16, bits 3..0 the effect, byte 2 the parameter, byte 3 a count of rows after it:
positive, that many empty rows (no effect runs on them); negative, the row repeats.

**Effects** (this replayer's numbering, not ProTracker's): 0 none, 1 slide up, 2 slide down,
3 tone portamento, 4 vibrato, 5 tone portamento + volume slide, 6 vibrato + volume slide, 7 tremolo,
**8 arpeggio** (step 1 adds the low nibble, step 2 the high one), 9 sample offset, A volume slide
(the parameter is a signed byte subtracted from the volume), B position jump (parameter × 4 bytes
into the position list), C volume, D next position (its row is ignored), E extended (0 LED filter,
1/2 fine slides, 5 finetune, 9 retrigger, A/B fine volume, C cut, D delay, E pattern delay), F speed.
Periods are clamped to `$71..$358`.

**Timing**: one tick per frame. A row's data is fetched on tick speed − 1 (or at the row itself at
speed 1) and played on tick 0. Speed is 6 at start.

## The sound effect bank

`LnFX`: 256 longs (the byte size of each effect), at `$400` the samples (each a long length in
words, then the data) up to a long `$ffffffff`, then the effects back to back. An effect is a word
(nonzero: a restart of a busy channel keeps its DMA running), a priority word, then 18-byte steps:

| At | What |
|---|---|
| `+0` | flags: bits 0–1 what follows the step (0 stop, 1 next step, 2 this step again, 3 the first); bits 2–3 and 4–5 the same for the period ramp and the volume ramp (0 hold, 4/$10 next step's ramp, 8/$20 this one again, $c/$30 the first); bit 7 the next step's sample starts without switching the channel off |
| `+1` | sample number (`$ff` none) |
| `+2` | duration in frames |
| `+4..+8` | period: from, to, steps |
| `+$a..+$e` | volume: from, to, steps |
| `+$10` | start offset into the sample, bytes |

A busy channel is taken only by an effect whose priority word is not higher than the one playing
(signed compare: `playing < new` refuses). The ramps interpolate with `mulu`/`divu`, the driver
reproducing the 68000's word results.

## Command entry points

| Original | Address | Driver | Called from |
|---|---|---|---|
| `music_start` | `$13126` | `play_music` / `music_start` | world intro, title, credits, slot machine, end sequence; nothing if music is on |
| `music_stop` | `$132a0` | `stop_music` | the game's flow; also from the fade's end |
| `music_fade_wait` | `$1327a` | `fade_out`, then the caller waits up to 100 frames for `music_on` to drop, then `stop_music` (it falls into `music_stop`, so a fade that ended stops twice) | between scenes |
| `$146ae` | | `start_fade(word)` | nothing in the listing refers to it |
| `sfx_bank_init` | `$12a26` | `sfx_bank_init` | world loads, slot machine |
| `sfx_play` | `$12a72` (`d0` effect, `d1` channel) | `play_sfx` | the frog, items, objects, slot machine |
| `sample_play` | `$148b8` (`a0` = `$1491e` + 16k) | `play_sample(k)` | frog idle, title, objects |

`sfx_play` and `sample_play` return at once in attract mode (`h0+$18`); the driver leaves that test
to its caller.

## Per frame, in the vertical-blank interrupt

Outside a level: `sample_stream` (`$147e4`), `music_tick` (`$13424`: the replayer, then the fade
and the shadow copy `$145b6`), a raster wait, the DMA stage (`$13cc4`), `sfx_tick` (`$12c60`).
`AudioDriver::tick` does exactly that. In a level the frog and scroll code runs between the stream
and the music, so the level's frame calls `stream_tick`, `music_tick`, `music_dma_tick`, `sfx_tick`
itself at those points.

- **Shadow copy**: for each channel music holds, pointer, length, period, volume from the shadow
  block (`$146e4`, 16 bytes a channel) to Paula, after the fade (`$146e0`: ceiling byte, frames per
  step byte, counter) has capped each volume. The fade's end calls `music_stop`, which leaves `a1`
  at `sfx_steal` (`$13024`): that frame's copy goes to RAM from there (16 bytes a channel, to
  `$1305e`, unused but for `sfx_steal`'s high byte), not to Paula.
- **DMA stage**: if a note started this tick, DMACON switches the channels on; when a pattern ends
  the next position is taken; each music channel's shadow is pointed at its instrument's loop, which
  reaches Paula at the next tick's copy.
- A channel whose instrument pointer is still 0 reads its "loop" from chip RAM `$6..$b`, which the
  driver takes from the transplanted memory (zero at power-on).

## Driver variables (for `sf-core::layout`, T12)

All in hunk 0; `sf_core::audio::vars` lists them and `AudioDriver::transplant` / `vars` copy them:

| Block | Address | Bytes |
|---|---|---|
| effect channel records 0–3, `sfx_steal`, the unused bytes the fade's end writes | `$12f62` | `$fe` (records at +0, +$26, +$5a, +$8e) |
| `music_on` and the speed flag `$13423` | `$13420` | 4 |
| replayer channels (4 × `$2c`), globals (`$1458c`), free bytes (`$145b2`) | `$144dc` | `$da` |
| fade and shadow | `$146e0` | `$44` |
| stream pointer, words left (`$ffff` idle), volume | `$148b0` | 8 |
| sample and effect pointer tables | `$1499e` | `$800` |
| module address | `$9ec` | 4 |
| module header backup | `$4814c` | `$200` |
| the relocated module header | at the module address | `$200` |

Not modelled: the registers the replayer saves at `$1339e` (only `a6`, `$dff000`, is used from them)
and the LED filter bit, a CIA write.
