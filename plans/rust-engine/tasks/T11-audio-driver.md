# T11 — audio-driver

**Phase:** 1 · **Depends on:** T06, T07 · **Weight:** medium

## Goal

Port the game's own sound driver: its music replayer (the module format, whatever it turns out to be)
and its sound-effect player with channel allocation, so that for any sequence of "play tune X / play
effect Y / stop" commands the Rust driver makes exactly the Paula writes the original makes, frame for
frame. Mechanic tasks then call it the way the original's code calls its driver.

## Design sections this implements

DESIGN §2.1 (Paula writes), §2.3 (music and samples read from disks).

## Files

- `engine/crates/sf-assets/src/sound.rs` — music modules and sample banks decoded from disks.
- `engine/crates/sf-core/src/audio/{mod.rs,music.rs,sfx.rs}`
- `engine/crates/sf-core/tests/audio_driver.rs`
- `docs/11-sound.md`

## Interface

```rust
pub struct AudioDriver;  // state mirrors the original's driver variables (mapped in layout)
impl AudioDriver {
    pub fn play_music(&mut self, tune: TuneId); pub fn stop_music(&mut self);
    pub fn play_sfx(&mut self, sfx: SfxId, priority: u8);
    pub fn tick(&mut self, data: &SoundData, out: &mut Vec<PaulaWrite>);   // what the original's interrupt does each frame
}
```

## Tests

- [ ] routine conformance (T06): the replayer's per-tick routine on Moira and `tick` produce identical
      writes and driver state for every tune, from start for 3 000 ticks, and from 50 random transplanted states.
- [ ] sfx: each effect alone, two overlapping with different priorities, an effect during music: writes identical to the original routine.
- [ ] full-system: title music for 2 000 frames, Paula writes identical to the oracle.
- [ ] every tune and effect id from T07's list decodes.

## Done when

- [ ] Tests in the test block (the long runs in `sf-conform -- all`).
- [ ] docs/11 documents the music format and the driver's command entry points.
- [ ] Driver variables are in `sf-core::layout` (or, if T12 has not landed, listed in docs/11 for T12 to map).
