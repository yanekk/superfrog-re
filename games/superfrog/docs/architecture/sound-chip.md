+++
id = "sound-chip"
title = "The sound chip"
tier = "mechanism"
depth = "deep"
kind = "amiga"
mix = { amiga = 100, game = 0, data = 0, shell = 0 }
reuse = "superfrog-only"
owns = []
+++

# The sound chip

## 1. What it is

The chip itself, `amiga-paula`, is the toolkit's: [sound-chip](../../../../toolkit/docs/architecture/sound-chip.md) has the channel model, `PaulaWrite`,
the resampler and what a driver must give it, with their unit tests. This chapter is how Superfrog uses it. It
owns no files: like [copied-in-data](copied-in-data.md) it is a view across code other chapters own and count.
`sf-core` implements `ChipMemory` on `Game` (`Game::chip_word`). `Paula::booted` starts with DMA's master switch
on, as the game's boot leaves it. The comparison with the reference emulator is
`games/superfrog/engine/crates/sf-conform/tests/paula_vs_oracle.rs`, outside the crate, because the purity test scans
the pure crates' tests too. The RE note on how the game drives Paula is [games/superfrog/docs/11](../11-sound.md); the
crate's place in the engine is [rust-engine DESIGN §3.1, §3.2](../../../../plans/rust-engine/DESIGN.md).

## 2. Kind mix

`amiga` 100 over no lines of its own: everything here is how the game feeds the imitated chip.

### What is measured

- `paula_vs_oracle.rs` feeds the oracle's writes and chip RAM through the title music and finds the taps equal to
  vAmiga's at every clock (`channel_taps_are_the_references_at_every_clock_through_the_title_music`).
- The mix is held to vAmiga's own 48 kHz mix within tolerance (`the_mix_is_within_tolerance_of_the_references_own`;
  the float exception is rust-engine DESIGN §3.1, owner 2026-10-10).
- In the engine the beam positions come from the cost model, not the original (§4), so its channel output is close
  to the original's, not equal. `sf-conform channels` judges how close.
- Superfrog's driver runs from the vertical blank and does not need the audio interrupts, which do not reach a CPU.
  The missing analog filters are recorded in rust-engine FINDINGS 2026-10-11.

## 3. Where else the mechanism is used

- **The app** (`games/superfrog/engine/crates/sf-app/src/main.rs`): one `Paula::booted` per run, fed each frame's
  `FrameOut::paula` writes (`mixed()`) and the load's blanks (`Game::take_load_paula`), with the
  `Game` as `ChipMemory`. The samples go to the audio device (`games/superfrog/engine/crates/sf-app/src/audio.rs`,
  chapter [app](app.md)).
- **The front end and the story intro** run `AudioDma` on their frames' writes to time bus
  cycles (`games/superfrog/engine/crates/sf-core/src/front/mod.rs`, `games/superfrog/engine/crates/sf-core/src/front/story/sim.rs`,
  chapters [front-end](front-end.md), [story-and-ending](story-and-ending.md)).
- **The game's sound driver** makes the writes (`games/superfrog/engine/crates/sf-core/src/audio/`, chapter
  [sound-driver](sound-driver.md)); the level, front end and story place them on the beam (§4).
- **Verification**: `games/superfrog/engine/crates/sf-conform/src/channels.rs` replays a pacing feed headless
  through the same mixer and compares `Paula::channel_taps` with vAmiga's taps
  (`sf_oracle::FrameCapture::taps`, from the core patch
  `games/superfrog/engine/crates/sf-oracle/patches/0004-audio-sample-hook.patch`).
  `games/superfrog/engine/crates/sf-conform/src/sound_events.rs` reduces `PaulaWrite`s to sound events for
  check 3. Chapter [verification](verification.md).

## 4. The interface: `PaulaWrite` and the beam position (T213)

The game's driver emits its writes in the original's order with `vpos` and `hpos` left 0
(`wr` in `games/superfrog/engine/crates/sf-core/src/audio/mod.rs`). The caller then stamps each with where the
original's handler would have made it:

