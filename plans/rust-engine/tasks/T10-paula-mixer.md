# T10 — paula-mixer

**Phase:** 1 · **Depends on:** T01, T05 · **Weight:** light

## Goal

Turn the Paula register writes a frame produces into audio, the way the chip does: four DMA channels
fetching 8-bit samples at their periods, volumes, the hardware left/right channel pairing, then a
band-limited resample to the host rate. Pure; the shell only hands the samples to the device.

## Design sections this implements

DESIGN §2.1 (Paula writes are the definition; this turns them into sound), §2.5, §3.2 (`sf-paula`).

## Files

- `engine/crates/sf-paula/src/{lib.rs,channel.rs,resample.rs}`
- `engine/crates/sf-paula/tests/{unit.rs,vs_oracle.rs}`

## Interface

```rust
pub const PAULA_CLOCK_PAL: u32 = 3_546_895;
pub struct Paula;  // 4 channels, reads sample bytes from a chip-memory view
impl Paula {
    pub fn new(host_rate: u32) -> Paula;
    pub fn frame(&mut self, writes: &[PaulaWrite], chip: &dyn ChipMemory, out: &mut Vec<[i16; 2]>);  // one PAL frame
    pub fn channel_taps(&self) -> [&[i8]; 4];   // per-channel DAC values at Paula rate for the last frame, for tests
}
```

`resample.rs` is the one pure file allowed floating point and filter tables (DESIGN §3.1); channel
modelling in `channel.rs` stays integer.

`ChipMemory` is the engine's view of where samples live (`sf-core` provides it from `GameData`), so the
mixer never owns game data.

## Tests

- [ ] a single channel at period 428 plays a known looping waveform at the right frequency.
- [ ] LEN/LC reload at loop end; period below the DMA minimum clamps as the chip does; volume 64 = full, 0 = silent.
- [ ] channels 0/3 left, 1/2 right.
- [ ] against the oracle on the title music's first 500 frames: per-channel taps identical; the mixed
      output within the tolerance recorded in the test (vAmiga resamples differently, so exact is not possible there).
- [ ] the output is a pure function of the writes and memory: same input, same samples.

## Done when

- [ ] Tests in the test block.
- [ ] Channel taps identical to the oracle's for the title music sample.
- [ ] The tolerance and why it is that number are in the test's doc comment.