- **In a level**, the beam clock of [cpu-time-and-beam](cpu-time-and-beam.md):
  `LevelCtx::audio_spent` and `LevelCtx::stamp` in `games/superfrog/engine/crates/sf-core/src/level/mod.rs`, each
  write costing `Cost::AudioWrite` or `Cost::AudioStop`.
- **Outside a level**, `front::vbl::place` (`games/superfrog/engine/crates/sf-core/src/front/vbl.rs`): a cost model
  of the handler, `music_tick`'s writes spread over it, the DMA stage's eight lines later after
  the wait at `h0+$12a14`.
- **In the story intro**, at its wait's end (`games/superfrog/engine/crates/sf-core/src/front/story/intro.rs`).
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

What the toolkit chapter asks of a driver holds here: check 3 compares the events a frame's writes reduce to, and a
load's blanks (`Game::take_load_paula`) go through the mixer so its channels leave the load as the original's did.

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
| Channel state machine and DAC | Each channel's DAC value equals vAmiga's at every Paula clock, given the same writes and chip memory | `paula_vs_oracle.rs`'s `channel_taps_are_the_references_at_every_clock_through_the_title_music` |
| Retrigger restarts only after idle | A channel turned off and on restarts its note only if it went idle between | `games/superfrog/engine/crates/sf-conform/tests/channels.rs`: `the_titles_retriggered_notes_restart_as_on_the_original`, `the_slot_machines_effects_restart_as_on_the_original`; `the_whole_games_channels_sound_as_the_original_does` (ignored, over `games/superfrog/engine/conform/cases/game-whole.toml`) |
| Short frames in the app | The game keeps whole stretches of short frames (LACE cleared with LOF short) | No check. `paula_vs_oracle.rs` sets each frame's length from the oracle, but the app and `channels::engine_levels` never call `Paula::set_frame_lines`, so they mix every frame as long. Whether that is audible is unknown: run `sf-conform channels` with the frame lengths fed |
| DMA slots for bus timing | `AudioDma` takes the slots Agnus would | `games/superfrog/engine/crates/sf-conform/tests/story_draws.rs` (the intro's draws against the original's register writes, BLTSIZE among them, with `AudioDma` run on the original's Paula writes holding the blits up) |
| Attach modes | ADKCON attached volume and period | No check covers them. Superfrog's driver writes no ADKCON that this chapter found |
| Purity | Same writes and memory give the same output | `games/superfrog/engine/crates/sf-conform/tests/purity.rs` |
| Resampler (shell) | Mix within tolerance of vAmiga's | `paula_vs_oracle.rs`'s `the_mix_is_within_tolerance_of_the_references_own` |

## 7. Native sketch (non-binding)

One open improvement: pass each frame's length to the mixer (§6, short frames). The toolkit chapter has the rest.

## 8. Reuse

**`superfrog-only`** for what is left here: how this game's driver and handlers feed the chip. The chip is `as-is`.
`booted`'s DMACON `$8200` describes Superfrog's boot; another game may boot with other bits.

## 9. Sources

- `games/superfrog/engine/crates/sf-conform/tests/paula_vs_oracle.rs`, `games/superfrog/engine/crates/sf-conform/tests/channels.rs`,
  `games/superfrog/engine/crates/sf-conform/src/channels.rs`, `games/superfrog/engine/crates/sf-conform/src/sound_events.rs`,
  `games/superfrog/engine/conform/README.md` ("Each channel's output (T213)").
- `games/superfrog/engine/crates/sf-core/src/audio/mod.rs`, `games/superfrog/engine/crates/sf-core/src/level/mod.rs`,
  `games/superfrog/engine/crates/sf-core/src/front/vbl.rs`, `games/superfrog/engine/crates/sf-core/src/game.rs`,
  `games/superfrog/engine/crates/sf-app/src/main.rs`.
- [games/superfrog/docs/11](../11-sound.md); [rust-engine DESIGN](../../../../plans/rust-engine/DESIGN.md) §2.1, §3.1,
  §3.2; `plans/rust-engine/tasks/T213-paula-write-timing.md`; `plans/rust-engine/FINDINGS.md`.
